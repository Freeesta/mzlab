// @ts-strict TPMINE-PRIVATE  Pipeline of mzFinder in the browser: the Web Workers that run the feature engine (crates/mzlab-feat-wasm, two .wasm of their own: `feat` = detection and
// alignment, `proofs` = hypotheses and graph of the proofs) and the supervisor that drives them. One file, two roles:   - imported by the page as a module (from a Blob URL, like the other files of mzFinder): `createFeatPool(...)` is the supervisor;
//   - started as a module Worker (the same URL): the worker below. It reads the .mzidx files with OPFS synchronous handles (`createSyncAccessHandle`: the only reads the
//     .wasm can do while it runs, no SharedArrayBuffer) and hands the page byte arrays that are transferred, never views on the memory of the .wasm.
// One worker = one file at a time (detection) or one series (alignment, hypotheses, proofs: sequential, in the same worker); at most min(cores - 1, 4) workers (2 on iPad);
// the .wasm of each works within the budget of its step (`budgetMb`). A worker that dies (a trap of the .wasm, out of memory, a crash) is terminated and replaced, the job runs again once
// and the page is told (`notify`); a second death rejects the job. The OPFS files written here have the name of a finished file only when complete (written as `.part`).
// Messages of the worker: { op, id, ... } -> { id, ok: true, ...result } | { id, error: { key, params, text }, fatal }. Errors of the .wasm are JSON { error_key, params } (a recipe the engine refuses is "error.badRecipe").
const IN_WORKER = typeof WorkerGlobalScope !== "undefined" && self instanceof WorkerGlobalScope;
// ---------------------------------------------------------------- page side
export const poolSize = (small, cores) => { const n = Math.max(1, Math.min((cores || 2) - 1, 4)); return small ? Math.min(n, 2) : n; };
const IDLE_MS = 20000;      // the memory of a .wasm never shrinks: a pool idle for this long closes its workers (the next job starts one again, a few tens of ms)
// Working memory of a step in MB: `budgetMb` of the pool (a number or a function), else the safe budget of the engine of the page (`MZLAB_RUST`: its choice, or the level the supervisor lowered it to), else 0 (= 192)
export const budgetOf = o => (typeof o.budgetMb === "function" ? o.budgetMb() : o.budgetMb) || (globalThis.MZLAB_RUST && (globalThis.MZLAB_RUST.budgetMb || (globalThis.MZLAB_RUST.choice && globalThis.MZLAB_RUST.choice.mb))) || 0;
const fail = (key, params, text) => Object.assign(new Error(text || key), { key, params: params || {} });
// nothing queued, nothing running, no call to the archive worker for `idleMs`: the workers go (the state of a job lives in the page, never in an idle worker)
function idleRest(idleMs, st, slots, queue, kill) {
  let idle = null;
  const rest = () => {
    clearTimeout(idle);
    if (!idleMs) return;
    idle = setTimeout(() => {
      if (queue.length || [...slots].some(s => s.busy) || (st.archive && st.archive.calls.size)) return rest();
      for (const s of [...slots]) kill(s);
      if (st.archive) { try { st.archive.w.terminate(); } catch (_) { /* gone */ } st.archive = null; }
    }, idleMs);
  };
  return rest;
}

