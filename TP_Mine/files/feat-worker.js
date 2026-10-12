// TPMINE-PRIVATE  Pipeline of mzFinder in the browser: the Web Workers that run the feature engine (crates/mzlab-feat-wasm, two .wasm of their own: `feat` = detection and
// alignment, `proofs` = hypotheses and graph of the proofs) and the supervisor that drives them. One file, two roles:
//   - imported by the page as a module (from a Blob URL, like the other files of mzFinder): `createFeatPool(...)` is the supervisor;
//   - started as a module Worker (the same URL): the worker below. It reads the .mzidx files with OPFS synchronous handles (`createSyncAccessHandle`: the only reads the
//     .wasm can do while it runs, no SharedArrayBuffer) and hands the page byte arrays that are transferred, never views on the memory of the .wasm.
// One worker = one file at a time (detection) or one series (alignment, hypotheses, proofs: sequential, in the same worker); at most min(cores - 1, 4) workers (2 on iPad);
// 256 MB of WebAssembly memory each (.cargo/config.toml). A worker that dies (a trap of the .wasm, out of memory, a crash) is terminated and replaced, the job runs again once
// and the page is told (`notify`); a second death rejects the job. The OPFS files written here have the name of a finished file only when complete (written as `.part`).
// Messages of the worker: { op, id, ... } -> { id, ok: true, ...result } | { id, error: { key, params, text }, fatal }. Errors of the .wasm are JSON { error_key, params } (a recipe the engine refuses is "error.badRecipe").

const IN_WORKER = typeof WorkerGlobalScope !== "undefined" && self instanceof WorkerGlobalScope;

// ---------------------------------------------------------------- page side
export const poolSize = (small, cores) => { const n = Math.max(1, Math.min((cores || 2) - 1, 4)); return small ? Math.min(n, 2) : n; };
const fail = (key, params, text) => Object.assign(new Error(text || key), { key, params: params || {} });

// opts: { workerUrl, wasm: { feat: { jsUrl, wasmUrl, sha256? }, proofs: { ... } }, size, notify(key, params), onProgress(info), WorkerClass }
// The recipe is a JSON string: {"polarity": 1 | -1 | null, "mode": "ion" | "family"}.
// the supervisor: the workers alive, the queue of jobs, the replacement of a worker that dies
function createCore(opts) {
  const { workerUrl, wasm, notify = () => {}, WorkerClass = Worker } = opts;
  const limit = Math.max(1, opts.size || 1);
  const slots = new Set();                      // every worker alive: { w, busy, loaded:Set, dead }
  const st = { archive: null, cancelled: false };
  const queue = [];                             // jobs waiting for a worker: { run(slot), resolve, reject, tried }

  function spawn() {
    const slot = { w: new WorkerClass(workerUrl, { type: "module" }), busy: false, loaded: new Set(), dead: false, calls: new Map(), seq: 0 };
    slot.w.onmessage = ev => { const c = slot.calls.get(ev.data.id); if (c) { slot.calls.delete(ev.data.id); c(ev.data); } };
    const lost = text => { if (slot.dead) return; slot.dead = true; const cs = [...slot.calls.values()]; slot.calls.clear(); for (const c of cs) c({ error: { key: "error.featWorker", params: {}, text }, fatal: true }); };
    slot.w.onerror = e => lost(e && e.message || "worker error");
    slot.w.onmessageerror = () => lost("message error");
    slot.lost = lost;
    return slot;
  }
  // send a message and wait for the answer; a worker that dies answers with a fatal error
  function call(slot, msg, transfer) {
    return new Promise((resolve, reject) => {
      const id = ++slot.seq;
      slot.calls.set(id, r => (r.error ? reject(Object.assign(fail(r.error.key, r.error.params, r.error.text), { fatal: !!r.fatal })) : resolve(r)));
      if (slot.dead) { slot.calls.delete(id); return reject(Object.assign(fail("error.featWorker"), { fatal: true })); }
      try { slot.w.postMessage({ ...msg, id }, transfer || []); } catch (e) { slot.calls.delete(id); reject(Object.assign(fail("error.featWorker", {}, String(e)), { fatal: true })); }
    });
  }
  async function ensure(slot, which) {          // load the .wasm of `which` in the worker once
    if (slot.loaded.has(which)) return;
    const m = wasm[which];
    const r = await call(slot, { op: "init", which, jsUrl: m.jsUrl, wasmUrl: m.wasmUrl, sha256: m.sha256 || "" });
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
    slot.busy = true;
    try {
      const value = await job.run(slot, which => ensure(slot, which));
      slot.busy = false; job.resolve(value);
    } catch (e) {
      slot.busy = false;
      if (e && e.fatal) {                         // the worker is lost: a new one, and the job once more
        kill(slot);
        if (!job.tried && !st.cancelled) { job.tried = true; notify("tpmine.feat.restarted", { reason: e.key || "" }); queue.unshift(job); }
        else job.reject(e);
      } else job.reject(e);
    }
    pump();
  }
  function submit(run) {
    if (st.cancelled) return Promise.reject(fail("tpmine.feat.cancelled"));
    return new Promise((resolve, reject) => { queue.push({ run, resolve, reject, tried: false }); pump(); });
  }
  const copy = list => list.map(b => b.slice());      // inputs of a job that may run again are copied, never transferred
  return { st, slots, queue, spawn, call, submit, kill, copy };
}

