// @ts-check
// Flow (administrator mode): the «This browser» runner of the web. It runs the feature steps of a plan (features.detect, features.align, features.gapfill) on the .mzidx files in
// OPFS (mzlab/idx/) with the pool of feat-worker.js (the .wasm of mzFinder, used through its public API only) and reports every step as an event
// { passo, stato, mb, via, parte, di, modo } (document event `flow:web-step`, and the `onEvent` option): `via` = browser | computer (where the step can run), `parte`/`di` = file k of n
// for the steps per file, `mb` = the memory the step asks for (the Rust estimate `stima_series` for the alignment, the peak measured by the worker for the detection), `modo` = memory | slabs | refused.
// Each result is kept in OPFS (mzlab/flow/) with the key of what made it, so a run after a death of the worker starts again from the first step not finished. Docs: docs/agenti/flow-web.md.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  const LOCK = "mzlab-mem-grande", BIG_MB = 512, DIR = "mzlab/flow/";
  /** the steps this runner knows, and the ones that need a computer */
  const KNOWN = ["index.mzidx", "features.detect", "features.align", "features.gapfill"];
  const LAYERED = ["features.detect", "features.align", "features.gapfill"];
  /** @type {any} */ let pool = null;
  /** @type {any} */ let estimator = null;

  const base = (/** @type {any} */ inst) => String(inst.step).split("@")[0];
  /** a phone or a tablet: mzFlow and the high-resolution series are made for computers */
  const small = () => { const n = /** @type {any} */ (navigator); return !!G.MZLAB_PHONE || /iPad|iPhone|iPod|Android/i.test(n.userAgent || "") || (n.platform === "MacIntel" && n.maxTouchPoints > 1); };
  /** the high budget of the engine of the page in MB (0 = unknown) */
  const highMb = () => { const r = G.MZLAB_RUST; return r ? Number(r.budgetMb || (r.choice && r.choice.high_mb)) || 0 : 0; };
  const available = () => !G.MZLAB_DESKTOP && !!(G.QTOOLS && G.QTOOLS.ctx) && !!(navigator.storage && navigator.storage.getDirectory);

  /** @param {string[]} parts @param {boolean} create */
  const dirOf = async (parts, create) => { let d = await navigator.storage.getDirectory(); for (const p of parts) d = await d.getDirectoryHandle(p, { create }); return d; };

  /** the .mzidx files in OPFS (not the `.cen.mzidx`) with what identifies them: [{ name, path, key }]; the ones named by a file open in the Data tab, else all */
  const listIdx = async () => {
    const out = /** @type {any[]} */ ([]);
    try {
      const d = await dirOf(["mzlab", "idx"], false);
      for await (const [name, h] of /** @type {any} */ (d).entries()) {
        if (h.kind !== "file" || !name.endsWith(".mzidx") || name.endsWith(".cen.mzidx")) continue;
        const f = await h.getFile();
        out.push({ name, path: "mzlab/idx/" + name, key: `${f.size}:${f.lastModified}` });
      }
    } catch (_) { return []; }
    out.sort((a, b) => (a.name < b.name ? -1 : 1));
    const open = typeof E !== "undefined" && Array.isArray(E.files) ? E.files.map((/** @type {any} */ f) => String(f && f.file || "").split("#")[0]) : [];
    const held = out.filter(f => open.some((/** @type {string} */ n) => n && (f.name === n + ".mzidx" || f.name === n.replace(/\.[^.]+$/, "") + ".mzidx")));
    return held.length ? held : out;
  };

  /** the pool of feat-worker.js with the `feat` .wasm of the site; made once */
  const getPool = async () => {
    if (pool) return pool;
    const c = G.QTOOLS.ctx, u = (/** @type {string} */ p) => new URL("wasm-feat/" + p, c.base).href;
    const man = await (await fetch(u("manifest.json"))).json();
    const url = URL.createObjectURL(new Blob([c.files["feat-worker.js"]], { type: "text/javascript" }));
    const mod = await import(/* webpackIgnore: true */ url);
    const w = (/** @type {any} */ m) => ({ jsUrl: u(m.js), wasmUrl: u(m.wasm), sha256: m.sha256 });
    pool = mod.createFeatPool({ workerUrl: url, size: mod.poolSize(false, navigator.hardwareConcurrency), wasm: { feat: w(man.feat), proofs: w(man.proofs) } });
    pool.featWasm = w(man.feat);
    return pool;
  };

  /** what aligning `stored` (.mzfeatf bytes) asks for: { mb, via, files, rows } from the Rust rule, computed in a worker of its own @param {Uint8Array[]} stored @param {string} recipe */
  const stima = async (stored, recipe) => {
    const p = await getPool();
    if (!estimator) estimator = new Worker(new URL("mzfinder/lazy/worker-flow-web.js", G.QTOOLS.ctx.base), { type: "module" });
    const w = estimator, copy = stored.map(b => b.slice().buffer);
    return new Promise((resolve, reject) => {
      const done = (/** @type {MessageEvent} */ ev) => {
        w.removeEventListener("message", done);
        if (ev.data.error) reject(Object.assign(new Error(ev.data.error.text), ev.data.error)); else resolve(JSON.parse(ev.data.json));
      };
      w.addEventListener("message", done);
      w.addEventListener("error", () => { estimator = null; reject(Object.assign(new Error("estimator"), { key: "error.featWorker", params: {} })); }, { once: true });
      w.postMessage({ id: 1, wasm: p.featWasm, files: copy, recipe, budgetMb: Number(G.MZLAB_RUST && G.MZLAB_RUST.budgetMb) || 0 }, copy);
    });
  };

  /** true when `path` holds the result made for `key` (the key is kept beside it) @param {any} p @param {string} path @param {string} key */
  const fresh = async (p, path, key) => {
    try { return (await p.fs.exists(path)) && new TextDecoder().decode(await p.fs.read(path + ".key")) === key; } catch (_) { return false; }
  };
  /** @param {any} p @param {string} path @param {Uint8Array} bytes @param {string} key */
  const keep = async (p, path, bytes, key) => { await p.fs.write(path, bytes.slice()); await p.fs.write(path + ".key", new TextEncoder().encode(key)); };

  /** @param {any} o @param {any} ev */
  const emit = (o, ev) => { if (o.onEvent) o.onEvent(ev); document.dispatchEvent(new CustomEvent("flow:web-step", { detail: ev })); };

  /** `features.detect`: one job per file, the finished ones kept at once; a file with its result of this very index is read back instead @param {any} o @param {any} inst @param {any[]} files @param {string} recipe */
  const detect = async (o, inst, files, recipe) => {
    const p = await getPool(), n = files.length, out = /** @type {Uint8Array[]} */ (new Array(n));
    let done = 0, peak = 0;
    const prev = p.onProgress;
    p.onProgress = (/** @type {any} */ x) => { peak = Math.max(peak, x.memMB || 0); if (prev) prev(x); };
    emit(o, { passo: inst.id, stato: "running", via: "browser", parte: 0, di: n, mb: null, modo: null });
    try {
      await Promise.all(files.map(async (f, i) => {
        const path = DIR + f.name + ".mzfeatf", key = f.key + "|" + recipe;
        const was = await fresh(p, path, key);
        out[i] = was ? await p.fs.read(path) : (await p.detectAll([{ name: f.name, path: f.path }], recipe))[0];
        if (!was) await keep(p, path, out[i], key);
        emit(o, { passo: inst.id, stato: "running", via: "browser", parte: ++done, di: n, mb: peak || null, modo: was ? "reused" : null });
      }));
    } finally { p.onProgress = prev; }
    emit(o, { passo: inst.id, stato: "done", via: "browser", parte: n, di: n, mb: peak || null, modo: null });
    return out;
  };

  /** the refusal of the budget with the numbers, as the pipeline reports it @param {any} est */
  const refusal = (/** @type {any} */ est, /** @type {number} */ budget) => Object.assign(new Error("error.featMemory"), { key: "error.featMemory", params: { fase: "align", stima_mb: est.mb, budget_mb: budget, libero_mb: budget } });

  /** `features.align` (the table of the pool is the one with the gaps filled): the estimate first; past the budget the user is asked once, and past the high level the step is refused with its numbers
   * @param {any} o @param {any} inst @param {any[]} files @param {Uint8Array[]} stored @param {string} recipe */
  const align = async (o, inst, files, stored, recipe) => {
    if (!stored.length) throw Object.assign(new Error("err.flow.lineage"), { key: "err.flow.lineage", params: { step: inst.step } });
    const p = await getPool(), n = files.length, key = files.map(f => f.key).join(",") + "|" + recipe, path = DIR + "series.mzfeat";
    if (await fresh(p, path, key)) { emit(o, { passo: inst.id, stato: "reused", via: "browser", parte: 1, di: 1, mb: null, modo: "reused" }); return p.fs.read(path); }
    const est = await stima(stored, recipe), high = highMb(), budget = Number(G.MZLAB_RUST && G.MZLAB_RUST.budgetMb) || 0;
    const ev = { passo: inst.id, via: "browser", parte: 1, di: 1, mb: est.mb, modo: est.via };
    emit(o, { ...ev, stato: "running" });
    let raised = /** @type {number | undefined} */ (undefined);
    if (est.via === "refused") {
      const err = refusal(est, budget);
      const ok = est.mb <= high && high > budget && o.askMore ? await o.askMore({ fase: "align", stima_mb: est.mb, budget_mb: budget, libero_mb: budget, high_mb: high }) : false;
      if (!ok) { emit(o, { ...ev, stato: "refused" }); throw err; }
      raised = p.setBudget(high);
    }
    const run = () => p.align(stored, files.map(f => f.path), recipe);
    try {
      const table = est.mb > BIG_MB && navigator.locks ? await navigator.locks.request(LOCK, run) : await run();
      await keep(p, path, table, key);
      emit(o, { ...ev, stato: "done" });
      return table;
    } catch (e) { emit(o, { ...ev, stato: /** @type {any} */ (e).key === "error.featMemory" ? "refused" : "error" }); throw e; }
    finally {
      if (raised !== undefined) p.setBudget(raised);
      // a worker that grew past the big level goes at the end of the step: its memory never shrinks
      if (est.mb > BIG_MB) { p.cancel(); p.resume(); }
    }
  };

  /** the recipe text the pool reads for the steps of `plan` up to `upto` (all when none): the layers of the feature steps, which the Rust side applies as `flow run` does, and no gap filling when the plan has no `features.gapfill` @param {any} plan @param {any} [upto] */
  const recipeOf = (plan, upto) => {
    const layers = [];
    for (const inst of plan.steps) {
      const id = base(inst);
      if (LAYERED.includes(id)) layers.push({ step: id, params: inst.params || {} });
      if (inst === upto) break;
    }
    return JSON.stringify({ fill: plan.steps.some((/** @type {any} */ i) => base(i) === "features.gapfill"), layers });
  };

  /** runs the plan of S: the steps this runner knows, in the order of the plan; the others are left out with `via: "computer"`. Resolves with { steps: [{id, stato}], table }, rejects with {key, params}
   * @param {any} S @param {{ onEvent?: (e: any) => void, askMore?: (i: any) => Promise<boolean> }} [opts] */
  const run = async (S, opts = {}) => {
    const o = { askMore: G.mzlabAskMore, ...opts }, plan = S.plan;
    const files = await listIdx();
    if (!files.length) throw Object.assign(new Error("err.flow.input.missing"), { key: "err.flow.input.missing", params: {} });
    for (const inst of plan.steps) emit(o, { passo: inst.id, stato: "waiting", via: KNOWN.includes(base(inst)) ? "browser" : "computer", parte: 0, di: 0, mb: null, modo: null });
    const steps = /** @type {any[]} */ ([]);
    /** @type {Uint8Array[]} */ let stored = [];
    /** @type {Uint8Array | null} */ let table = null;
    for (const inst of plan.steps) {
      const id = base(inst), ev = { passo: inst.id, via: "browser", parte: 0, di: 0, mb: null, modo: null };
      if (id === "index.mzidx") emit(o, { ...ev, stato: "reused", parte: files.length, di: files.length });
      else if (id === "features.detect") stored = await detect(o, inst, files, recipeOf(plan, inst));
      else if (id === "features.align") table = await align(o, inst, files, stored, recipeOf(plan));
      else if (id === "features.gapfill") emit(o, { ...ev, stato: table ? "reused" : "skipped", parte: 1, di: 1 });
      else emit(o, { ...ev, stato: "skipped", via: "computer" });
      steps.push(inst.id);
    }
    return { steps, table };
  };

  /** the estimate of the alignment when every detection is already kept in OPFS, else null: lets the page refuse a series that no level of memory holds before it starts @param {any} S */
  const preflight = async (S) => {
    if (!available()) return null;
    const detectInst = S.plan.steps.find((/** @type {any} */ i) => base(i) === "features.detect");
    const detectRecipe = recipeOf(S.plan, detectInst), recipe = recipeOf(S.plan);
    const files = await listIdx(), p = await getPool();
    if (!files.length) return null;
    const stored = /** @type {Uint8Array[]} */ ([]);
    for (const f of files) {
      const path = DIR + f.name + ".mzfeatf";
      if (!await fresh(p, path, f.key + "|" + detectRecipe)) return null;
      stored.push(await p.fs.read(path));
    }
    return stima(stored, recipe);
  };

  F.runners.web = { name: "This browser", available, run, preflight, small, highMb, recipeOf };
})();