// opts: { workerUrl, wasm: { feat: { jsUrl, wasmUrl, sha256? }, proofs: { ... }, table: { ... } }, size, budgetMb (working memory of a step of the .wasm, MB, a number or a function; 0 = the safe budget of the engine of the page, else 192; `setBudget(mb)` changes it and returns the old one), idleMs (a pool with nothing to do for this long closes its workers, 0 = never; IDLE_MS), notify(key, params), onProgress(info), WorkerClass }
// The recipe is a JSON string: {"polarity": 1 | -1 | null, "mode": "ion" | "family"}.
// the supervisor: the workers alive, the queue of jobs, the replacement of a worker that dies
function createCore(opts) {
  const { workerUrl, wasm, notify = () => {}, WorkerClass = Worker } = opts;
  const limit = Math.max(1, opts.size || 1);
  const slots = new Set();                      // every worker alive: { w, busy, loaded:Set, dead }
  const st = /** @type {{ archive: any, cancelled: boolean }} */ ({ archive: null, cancelled: false });
  const queue = [];                             // jobs waiting for a worker: { run(slot), resolve, reject, tried }
  const rest = idleRest(opts.idleMs ?? IDLE_MS, st, slots, queue, s => kill(s));
  function spawn() {
    const slot = { w: new WorkerClass(workerUrl, { type: "module" }), busy: false, loaded: new Set(), dead: false, calls: new Map(), seq: 0 };
    slot.w.onmessage = ev => { const d = ev.data, c = slot.calls.get(d.id); if (!c) return; if (d.progress) { if (c.step) c.step(d.progress[0], d.progress[1]); return; } slot.calls.delete(d.id); c(d); };
    const lost = text => { if (slot.dead) return; slot.dead = true; const cs = [...slot.calls.values()]; slot.calls.clear(); for (const c of cs) c({ error: { key: "error.featWorker", params: {}, text }, fatal: true }); };
    slot.w.onerror = e => lost(e && e.message || "worker error");
    slot.w.onmessageerror = () => lost("message error");
    slot.lost = lost;
    return slot;
  }
  // send a message and wait for the answer; a worker that dies answers with a fatal error
  function call(slot, msg, transfer, step) {
    return new Promise((resolve, reject) => {
      const id = ++slot.seq;
      slot.calls.set(id, Object.assign(r => (r.error ? reject(Object.assign(fail(r.error.key, r.error.params, r.error.text), { fatal: !!r.fatal })) : resolve(r)), { step }));
      if (slot.dead) { slot.calls.delete(id); return reject(Object.assign(fail("error.featWorker"), { fatal: true })); }
      try { slot.w.postMessage({ ...msg, id }, transfer || []); } catch (e) { slot.calls.delete(id); reject(Object.assign(fail("error.featWorker", {}, String(e)), { fatal: true })); }
    });
  }
  // the table of chemical classes, a file beside the `.wasm` `attributio_rev` (admin only), fetched once when the first review opens; "" without it
  let classes = null;
  async function classesText() {
    const u = wasm.attributio_rev && wasm.attributio_rev.classesUrl;
    if (!u) return "";
    if (classes === null) classes = await (await fetch(u)).text();
    return classes;
  }
  async function ensure(slot, which) {          // load the .wasm of `which` in the worker once; keys that name the same .wasm (fused module) share the instance
    if (slot.loaded.has(which)) return;
    const m = wasm[which], twin = [...slot.loaded].find(k => wasm[k].wasmUrl === m.wasmUrl);
    const r = await call(slot, twin ? { op: "alias", which, of: twin } : { op: "init", which, jsUrl: m.jsUrl, wasmUrl: m.wasmUrl, sha256: m.sha256 || "" });
    slot.crate = r.crate || slot.crate;
    slot.loaded.add(which);
  }
  function kill(slot) { slot.dead = true; slots.delete(slot); try { slot.w.terminate(); } catch (_) { /* gone */ } }
  function pump() {
    while (queue.length) {
      let slot = [...slots].find(s => !s.busy && !s.dead);
      if (!slot && slots.size < limit) { slot = spawn(); slots.add(slot); }
      if (!slot) return;
      start(slot, queue.shift());
    }
  }
  async function start(slot, job) {
    slot.busy = true; slot.job = job.id; slot.current = job;
    try {
      const value = await job.run(slot, which => ensure(slot, which));
      slot.busy = false; slot.job = slot.current = undefined;
      if (slot.recycle) kill(slot);               // a worker of one heavy phase: its memory goes back by closing it
      job.resolve(value);
    } catch (e) {
      slot.busy = false; slot.job = slot.current = undefined;
      if (slot.recycle) kill(slot);
      if (job.cancelled) job.reject(fail("tpmine.feat.cancelled")); else if (e && e.fatal) {   // the worker is lost: a new one, and the job once more
        kill(slot);
        if (!job.tried && !st.cancelled) { job.tried = true; notify("tpmine.feat.restarted", { reason: e.key || "" }); queue.unshift(job); }
        else job.reject(e);
      } else job.reject(e);
    }
    pump();
    rest();
  }
  function submit(run, id) {
    if (st.cancelled) return Promise.reject(fail("tpmine.feat.cancelled"));
    return new Promise((resolve, reject) => { queue.push({ run, resolve, reject, tried: false, id }); pump(); });
  }
  const cancelJob = makeCancel({ queue, slots, kill, fail });
  const copy = list => list.map(b => b.slice());      // inputs of a job that may run again are copied, never transferred
  return { st, slots, queue, spawn, call, submit, cancelJob, kill, copy, rest, classesText };
}
// the folder of a campaign as the worker reads it
const folder = (root, files) => JSON.stringify({ root, files: files.map(f => ({ path: f.path, size: f.file.size })) });
// cancelJob(id) of a core: a queued job is rejected, a running one loses its worker alone and is not tried again; false when no such job
const makeCancel = ({ queue, slots, kill, fail }) => id => {
  const at = queue.findIndex(j => j.id === id), slot = [...slots].find(s => s.busy && s.job === id);
  if (at >= 0) queue.splice(at, 1)[0].reject(fail("tpmine.feat.cancelled"));
  else if (slot) { slot.current.cancelled = true; slot.lost("cancelled"); kill(slot); }
  return at >= 0 || !!slot;
};
// progress of a detection for `onStep` and `onJob` (`f.id` names the job for `cancelJob`)
const detectStep = (opts, f) => (opts.onStep || opts.onJob) && ((d, t) => { opts.onStep?.({ phase: "detect", name: f.name, id: f.id, done: d, total: t }); if (f.id !== undefined) opts.onJob?.({ job: f.id, frac: t ? Math.min(1, d / t) : 1 }); });
// Runs a .wasm export over the job's files and writes into memory: { bytes }.
function viaFiles(m, run) {
  const out = memSink(), rd = new FileReaderSync();
  run((k, off, len) => new Uint8Array(rd.readAsArrayBuffer(m.files[k].slice(off, off + len))), out.write);
  return { bytes: out.bytes() };
}
// The exports of a campaign folder (admin mode): `kind` = schymanski | publication | quantify | gradient (`.wasm` `attributio_export`) or evidence (`.wasm` `attributio_rev`, it needs the review);
// `extra` = { matrix?, generated? }. Resolves with { files: [{ name, text } | { name, rows: { hojas } }], warnings }.
function exportOps(submit, call) {
  return {
    attributioExport: (files, root, kind, extra = {}) => submit(async (slot, load) => {
      const which = kind === "evidence" ? "attributio_rev" : "attributio_export";
      await load(which);
      const names = JSON.stringify({ ...extra, root, files: files.map(f => ({ path: f.path, size: f.file.size })) });
      return JSON.parse(new TextDecoder().decode((await call(slot, { op: "attributio_export", which, files: files.map(f => f.file), names, kind })).bytes));
    }),
    attributioMs2: (files, root, request) => submit(async (slot, load) => {      // `.wasm` attributio_ms2; request.op = engine | insilico | rearrange | rt | chimeric
      await load("attributio_ms2");
      const names = JSON.stringify({ root, files: files.map(f => ({ path: f.path, size: f.file.size })) });
      return JSON.parse(new TextDecoder().decode((await call(slot, { op: "attributio_ms2", files: files.map(f => f.file), names, request: JSON.stringify(request) })).bytes));
    }),
  };
}
// crateVersion(): the version of the crate mzlab-feat inside the .wasm (compared with cancello.json); preview(file, recipe): the estimates with their proofs, the histograms and three XIC
// of one file (JSON of mzlab_feat::params::preview_json); `recipe` may carry the 6 numbers of the panel; family(file, recipe): the `famiglia/1` record with the proofs of the arcs (family_record).
function previewOps(submit, call, opts) {
  return {
    crateVersion() { return submit(async (slot, load) => { await load("feat"); return slot.crate; }); },
    preview(file, recipe) {
      return submit(async (slot, load) => {
        await load("feat");
        const t0 = performance.now();
        const r = await call(slot, { op: "preview", path: file.path, recipe });
        if (opts.onProgress) opts.onProgress({ phase: "preview", name: file.name, done: 1, total: 1, ms: Math.round(performance.now() - t0), memMB: r.mem / 1048576 });
        return { ...JSON.parse(r.json), ms: Math.round(performance.now() - t0), crate: slot.crate };
      });
    },
    family(file, recipe) { return submit(async (slot, load) => { await load("feat"); return JSON.parse((await call(slot, { op: "family", path: file.path, name: file.name, recipe })).json); }); },
  };
}
/** drift of the RT of the parent along a series (JSON of mzlab_feat::params_drift): `stored` = the .mzfeatf of its files, `baseRtTol` = the tolerance from the peak width (NaN = none) */
function driftOps(submit, call, copy) {
  return { drift: ({ stored, parentMz, baseRtTol }) => submit(async (slot, load) => { await load("feat"); return JSON.parse((await call(slot, { op: "drift", files: copy(stored), parentMz, baseRtTol })).json); }) };
}
function proofOps(submit, call, copy, opts) {
  return {
    // L7 (hypotheses of the parent searched in every file) and L8-L9 (graph of the proofs on the table) in two workers of their own: the hypotheses worker is closed
    // when it has answered (its memory goes back) and the state it leaves (small) is loaded by the worker of the proofs. Each can be refused with `error.featMemory`.
    // `structure`: grafo/1 JSON of the SMILES of the parent (optional). `hypotheses(...)` resolves with { state, tried, found, registry, ms }; `evidence(...)` with { table }.
    hypotheses({ stored, paths, rules, parent, parentRt, recipe, structure = "" }) {
      return submit(async (slot, load) => {
        await load("proofs");
        const t0 = performance.now();
        const h = await call(slot, { op: "hypotheses", files: copy(stored), paths, rules: rules.slice(), parent, parentRt, recipe, structure });
        slot.recycle = true;                    // the memory of the .wasm is not given back: the worker is closed
        if (opts.onProgress) opts.onProgress({ phase: "hypotheses", done: 1, total: 1, ms: Math.round(performance.now() - t0), memMB: h.mem / 1048576 });
        return { state: h.state, tried: h.tried, found: h.found, registry: h.registry, ms: Math.round(performance.now() - t0) };
      });
    },
    evidence({ table, state, minutes, parentMz, parentRt, recipe }) {
      return submit(async (slot, load) => {
        await load("proofs");
        const t0 = performance.now();
        const e = await call(slot, { op: "evidence", table: table.slice(), state: state.slice(), minutes, parentMz, parentRt, recipe, budgetMb: budgetOf(opts) });
        slot.recycle = true;
        if (opts.onProgress) opts.onProgress({ phase: "proofs", done: 1, total: 1, ms: Math.round(performance.now() - t0), memMB: e.mem / 1048576 });
        return { table: e.bytes };
      });
    },
  };
}

