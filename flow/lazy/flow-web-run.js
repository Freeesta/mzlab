// @ts-check
// Flow (administrator mode): the «This browser» runner of the web. Its helpers (the pool of feat-worker.js with the `feat` .wasm of the site, the .mzidx files of OPFS, the estimate of the alignment,
// the recipe text) are the `kit` the live session uses (flow-live.js, flow-live-jobs.js); `run(S)` starts that session and waits until every step the browser can do is done or reused, reporting each step
// as an event { passo, stato, mb, via, parte, di, modo } (document event `flow:web-step`, and the `onEvent` option): `via` = browser | computer, `parte`/`di` = file k of n for the steps per file,
// `mb` = the memory the step asks for, `modo` = memory | slabs | refused. Docs: docs/agenti/flow-web.md.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  /** the steps this runner knows, and the ones that need a computer */
  const KNOWN = ["spectra.centroid", "index.mzidx", "features.detect", "features.align", "features.gapfill", ...(F.webMsn ? F.webMsn.STEPS : []), ...(F.webTp ? F.webTp.STEPS : []), ...(F.webIdent ? F.webIdent.STEPS : []), ...(F.webQc ? F.webQc.STEPS : []), ...(F.webSuspects ? F.webSuspects.STEPS : []), ...(F.webInsilico ? F.webInsilico.STEPS : []), ...(F.webDrift ? F.webDrift.STEPS : []), "export.table_csv", "export.mztab", "table.filter"];
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

  /** the .mzidx files in OPFS (not the `.cen.mzidx`) with what identifies them: [{ name, path, key, cen }], `cen` = the `.cen.mzidx` of the same file (the index Data keeps for the peaks of a profile file) or null;
   * the ones named by a file open in the Data tab, else all */
  const listIdx = async () => {
    const out = /** @type {any[]} */ ([]), cens = /** @type {Map<string, any>} */ (new Map());
    try {
      const d = await dirOf(["mzlab", "idx"], false);
      for await (const [name, h] of /** @type {any} */ (d).entries()) {
        if (h.kind !== "file" || !name.endsWith(".mzidx")) continue;
        const f = await h.getFile(), one = { name, path: "mzlab/idx/" + name, key: `${f.size}:${f.lastModified}`, cen: null };
        if (name.endsWith(".cen.mzidx")) cens.set(name.replace(/\.cen\.mzidx$/, ".mzidx"), one); else out.push(one);
      }
    } catch (_) { return []; }
    for (const f of out) f.cen = cens.get(f.name) || null;
    out.sort((a, b) => (a.name < b.name ? -1 : 1));
    const open = typeof E !== "undefined" && Array.isArray(E.files) ? E.files.map((/** @type {any} */ f) => String(f && f.file || "").split("#")[0]) : [];
    const held = out.filter(f => open.some((/** @type {string} */ n) => n && (f.name === n + ".mzidx" || f.name === n.replace(/\.[^.]+$/, "") + ".mzidx")));
    return held.length ? held : out;
  };

  /** the context of the Tabui package once it holds the data files: the Flow tab runs from the early scripts, before the loader has filled `ctx.files`, so a run started at once would find the rules and the worker missing @param {string} name a file that must be there */
  const tpCtx = async name => {
    for (let i = 0; i < 600; i++) {
      const c = G.QTOOLS && G.QTOOLS.ctx;
      if (c && c.files && c.files[name]) return c;
      await (G.TPLAZY && G.TPLAZY.whenLoaded ? G.TPLAZY.whenLoaded().catch(() => {}) : Promise.resolve());
      await new Promise(r => setTimeout(r, 100));
    }
    return G.QTOOLS && G.QTOOLS.ctx;
  };
  F.tpCtx = tpCtx;

  /** the pool of feat-worker.js with the `feat` .wasm of the site; made once */
  const getPool = async () => {
    if (pool) return pool;
    const c = await tpCtx("feat-worker.js"), u = (/** @type {string} */ p) => new URL("wasm-feat/" + p, c.base).href;
    const man = await (await fetch(u("manifest.json"))).json();
    const url = URL.createObjectURL(new Blob([c.files["feat-worker.js"]], { type: "text/javascript" }));
    const mod = await import(/* webpackIgnore: true */ url);
    const w = (/** @type {any} */ m) => ({ jsUrl: u(m.js), wasmUrl: u(m.wasm), sha256: m.sha256 });
    pool = mod.createFeatPool({ workerUrl: url, size: mod.poolSize(false, navigator.hardwareConcurrency), wasm: { feat: w(man.feat), proofs: w(man.proofs), ...(man.interp ? { interp: w(man.interp) } : {}), ...(man.table ? { table: w(man.table) } : {}), ...(man.finder_lr ? { finder_lr: w(man.finder_lr) } : {}), ...(man.attributio_ms2 ? { attributio_ms2: w(man.attributio_ms2) } : {}), ...(man.formula ? { formula: w(man.formula) } : {}), ...(man.flowtab ? { flowtab: w(man.flowtab) } : {}) } });
    pool.featWasm = w(man.feat);
    pool.limit = Math.max(1, mod.poolSize(false, navigator.hardwareConcurrency));
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

  /** the recipe text the pool reads for the steps of `plan` up to `upto` (all when none): the layers of the feature steps, which the Rust side applies as `flow run` does, and no gap filling when the plan has no `features.gapfill` @param {any} plan @param {any} [upto] */
  const recipeOf = (plan, upto) => {
    const layers = [];
    for (const inst of plan.steps) {
      const id = base(inst);
      // the algorithm of the detection is a parameter of its layer (as `flow run` writes it): without it the pool computes the default one
      if (LAYERED.includes(id)) layers.push({ step: id, params: id === "features.detect" && inst.algorithm ? { algorithm: inst.algorithm, ...inst.params } : inst.params || {} });
      if (inst === upto) break;
    }
    return JSON.stringify({ fill: plan.steps.some((/** @type {any} */ i) => base(i) === "features.gapfill"), layers });
  };

  /** runs the plan of S: every plan in the live session (flow-live.js), which starts the jobs the browser can do as soon as the plan allows and
   * here is only waited for. Resolves with { steps, table, llm }, rejects with {key, params}
   * @param {any} S @param {{ onEvent?: (e: any) => void, askMore?: (i: any) => Promise<boolean> }} [opts] */
  const run = async (S, opts = {}) => {
    const o = { askMore: G.mzlabAskMore, ...opts };
    if (!F.live) await F.whenLate;
    return F.live.run(S, o);
  };

  /** the estimate of the alignment when every detection of the live session is kept in OPFS, else null: lets the page refuse a series that no level of memory holds before it starts @param {any} S */
  const preflight = async S => (!available() || !F.live ? null : F.live.preflight(S));

  /** the pieces the live session uses: the pool of workers, the list of the indexes, the estimate, the recipe */
  const kit = { getPool, listIdx, stima, recipeOf, highMb, KNOWN, poolIfAny: () => pool };
  F.runners.web = { name: "This browser", available, run, preflight, small, highMb, recipeOf, kit };
})();
