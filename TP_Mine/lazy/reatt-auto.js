// @ts-check
// TPMINE-PRIVATE  TPREATTAUTO: the automatic calculation of the reactivity of the parent in Tabui (administrator mode only; docs/agenti/reattivita.md). When the TPs are done and the parent has a SMILES,
// the calculation starts by itself in chem-worker.js (the .wasm `chem` is fetched at that moment, never before) or, in the desktop app, on the native route feat/grafo; the TPs always come first:
// a new run of the TPs stops the calculation (the worker is terminated) and it starts again after them. One calculation at a time. The result of a SMILES + engine + options is kept for the session.
// The panel (TPREATT, reattivita.js) shows "waiting for TPs", "computing…", "failed: …" or the result; the bar under it has the status, the options, «Recompute» and (after a failure) «Retry».
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  const admin = () => typeof G.uipAdminOn === "function" && G.uipAdminOn();
  const DEFAULTS = { solvent: "water", conformers: 4, protomer: "none", bde: false };
  /** @type {{ host: HTMLElement|null, ctl: HTMLElement|null, smiles: string, gen: number, worker: Worker|null, workerUrl: string, tpsBusy: boolean, ready: boolean, timer: any, phase: string, opts: { solvent: string, conformers: number, protomer: string, bde: boolean }, cache: Map<string, any>, version: string, native: Promise<any>|null, pool: any, calls: Map<number, { ok: (r: any) => void, ko: (e: any) => void }>, seq: number, man: any, canon: Map<string, string> }} */
  const st = { host: null, ctl: null, smiles: "", gen: 0, worker: null, workerUrl: "", tpsBusy: false, ready: false, timer: null, phase: "idle", opts: { ...DEFAULTS }, cache: new Map(), version: "", native: null, pool: null, calls: new Map(), seq: 0, man: null, canon: new Map() };

  const CSS = `.tpra{display:flex;flex-wrap:wrap;align-items:center;gap:6px 12px;margin:6px 0;font-size:12px}.tpra label{display:inline-flex;align-items:center;gap:4px}
.tpra select,.tpra input{min-height:28px;font:inherit;color:var(--ink);background:var(--panel);border:1px solid var(--line);border-radius:4px;padding:2px 6px}.tpra input{width:4.5em}
.tpra button{min-height:28px;min-width:28px;padding:2px 10px}.tpra .tpra-st{color:var(--muted)}`;

  /** @param {string} text */
  const status = text => { const e = st.ctl && st.ctl.querySelector(".tpra-st"); if (e) e.textContent = text; };
  /** @param {string} phase */
  const setPhase = phase => { st.phase = phase; if (st.ctl) st.ctl.dataset.phase = phase; const r = st.ctl && st.ctl.querySelector(".tpra-retry"); if (r) /** @type {HTMLElement} */ (r).hidden = phase !== "failed"; syncButtons(); };
  const syncButtons = () => {
    const re = st.ctl && /** @type {HTMLButtonElement|null} */ (st.ctl.querySelector(".tpra-re")); if (!re) return;
    re.disabled = st.tpsBusy || !st.smiles; re.title = st.tpsBusy ? "The TPs are being calculated: the reactivity waits for them" : "";
  };

  function build() {
    if (!st.ctl || st.ctl.firstChild) return;
    if (!document.getElementById("tpra-css")) { const s = document.createElement("style"); s.id = "tpra-css"; s.textContent = CSS; document.head.appendChild(s); }
    st.ctl.innerHTML = `<div class="tpra" role="group" aria-label="Reactivity calculation"><span class="tpra-st" role="status"></span>`
      + `<label>Solvent <select class="tpra-sol" aria-label="Solvent"><option value="water">Water</option><option value="none">None</option></select></label>`
      + `<label>Form <select class="tpra-form" aria-label="Protonation form"><option value="none">As drawn</option><option value="ph7">Most probable at pH 7</option></select></label>`
      + `<label>C–H ranking <select class="tpra-bde" aria-label="C–H site ranking"><option value="0">Fukui f(−)</option><option value="1">Fukui f(−) + radical energy</option></select></label>`
      + `<label>Conformers <input class="tpra-conf" type="number" min="1" max="16" step="1" value="${DEFAULTS.conformers}" aria-label="Conformers"></label>`
      + `<button type="button" class="tpra-re">Recompute</button><button type="button" class="tpra-retry" hidden>Retry</button></div>`;
    const sol = /** @type {HTMLSelectElement} */ (st.ctl.querySelector(".tpra-sol")), conf = /** @type {HTMLInputElement} */ (st.ctl.querySelector(".tpra-conf")), form = /** @type {HTMLSelectElement} */ (st.ctl.querySelector(".tpra-form")), bde = /** @type {HTMLSelectElement} */ (st.ctl.querySelector(".tpra-bde"));
    const read = () => { st.opts = { solvent: sol.value === "none" ? "none" : "water", conformers: Math.min(16, Math.max(1, Math.round(+conf.value) || DEFAULTS.conformers)), protomer: form.value === "ph7" ? "ph7" : "none", bde: bde.value === "1" }; };
    sol.addEventListener("change", read); form.addEventListener("change", read); bde.addEventListener("change", read); conf.addEventListener("change", () => { read(); conf.value = String(st.opts.conformers); });
    const go = () => { read(); recompute(); };
    /** @type {HTMLElement} */ (st.ctl.querySelector(".tpra-re")).addEventListener("click", go);
    /** @type {HTMLElement} */ (st.ctl.querySelector(".tpra-retry")).addEventListener("click", go);
  }

  /** the canonical SMILES of OpenChemLib (the trimmed text when it cannot read it) @param {string} s @returns {Promise<string>} */
  async function canonical(s) {
    const hit = st.canon.get(s); if (hit) return hit;
    let c = s.trim();
    try { const OCL = await import(new URL("vendor/openchemlib.js", G.QTOOLS.ctx.base).href); c = OCL.Molecule.fromSmiles(c).toSmiles() || c; } catch (_) { /* the text itself */ }
    st.canon.set(s, c); return c;
  }

  // ---------------------------------------------------------------- engines
  function stopWorker() {
    if (st.worker) { st.worker.terminate(); st.worker = null; }
    const cs = [...st.calls.values()]; st.calls.clear(); for (const c of cs) c.ko({ error_key: "cancelled", params: {} });
  }
  async function manifest() {
    if (!st.man) st.man = (await (await fetch(new URL("wasm-feat/manifest.json", G.QTOOLS.ctx.base).href)).json()).chem;
    if (!st.man) throw { error_key: "error.featFailed", params: {} };
    return st.man;
  }
  /** @returns {Promise<Worker>} */
  async function ensureWorker() {
    if (st.worker) return st.worker;
    if (!st.workerUrl) st.workerUrl = URL.createObjectURL(new Blob([G.QTOOLS.ctx.files["chem-worker.js"]], { type: "text/javascript" }));
    const w = new Worker(st.workerUrl, { type: "module" });
    w.onmessage = ev => { const c = st.calls.get(ev.data.id); if (c) { st.calls.delete(ev.data.id); ev.data.ok ? c.ok(ev.data) : c.ko(ev.data.error); } };
    w.onerror = () => { if (st.worker === w) st.worker = null; const cs = [...st.calls.values()]; st.calls.clear(); for (const c of cs) c.ko({ error_key: "error.featWorker", params: {} }); };
    st.worker = w; return w;
  }
  /** @param {string} smiles @param {string} opts @returns {Promise<{ json: string, version: string, ms: number }>} */
  async function viaWorker(smiles, opts) {
    const man = await manifest(), u = (/** @type {string} */ p) => new URL("wasm-feat/" + p, G.QTOOLS.ctx.base).href, w = await ensureWorker(), id = ++st.seq;
    return new Promise((ok, ko) => { st.calls.set(id, { ok, ko }); w.postMessage({ id, smiles, opts, jsUrl: u(man.js), wasmUrl: u(man.wasm), sha256: man.sha256 || "" }); });
  }
  /** the app: the native route feat/grafo, one call at a time (a native call cannot be stopped, a cancelled one is ignored when it ends) @param {string} smiles @param {string} opts */
  async function viaNative(smiles, opts) {
    const prev = st.native; if (prev) await prev.catch(() => {});
    if (!st.pool) { const url = URL.createObjectURL(new Blob([G.QTOOLS.ctx.files["feat-desktop-pool.js"]], { type: "text/javascript" })); st.pool = (await import(url)).createDesktopPool({ notify() {} }); }
    const t0 = performance.now();
    const p = st.pool.reactivity(smiles, opts).then((/** @type {string} */ json) => ({ json, version: "mzlab-chem/native", ms: performance.now() - t0 }));
    st.native = p; return p;
  }

  // ---------------------------------------------------------------- run
  /** stops what is running and what is scheduled (the generation number makes the late answers harmless) */
  function cancel() {
    st.gen++; if (st.timer != null) { if (typeof cancelIdleCallback === "function" && st.timer.idle) cancelIdleCallback(st.timer.h); else clearTimeout(st.timer.h); st.timer = null; }
    stopWorker();
  }
  /** @param {any} e */
  function fail(e) {
    const o = e && e.error_key ? e : { error_key: "error.featFailed", params: {} };
    setPhase("failed"); status("failed"); if (st.host && G.TPREATT) G.TPREATT.state(st.host, { state: "failed", error_key: o.error_key, message: o.params && o.params.detail });
  }
  async function run() {
    st.timer = null;
    if (!admin() || !st.smiles || !st.host || !G.TPREATT || st.tpsBusy) return;
    const gen = ++st.gen, host = st.host, smiles = st.smiles, opts = { ...st.opts };
    setPhase("computing"); status("computing…"); G.TPREATT.state(host, "computing");
    try {
      const canon = await canonical(smiles), key = `${canon}|${st.version}|${opts.solvent}|${opts.conformers}|${opts.protomer}|${opts.bde ? 1 : 0}`, hit = st.version ? st.cache.get(key) : null;
      if (gen !== st.gen) return;
      if (hit) { G.TPREATT.mount(host, hit); setPhase("done"); status("done (kept from this session)"); return; }
      const res = await (G.MZLAB_DESKTOP ? viaNative : viaWorker)(smiles, JSON.stringify(opts));
      if (gen !== st.gen) return;
      const r = JSON.parse(res.json); st.version = res.version; st.cache.set(`${canon}|${st.version}|${opts.solvent}|${opts.conformers}|${opts.protomer}|${opts.bde ? 1 : 0}`, r);
      G.TPREATT.mount(host, r); setPhase("done"); status(`done in ${(res.ms / 1000).toFixed(1)} s`);
    } catch (e) {
      if (gen !== st.gen || (e && /** @type {any} */ (e).error_key === "cancelled")) return;
      fail(typeof e === "string" ? (() => { try { return JSON.parse(e); } catch (_) { return null; } })() : e);
    }
  }
  /** starts after a short idle moment so that the page and the TPs keep the CPU */
  function schedule() {
    cancel();
    if (!admin() || !st.smiles || st.tpsBusy) return;
    const fire = () => { run(); };
    if (typeof requestIdleCallback === "function") st.timer = { idle: true, h: requestIdleCallback(fire, { timeout: 3000 }) };
    else st.timer = { idle: false, h: setTimeout(fire, 400) };
  }
  function recompute() { if (st.tpsBusy || !st.smiles) return; schedule(); }

  // ---------------------------------------------------------------- called by 20_tpmine.js
  /** the parent has a new SMILES (or none): whatever ran is stopped and the panel waits for the TPs @param {HTMLElement} host @param {string} smiles */
  function attach(host, smiles) {
    const again = st.ready && !!smiles && smiles === st.smiles;       // the card was drawn again after the TPs (the results page replaces it): the calculation goes on
    cancel(); st.host = host; st.smiles = smiles || ""; if (!again) st.ready = false;
    st.ctl = document.getElementById("tp-reatt-ctl"); if (st.ctl) { st.ctl.hidden = !admin() || !st.smiles; if (admin()) build(); }
    setPhase("waiting"); status(st.tpsBusy ? "waiting for TPs" : ""); if (G.TPREATT && admin() && st.smiles) G.TPREATT.state(host, "waiting");
    if (again) schedule();
  }
  /** a run of the TPs begins: the calculation stops and waits */
  function tpsStart() { st.tpsBusy = true; st.ready = false; cancel(); if (st.host && st.smiles && admin() && G.TPREATT) { G.TPREATT.state(st.host, "waiting"); setPhase("waiting"); status("waiting for TPs"); } syncButtons(); }
  /** the TPs are done: the calculation starts (when there is a SMILES) */
  function tpsDone() { st.tpsBusy = false; st.ready = true; syncButtons(); schedule(); }
  /** the run of the TPs ended without a result (error, cancelled): nothing starts by itself */
  function tpsEnd() { st.tpsBusy = false; syncButtons(); if (st.phase === "waiting") status(""); }

  G.TPREATTAUTO = { attach, tpsStart, tpsDone, tpsEnd, recompute, cancel, info: () => ({ ready: st.ready, smiles: st.smiles, phase: st.phase, version: st.version, cached: st.cache.size, opts: { ...st.opts }, tpsBusy: st.tpsBusy, worker: !!st.worker }) };
})();