// Methods of the pool that read one row or one spectrum of the table (MS2 file, traces, compositions).
function rowOps(submit, call) {
  return {
    // MS2 file of the table (FBMN `.mgf`, SIRIUS `.ms`): the best MS2 of each row is read from the .mzidx of its file (OPFS). Resolves with the text. `tableFile`: CSV or mzTab-M in the polarity of the table and its files (as `flow run`).
    ms2File: ({ table, samples, spec, sha, pol, kind, paths }) => submit(async (slot, load) => { await load("table"); return (await call(slot, { op: "ms2file", table: table.slice(), samples, spec, sha, pol, kind, paths })).text; }), tableFile: ({ table, samples, sha, kind, paths }) => submit(async (slot, load) => { await load("table"); return (await call(slot, { op: "table_file", table: table.slice(), samples, sha, kind, paths })).text; }), tableView: ({ table, samples, params }) => submit(async (slot, load) => { await load("table"); return (await call(slot, { op: "table_view", table: table.slice(), samples, params })).text; }),
    // XIC of every file, MS1 at the best apex and MS2 of one row (`riga_tracce/1`), read from the .mzidx in OPFS. Resolves with the text.
    rowTraces: ({ table, samples, id, ppm, paths }) => submit(async (slot, load) => { await load("table"); return (await call(slot, { op: "rowtraces", table: table.slice(), samples, id, ppm, paths })).text; }),
    // Candidate elemental compositions of one feature from its MS1 spectrum (`.wasm` `untargeted`, `formula_id/1`): `mz`, `y` the spectrum, `featureMz` the monoisotopic m/z,
    // `params` the JSON text of the options (ion, z, tol_ppm, elements, nitrogen, rdb_min, max_results). Resolves with the JSON text.
    formulaId({ mz, y, featureMz, params }) {
      return submit(async (slot, load) => {
        await load("untargeted");
        return (await call(slot, { op: "formula_id", mz: Float64Array.from(mz), y: Float64Array.from(y), featureMz, params: params || "" })).text;
      });
    },
    crossPolarity: ({ pos, neg, params }) => submit(async (slot, load) => { await load("table"); return (await call(slot, { op: "cross_polarity", pos: pos.slice(), neg: neg.slice(), params: params || "" })).text; }),
    interpBuild: m => submit(async (slot, load) => { await load("interp"); return (await call(slot, { op: "interp_build", ...m, table: m.table.slice() })).bytes; }),
    interpRow: m => submit(async (slot, load) => { await load("interp"); return (await call(slot, { op: "table_interp", ...m, table: m.table.slice() })).text; }),
    linkEvidence: m => submit(async (slot, load) => { await load("proofs"); const r = await call(slot, { op: "tp_link_evidence", ...m, table: m.table.slice(), rules: m.rules.slice(), peaks: m.peaks.map(b => b.slice()) }); slot.recycle = true; return r; }),
    tpPost: m => submit(async (slot, load) => { await load(m.kind === "network" ? "table" : "interp"); return (await call(slot, { op: "tp_post", ...m, tp: m.tp.slice(), table: m.table && m.table.slice(), rules: m.rules && m.rules.slice() })).text; }),
    linkInterp: m => submit(async (slot, load) => { await load("interp"); return (await call(slot, { op: "tp_link_interp", ...m, table: m.table.slice() })).bytes; }),
    digLr: m => submit(async (slot, load) => { await load("proofs"); await load("finder_lr"); return (await call(slot, { op: "dig_lr", ...m, table: m.table.slice(), rules: m.rules.slice(), peaks: m.peaks.map(b => b.slice()) })).text; }),
  };
}