// crateVersion(): the version of the crate mzlab-feat inside the .wasm (compared with cancello.json); preview(file, recipe): the estimates with their proofs, the histograms and three XIC
// of one file (JSON of mzlab_feat::params::preview_json); `recipe` may carry the 6 numbers of the panel.
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
  };
}

export function createFeatPool(opts) {
  const { st, slots, queue, spawn, call, submit, kill, copy } = createCore(opts);


  return {
    // L1-L3 of every file, in parallel; `files`: [{ name, path }] (path: OPFS path of the .mzidx). Resolves with the .mzfeatf bytes in the order of `files`.
    async detectAll(files, recipe) {
      let done = 0;
      return Promise.all(files.map(f => submit(async (slot, load) => {
        await load("feat");
        const t0 = performance.now();
        const r = await call(slot, { op: "detect", path: f.path, name: f.name, recipe });
        if (opts.onProgress) opts.onProgress({ phase: "detect", name: f.name, done: ++done, total: files.length, ms: Math.round(performance.now() - t0), memMB: r.mem / 1048576 });
        return r.bytes;
      })));
    },
    ...previewOps(submit, call, opts),
    // L4-L6: alignment and gap filling of the series in one worker. `stored`: .mzfeatf bytes in the order of `paths`. Resolves with the .mzfeat bytes.
    align(stored, paths, recipe) {
      return submit(async (slot, load) => {
        await load("feat");
        const t0 = performance.now();
        const r = await call(slot, { op: "align", files: copy(stored), paths, recipe });
        if (opts.onProgress) opts.onProgress({ phase: "align", done: 1, total: 1, ms: Math.round(performance.now() - t0), memMB: r.mem / 1048576 });
        return r.bytes;
      });
    },
    // L7-L9 in one worker: hypotheses of the parent searched in every file, then the graph of the proofs on the table. Resolves with { table, tried, found, registry }.
    proofs({ table, stored, paths, rules, parent, parentRt, minutes, parentMz, recipe }) {
      return submit(async (slot, load) => {
        await load("proofs");
        const t0 = performance.now();
        const h = await call(slot, { op: "hypotheses", files: copy(stored), paths, rules: rules.slice(), parent, parentRt, recipe });
        const t1 = performance.now();
        const e = await call(slot, { op: "evidence", table: table.slice(), minutes, parentMz, parentRt, recipe });
        if (opts.onProgress) opts.onProgress({ phase: "proofs", done: 1, total: 1, ms: Math.round(performance.now() - t0), msHypotheses: Math.round(t1 - t0), memMB: Math.max(h.mem, e.mem) / 1048576 });
        return { table: e.bytes, tried: h.tried, found: h.found, registry: h.registry, msHypotheses: Math.round(t1 - t0) };
      });
    },
    // the archive worker (the only one that writes OPFS): path = "mzlab/feat/<name>"
    fs: {
      async _call(msg, transfer) {
        if (!st.archive || st.archive.dead) st.archive = spawn();
        return call(st.archive, msg, transfer);
      },
      write(path, bytes) { return this._call({ op: "fs.write", path, bytes }, [bytes.buffer]); },
      async read(path) { return (await this._call({ op: "fs.read", path })).bytes; },
      async exists(path) { return (await this._call({ op: "fs.stat", path })).size >= 0; },
      remove(path) { return this._call({ op: "fs.remove", path }); },
    },
    cancel() {                                  // everything stops: running jobs are lost with their workers, the queued ones are rejected
      st.cancelled = true;
      for (const j of queue.splice(0)) j.reject(fail("tpmine.feat.cancelled"));
      for (const s of [...slots]) { s.lost("cancelled"); kill(s); }
      if (st.archive) { try { st.archive.w.terminate(); } catch (_) { /* gone */ } st.archive = null; }
    },
    resume() { st.cancelled = false; },
    get onProgress() { return opts.onProgress; },
    set onProgress(f) { opts.onProgress = f; },
    get workers() { return slots.size; },
    // for the tests: the worker that is working dies as a crashed tab would
    _killBusy() { for (const s of slots) if (s.busy) { s.lost("killed"); try { s.w.terminate(); } catch (_) { /* gone */ } } },
    _die() { return submit(async (slot, load) => { await load("feat"); await call(slot, { op: "die" }); }); },
  };
}