export function createFeatPool(opts) {
  const { st, slots, queue, spawn, call, submit, cancelJob, kill, copy, rest, classesText } = createCore(opts);
  return {
    notify: (k, p) => (opts.notify || (() => {}))(k, p),
    // L1-L3 of every file, in parallel; `files`: [{ name, path }] (path: OPFS path of the .mzidx). Resolves with the .mzfeatf bytes in the order of `files`.
    async detectAll(files, recipe) {
      let done = 0;
      return Promise.all(files.map(f => submit(async (slot, load) => {
        await load("feat");
        const t0 = performance.now();
        const r = await call(slot, { op: "detect", path: f.path, name: f.name, recipe }, undefined, detectStep(opts, f));
        if (opts.onProgress) opts.onProgress({ phase: "detect", name: f.name, done: ++done, total: files.length, ms: Math.round(performance.now() - t0), memMB: r.mem / 1048576 });
        return r.bytes;
      }, f.id)));
    },
    // The pipeline after mzmine 3.9.0 (admin mode, `.wasm` `untargeted`): the same two steps as `detectAll` and `align`; `params` = JSON string of the recipe {"engine": "untargeted", ...}.
    async detectAllUnt(files, params) {
      let done = 0;
      return Promise.all(files.map(f => submit(async (slot, load) => {
        await load("untargeted");
        const t0 = performance.now();
        const r = await call(slot, { op: "detect_unt", path: f.path, name: f.name, params });
        if (opts.onProgress) opts.onProgress({ phase: "detect", name: f.name, done: ++done, total: files.length, ms: Math.round(performance.now() - t0), memMB: r.mem / 1048576 });
        return r.bytes;
      })));
    },
    alignUnt(stored, paths, params) {
      return submit(async (slot, load) => {
        await load("untargeted");
        const t0 = performance.now();
        const r = await call(slot, { op: "align_unt", files: copy(stored), paths, params, budgetMb: budgetOf(opts) });
        if (opts.onProgress) opts.onProgress({ phase: "align", done: 1, total: 1, ms: Math.round(performance.now() - t0), memMB: r.mem / 1048576 });
        return r.bytes;
      });
    },
    // The table of levels of a campaign folder (admin mode, `.wasm` `attributio`): `files` = [{ path, file }] (path relative to the folder, File of the page), `root` = folder name,
    // `kind` = "json" | "csv" | "xlsx". Resolves with { summary (object), bytes (the rendering) }.
    attributioAnnotate(files, root, kind = "json") {
      return submit(async (slot, load) => {
        await load("attributio");
        const r = await call(slot, { op: "attributio_annotate", files: files.map(f => f.file), names: folder(root, files), kind });
        return { summary: JSON.parse(r.summary), bytes: r.bytes };
      });
    },
    // The Banco 2b (admin mode, `.wasm` `attributio_rev`, loaded when the Banco opens): the review of ONE feature of the folder. Resolves with the object of the review (proposal, proofs, checklist, alternatives).
    attributioReview(files, root, pol, id) {
      return submit(async (slot, load) => {
        await load("attributio_rev");
        const r = await call(slot, { op: "attributio_review", files: files.map(f => f.file), names: folder(root, files), pol, feature: id, classes: await classesText() });
        return JSON.parse(new TextDecoder().decode(r.bytes));
      });
    },
    // The decision of the chemist (`decision` = JSON text of the Banco) added to the decisions of the folder. Resolves with { summary: { path, added, replaced, unchanged }, text } (the whole decisions file).
    attributioDecision(files, root, decision) {
      return submit(async (slot, load) => {
        await load("attributio_rev");
        const r = await call(slot, { op: "attributio_decision", files: files.map(f => f.file), names: folder(root, files), decision });
        return { summary: JSON.parse(r.summary), text: new TextDecoder().decode(r.bytes) };
      });
    },
    ...exportOps(submit, call),
    ...previewOps(submit, call, opts),
    ...driftOps(submit, call, copy),
    ...proofOps(submit, call, copy, opts),
    ...rowOps(submit, call),
    // L4-L6: alignment and gap filling of the series in one worker. `stored`: .mzfeatf bytes in the order of `paths`. Resolves with the .mzfeat bytes.
    align(stored, paths, recipe, id, recycle) {
      return submit(async (slot, load) => {
        await load("feat");
        if (recycle) slot.recycle = true;
        const t0 = performance.now();
        const r = await call(slot, { op: "align", files: copy(stored), paths, recipe, budgetMb: budgetOf(opts) });
        if (opts.onProgress) opts.onProgress({ phase: "align", done: 1, total: 1, ms: Math.round(performance.now() - t0), memMB: r.mem / 1048576 });
        return r.bytes;
      }, id);
    },
    // both phases in a row (hypotheses worker, then proofs worker). Resolves with { table, tried, found, registry, msHypotheses }.
    async proofs({ table, stored, paths, rules, parent, parentRt, minutes, parentMz, recipe }) {
      const h = await this.hypotheses({ stored, paths, rules, parent, parentRt, recipe });
      const e = await this.evidence({ table, state: h.state, minutes, parentMz, parentRt, recipe });
      return { table: e.table, tried: h.tried, found: h.found, registry: h.registry, msHypotheses: h.ms };
    },
    op: (which, msg) => submit(async (slot, load) => { await load(which); return call(slot, { ...msg, which }); }),   // one call of the .wasm `which` in a worker of the pool (the cache of the detections)
    // the archive worker (the only one that writes OPFS): path = "mzlab/feat/<name>"
    fs: {
      async _call(msg, transfer) {
        if (!st.archive || st.archive.dead) st.archive = spawn();
        try { return await call(st.archive, msg, transfer); } finally { rest(); }
      },
      write(path, bytes) { return this._call({ op: "fs.write", path, bytes }, [bytes.buffer]); },
      async read(path) { return (await this._call({ op: "fs.read", path })).bytes; },
      async exists(path) { return (await this._call({ op: "fs.stat", path })).size >= 0; },
      remove(path) { return this._call({ op: "fs.remove", path }); },
      trim(dir, cap) { return this._call({ op: "fs.trim", dir, cap }); },   // keeps the newest files of the folder within `cap` bytes (0 empties it): { freed }
    },
    cancel() {                                  // everything stops: running jobs are lost with their workers, the queued ones are rejected
      st.cancelled = true;
      for (const j of queue.splice(0)) j.reject(fail("tpmine.feat.cancelled"));
      for (const s of [...slots]) { s.lost("cancelled"); kill(s); }
      if (st.archive) { try { st.archive.w.terminate(); } catch (_) { /* gone */ } st.archive = null; }
    },
    resume() { st.cancelled = false; },
    cancelJob, get onJob() { return opts.onJob; }, set onJob(f) { opts.onJob = f; },
    setBudget(mb) { const was = opts.budgetMb; opts.budgetMb = mb; return was; },
    get onProgress() { return opts.onProgress; },
    set onProgress(f) { opts.onProgress = f; },
    get workers() { return slots.size; },
    // for the tests: the worker that is working dies as a crashed tab would
    _killBusy() { let n = 0; for (const s of slots) if (s.busy) { n++; s.lost("killed"); try { s.w.terminate(); } catch (_) { /* gone */ } } return n; },
    _die() { return submit(async (slot, load) => { await load("feat"); await call(slot, { op: "die" }); }); },
  };
}

// ---------------------------------------------------------------- worker side
const WASM_KEYS = ["error.featState"];                  // keys that only the .wasm returns (the page translates them)
const mods = {}, mems = {};                              // { feat, proofs, untargeted, ... }: the wasm-bindgen modules
let hypo = null, scratch = new Uint8Array(1 << 16);   // Hypotheses of the last run_hypotheses (kept in the memory of this worker for evidence); buffer reused by the reads of the .mzidx

async function dirOf(parts, create) {
  let d = await navigator.storage.getDirectory(); for (const p of parts) d = await d.getDirectoryHandle(p, { create }); return d;
}
async function openSync(path) {
  const parts = path.split("/").filter(Boolean), name = parts.pop(), fh = await (await dirOf(parts, false)).getFileHandle(name);
  for (let i = 0; ; i++) {   // a handle of a worker that has just died is released a moment later
    try { const h = await fh.createSyncAccessHandle(); return { h, size: h.getSize() }; }
    catch (e) { if (i >= 40 || e.name !== "NoModificationAllowedError") throw e; await new Promise(r => setTimeout(r, 50)); }
  }
}
// the readers of a list of files: read(k, offset, length) -> Uint8Array (a view of a buffer that is reused: the .wasm copies it at once)
async function openAll(paths) {
  const hs = [];
  try { for (const p of paths) hs.push(await openSync(p)); } catch (e) { for (const x of hs) x.h.close(); throw e; }
  const read = (k, off, len) => {
    if (scratch.length < len) scratch = new Uint8Array(Math.max(len, scratch.length * 2));
    const out = scratch.subarray(0, len);
    if (hs[k].h.read(out, { at: off }) !== len) throw new Error("error.io");
    return out;
  };
  return { read, sizes: Float64Array.from(hs, x => x.size), close: () => { for (const x of hs) { try { x.h.close(); } catch (_) { /* closed */ } } } };
}
async function writeFile(path, bytes) {
  const parts = path.split("/").filter(Boolean), name = parts.pop();
  const dir = await dirOf(parts, true);
  const put = async n => { const h = await (await dir.getFileHandle(n, { create: true })).createSyncAccessHandle(); try { h.truncate(0); h.write(bytes, { at: 0 }); h.flush(); } finally { h.close(); } };
  await put(name + ".part");
  const part = await dir.getFileHandle(name + ".part");
  if (typeof part.move === "function") await part.move(name);
  else { await put(name); await dir.removeEntry(name + ".part"); }
}
const hex = buf => [...new Uint8Array(buf)].map(b => b.toString(16).padStart(2, "0")).join("");
async function init(m) {
  if (mods[m.which]) return;
  const bytes = await (await fetch(m.wasmUrl)).arrayBuffer();
  if (m.sha256 && hex(await crypto.subtle.digest("SHA-256", bytes)) !== m.sha256) throw Object.assign(new Error("error.featWasmHash"), { key: "error.featWasmHash" });
  const mod = await import(m.jsUrl);
  const ex = await mod.default({ module_or_path: bytes });
  mods[m.which] = mod; mems[m.which] = ex.memory;
}
const reply = (id, r, transfer) => self.postMessage({ id, ok: true, ...r }, transfer || []), arrays = list => list.map(b => (b instanceof Uint8Array ? b : new Uint8Array(b)));
const OPS = {
  async init(m) { await init(m); const x = mods[m.which]; return { version: x.version?.(), crate: x.crate_version?.() }; },
  async alias(m) { mods[m.which] = mods[m.of]; mems[m.which] = mems[m.of]; return {}; },
  async detect(m) {
    const f = await openAll([m.path]), out = memSink();
    try { mods.feat.detect_file_to(f.read, f.sizes[0], m.name, m.recipe, out.write, (d, t) => self.postMessage({ id: m.id, progress: [d, t] })); return { bytes: out.bytes() }; } finally { f.close(); }   // the .wasm calls back at most once per 1 %; the pool hands `progress` to the step of the call
  },
  async interp_build(m) { const f = await openAll(m.paths), t = new Uint8Array(m.table), out = memSink(); try { mods.interp.interp_build((_, off, len) => t.subarray(off, off + len), t.length, m.parent, m.registry, m.polarity, f.read, f.sizes, m.infusion, out.write); } finally { f.close(); } return { bytes: out.bytes() }; },
  async tp_link_evidence(m) { const f = await openAll(m.paths); try { const r = mods.proofs.tp_link_evidence(m.rules, m.parent, m.structure, m.hypotheses, new Uint8Array(m.table), m.samples, m.recipe, m.polarity, m.budgetMb || 0, arrays(m.peaks), f.read, f.sizes), o = { table: r.table(), registry: r.registry(), parent: r.parent(), polarity: r.polarity(), infusion: r.infusion() }; r.free(); return o; } finally { f.close(); } },
  async tp_link_interp(m) { const f = await openAll(m.paths), out = memSink(); try { mods.interp.tp_link_interp(new Uint8Array(m.table), m.parent, m.registry, m.polarity, m.params, f.read, f.sizes, m.infusion, out.write); } finally { f.close(); } return { bytes: out.bytes() }; },
  async table_interp(m) { const t = new Uint8Array(m.table); return { text: mods.interp.table_interp((_, off, len) => t.subarray(off, off + len), t.length, m.id) }; },
  async tp_post(m) { const t = new Uint8Array(m.tp), i = mods.interp; return { text: m.kind === "filter" ? i.tp_refilter(t, m.hypotheses, m.params) : m.kind === "review" ? i.tp_review(t, m.decisions) : m.kind === "position" ? i.tp_position(t, m.smiles, m.formula, m.params) : mods.table.rete_build((_, off, len) => new Uint8Array(m.table).subarray(off, off + len), m.table.byteLength, (_, off, len) => t.subarray(off, off + len), t.length, m.samples, new Uint8Array(m.rules), m.ppm, m.minFragments) }; },
  async detect_unt(m) {
    const f = await openAll([m.path]), out = memSink();
    try { mods.untargeted.detect_untargeted_to(f.read, f.sizes[0], m.name, m.params, out.write); return { bytes: out.bytes() }; } finally { f.close(); }
  },
  async formula_id(m) { return { text: mods.untargeted.untargeted_formula_id(m.mz, m.y, m.featureMz, m.params) }; },
  async align_unt(m) {
    const f = await openAll(m.paths), out = memSink();
    try { mods.untargeted.align_untargeted_to(arrays(m.files), f.read, f.sizes, m.params, out.write, m.budgetMb || 0); return { bytes: out.bytes() }; } finally { f.close(); }
  },
  // the table of levels of a campaign folder (admin mode, `.wasm` `attributio`): `files` are the File objects of the folder, read in slices (FileReaderSync); `kind` json | csv | xlsx
  async attributio_annotate(m) {
    const out = memSink(), rd = new FileReaderSync();
    const read = (k, off, len) => new Uint8Array(rd.readAsArrayBuffer(m.files[k].slice(off, off + len)));
    const summary = mods.attributio[{ csv: "table_csv", xlsx: "table_xlsx_rows" }[m.kind] || "annotate_to"](read, m.names, out.write);
    return { summary, bytes: out.bytes() };
  },
  async preview(m) {
    const f = await openAll([m.path]);
    try { return { json: mods.feat.preview_file(f.read, f.sizes[0], m.recipe || "") }; } finally { f.close(); }
  },
  async family(m) { const f = await openAll([m.path]); try { return { json: mods.feat.family_record(f.read, f.sizes[0], m.name, m.recipe || "") }; } finally { f.close(); } },
  async drift(m) {
    return { json: mods.feat.drift_series(arrays(m.files), m.parentMz, m.baseRtTol) };
  },
  async align(m) {
    const f = await openAll(m.paths), out = memSink();
    try { mods.feat.align_series_to(arrays(m.files), f.read, f.sizes, m.recipe, out.write, m.budgetMb || 0, m.campioni || undefined); return { bytes: out.bytes() }; } finally { f.close(); }
  },
  async hypotheses(m) {
    if (hypo) { hypo.free(); hypo = null; }
    const f = await openAll(m.paths);
    try {
      hypo = mods.proofs.run_hypotheses(arrays(m.files), f.read, f.sizes, m.rules, m.structure || "", m.parent, m.parentRt == null ? NaN : m.parentRt, m.recipe);
      return { tried: hypo.tried(), found: hypo.found(), registry: hypo.registry_json(), state: hypo.state() };
    } finally { f.close(); }
  },
  async evidence(m) {
    if (!m.state) throw Object.assign(new Error("error.featNoHypotheses"), { key: "error.featNoHypotheses" });
    if (hypo) hypo.free();
    hypo = mods.proofs.load_hypotheses(new Uint8Array(m.state));
    const nan = v => (v == null ? NaN : v), table = new Uint8Array(m.table), out = memSink();
    mods.proofs.build_evidence_to((_, off, len) => table.subarray(off, off + len), table.length, hypo, Float64Array.from(m.minutes, nan), nan(m.parentMz), nan(m.parentRt), m.recipe, out.write, m.budgetMb || 0, m.campioni || undefined);
    return { bytes: out.bytes() };
  },
  async ms2file(m) {
    const f = await openAll(m.paths), t = new Uint8Array(m.table);
    try { return { text: mods.table.export_ms2((_, off, len) => t.subarray(off, off + len), t.length, m.samples, m.spec, m.sha, m.pol, m.kind, f.read, f.sizes) }; } finally { f.close(); }
  },
  async cross_polarity(m) { const p = new Uint8Array(m.pos), n = new Uint8Array(m.neg); return { text: mods.table.cross_polarity((_, off, len) => p.subarray(off, off + len), p.length, (_, off, len) => n.subarray(off, off + len), n.length, m.params) }; }, async table_file(m) { const f = await openAll(m.paths), t = new Uint8Array(m.table); try { return { text: mods.table.export_table_auto((_, off, len) => t.subarray(off, off + len), t.length, m.samples, m.sha, m.kind, f.read, f.sizes) }; } finally { f.close(); } }, async table_view(m) { const t = new Uint8Array(m.table); return { text: mods.table.table_view_auto((_, off, len) => t.subarray(off, off + len), t.length, m.samples, m.params) }; },
  async rowtraces(m) { const f = await openAll(m.paths), t = new Uint8Array(m.table); try { return { text: mods.table.table_row_traces((_, off, len) => t.subarray(off, off + len), t.length, m.samples, m.id, f.read, f.sizes, m.ppm) }; } finally { f.close(); } },
  async "fs.write"(m) { await writeFile(m.path, m.bytes); return {}; },
  async "fs.read"(m) {
    const { h, size } = await openSync(m.path);
    try { const b = new Uint8Array(size); h.read(b, { at: 0 }); return { bytes: b }; } finally { h.close(); }
  },
  async "fs.stat"(m) { try { const { h, size } = await openSync(m.path); h.close(); return { size }; } catch (_) { return { size: -1 }; } },
  async "fs.remove"(m) {
    const parts = m.path.split("/").filter(Boolean), name = parts.pop();
    try { await (await dirOf(parts, false)).removeEntry(name); } catch (_) { /* not there */ }
    return {};
  },
  async die() { throw new WebAssembly.RuntimeError("unreachable"); },       // for the tests: a trap of the .wasm
};