// ---------------------------------------------------------------- worker side
const mods = {}, mems = {};                              // { feat, proofs }: the wasm-bindgen modules
let hypo = null;                              // Hypotheses of the last run_hypotheses (kept in the memory of this worker for evidence)
let scratch = new Uint8Array(1 << 16);        // buffer reused by the reads of the .mzidx

async function dirOf(parts, create) {
  let d = await navigator.storage.getDirectory();
  for (const p of parts) d = await d.getDirectoryHandle(p, { create });
  return d;
}
async function openSync(path) {
  const parts = path.split("/").filter(Boolean), name = parts.pop();
  const fh = await (await dirOf(parts, false)).getFileHandle(name);
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
const reply = (id, r, transfer) => self.postMessage({ id, ok: true, ...r }, transfer || []);
const arrays = list => list.map(b => (b instanceof Uint8Array ? b : new Uint8Array(b)));

const OPS = {
  async init(m) { await init(m); return { version: mods[m.which].version(), crate: mods[m.which].crate_version() }; },
  async detect(m) {
    const f = await openAll([m.path]);
    try { const bytes = mods.feat.detect_file(f.read, f.sizes[0], m.name, m.recipe); return { bytes }; } finally { f.close(); }
  },
  async preview(m) {
    const f = await openAll([m.path]);
    try { return { json: mods.feat.preview_file(f.read, f.sizes[0], m.recipe || "") }; } finally { f.close(); }
  },
  async align(m) {
    const f = await openAll(m.paths);
    try { return { bytes: mods.feat.align_series(arrays(m.files), f.read, f.sizes, m.recipe) }; } finally { f.close(); }
  },
  async hypotheses(m) {
    if (hypo) { hypo.free(); hypo = null; }
    const f = await openAll(m.paths);
    try {
      hypo = mods.proofs.run_hypotheses(arrays(m.files), f.read, f.sizes, m.rules, m.parent, m.parentRt == null ? NaN : m.parentRt, m.recipe);
      return { tried: hypo.tried(), found: hypo.found(), registry: hypo.registry_json() };
    } finally { f.close(); }
  },
  async evidence(m) {
    if (!hypo) throw Object.assign(new Error("error.featNoHypotheses"), { key: "error.featNoHypotheses" });
    const nan = v => (v == null ? NaN : v);
    return { bytes: mods.proofs.build_evidence(m.table, hypo, Float64Array.from(m.minutes, nan), nan(m.parentMz), nan(m.parentRt), m.recipe) };
  },
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

// JSON { error_key, params } of the .wasm, or a JavaScript error
function describe(e) {
  const text = String(e && e.message || e);
  try { const j = JSON.parse(typeof e === "string" ? e : text); if (j && j.error_key) return { key: j.error_key, params: j.params || {}, text }; } catch (_) { /* not JSON */ }
  return { key: (e && e.key) || (e instanceof WebAssembly.RuntimeError ? "error.featWorker" : "error.featFailed"), params: {}, text };
}
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