OPS.dig_lr = async m => { const f = await openAll(m.paths); try { const p = mods.proofs.tp_dig_inputs(m.rules, m.parent, m.structure, m.samples, m.names, m.recipe, arrays(m.peaks), f.read, f.sizes); try { const d = mods.finder_lr.finder_lr_dig(p.files(), p.who(), m.settings, new Uint8Array(m.table), p.registry(), f.read, f.sizes); try { return { text: d.summary("{}") + "\n" }; } finally { d.free(); } } finally { p.free(); } } finally { f.close(); } };   // tp.dig_lr: the hypotheses searched in the stored peaks (proofs) and the Dig of the series (finder_lr)
// the sink of a step of the .wasm: write(offset, bytes) into one growing buffer (JavaScript memory, not the 256 MB of the .wasm); the step answers { bytes, sha256 }
function memSink() {
  let buf = new Uint8Array(1 << 20), n = 0;
  return {
    write: (off, b) => {
      if (off + b.length > buf.length) { const g = new Uint8Array(Math.max(off + b.length, buf.length * 2)); g.set(buf.subarray(0, n)); buf = g; }
      buf.set(b, off); n = Math.max(n, off + b.length);
    }, bytes: () => buf.slice(0, n),
  };
}
// JSON { error_key, params } of the .wasm, or a JavaScript error
function describe(e) {
  const text = String(e && e.message || e);
  try { const j = JSON.parse(typeof e === "string" ? e : text); if (j && j.error_key) return { key: j.error_key === "error.memoryBudget" ? "error.featMemory" : j.error_key, params: j.params || {}, text }; } catch (_) { /* not JSON */ }
  return { key: (e && e.key) || (e instanceof WebAssembly.RuntimeError ? "error.featWorker" : "error.featFailed"), params: {}, text };
}
// the Banco 2b (`.wasm` `attributio_rev`): the review of one feature, and the decisions file after a decision
Object.assign(OPS, {
  async file_key(m) { const f = await openAll([m.path]), w = mods[m.which]; try { return { text: (m.which === "untargeted" ? w.file_key_untargeted : w.file_key)(f.read, f.sizes[0], m.name, m.recipe) }; } finally { f.close(); } },
  async check_stored(m) { return { text: mods[m.which].check_stored(new Uint8Array(m.bytes), m.key) }; },
  async "fs.trim"(m) { const dir = await dirOf(m.dir.split("/").filter(Boolean), true), all = []; let freed = 0;
    for await (const [n, h] of /** @type {any} */ (dir).entries()) if (h.kind === "file") all.push({ n, f: await h.getFile() });
    let size = all.reduce((a, x) => a + x.f.size, 0); for (const x of all.sort((a, b) => a.f.lastModified - b.f.lastModified)) if (size > m.cap) { await dir.removeEntry(x.n); size -= x.f.size; freed += x.f.size; } return { freed }; },
  attributio_review: m => viaFiles(m, (read, write) => mods.attributio_rev.review_to(read, m.names, m.feature, m.pol, m.classes || "", write)),
  attributio_export: m => viaFiles(m, (read, write) => mods[m.which].export_to(read, m.names, m.kind, write)),
  attributio_ms2: m => viaFiles(m, (read, write) => mods.attributio_ms2.ms2_to(read, m.names, m.request, write)),
  async attributio_decision(m) {
    const out = memSink(), rd = new FileReaderSync();
    const read = (k, off, len) => new Uint8Array(rd.readAsArrayBuffer(m.files[k].slice(off, off + len)));
    const summary = mods.attributio_rev.decision_to(read, m.names, m.decision, out.write);
    return { summary, bytes: out.bytes() };
  },
});
if (IN_WORKER) self.onmessage = async ev => {
  const m = ev.data;
  try {
    const r = await OPS[m.op](m);
    r.mem = Math.max(0, ...Object.values(mems).map(x => x.buffer.byteLength));   // linear memory of the .wasm (it only grows): the peak of this worker
    const transfer = [];
    if (r.bytes && r.bytes.buffer instanceof ArrayBuffer) transfer.push(r.bytes.buffer);
    reply(m.id, r, transfer);
  } catch (e) {
    const trap = e instanceof WebAssembly.RuntimeError;      // a trap (or an abort for lack of memory) leaves the .wasm unusable: the supervisor replaces this worker
    self.postMessage({ id: m.id, error: describe(e), fatal: trap });
  }
};
