// @ts-strict
// Browser version of mzLab (GitHub Pages): there is no server. The calls the page makes to api/... are answered by the Rust engine
// (WebAssembly: rust-bridge.js, rust-worker.js, lr.js). The student's files stay in this browser (OPFS and IndexedDB, to resume after a
// reload): nothing is uploaded anywhere.
// This script is injected into index.html by tools/build_site.py (site) and by crates/mzlab-desktop/src/update.rs (app).
(() => {
  window.MZLAB_BROWSER = true;
  const SRC = /** @type {HTMLScriptElement} */ (document.currentScript).src;
  // an old browser cannot run the engine: say it clearly instead of staying blank
  // OPFS keeps the files and their indexes
  // set by a reset (reset.js): this page follows a wipe, so a guard left in sessionStorage by the page before it must not count (read now, the engine start removes it from the address)
  const RESET_NONCE = new URLSearchParams(location.search).get("ripn") || "";
  const modern = !!(window.Worker && window.WebAssembly && window.indexedDB && window.Promise && window.Response && navigator.storage && navigator.storage.getDirectory);
  try { if ("serviceWorker" in navigator && !window.QQ_RESETTING) navigator.serviceWorker.register(new URL("../sw.js", SRC)).then(isolateOnce).catch(() => {}); } catch (_) { /* offline copy is optional */ }
  // ?nav=cache (sw.js): a newer version is complete in its cache and comes with the next load; the line that says so is lazy (sw-avviso.js)
  try { navigator.serviceWorker.addEventListener("message", e => { if (e.data && e.data.op === "sw-nuova") import(new URL("sw-avviso.js", SRC).href).then(m => m.mostra()); }); } catch (_) { /* no worker */ }
  // the first visit starts not isolated (everything works as before); once the service worker (which adds COOP/COEP) is active the page reloads ONCE,
  // at the first moment with no work going on (no open file). The session guard prevents loops; ?cap-off=isolamento turns it off.
  // After a reset the guard of the earlier page can come back (Chrome restores the sessionStorage of a page whose worker was unregistered, even after clear()):
  // such a page uses the reset's nonce as its own guard, so the stale "1" does not stop the one reload that isolates it.
  function isolateOnce() {
    try {
      if (window.crossOriginIsolated || /[?&]cap-off=[^&]*isolamento/.test(location.search) || ((seen) => seen && (!RESET_NONCE || seen === RESET_NONCE))(sessionStorage.getItem("mzlab.coiReload"))) return;
      // no open file and nothing being copied, indexed or converted (those are not in E.files until «Load data»)
      const idle = () => (typeof E === "undefined" || !Array.isArray(E.files) || !E.files.some((/** @type {any} */ f) => f && !f.gone))
        && !(typeof UPBUSY !== "undefined" && UPBUSY > 0)
        && !(typeof S !== "undefined" && S.opening)
        && !(typeof PROGRESSO !== "undefined" && PROGRESSO.busy())
        && !(typeof CONVROWS !== "undefined" && CONVROWS.rows.some((/** @type {any} */ r) => !["done", "fail", "cancel"].includes(r.state)));
      const go = () => { if (window.crossOriginIsolated) return; if (!idle()) { setTimeout(go, 1000); return; } try { sessionStorage.setItem("mzlab.coiReload", RESET_NONCE || "1"); } catch (_) { return; } location.reload(); };
      // after a reset the new page can reuse the registration the reset just unregistered: its worker is active but never claims this page, so no controllerchange comes; the one reload gives the page its worker
      navigator.serviceWorker.ready.then(() => { if (navigator.serviceWorker.controller || RESET_NONCE) go(); else navigator.serviceWorker.addEventListener("controllerchange", go, { once: true }); });
    } catch (_) { /* stays not isolated */ }
  }
  // while the engine loads the page shows the same loading screen as everywhere else (#loading, funny phrases);
  // the phase of the start (window.qqPhaseCur = {key, pct}: a catalog key and the percentage when known) replaces the phrase in the single status line of #loading
  // (ldRefresh() in index.html); an ERROR replaces it too, clearly, in the language of the page
  window.qqStep = "Preparing the calculation engine...";
  // for the ?perf meter (perf.js loads later: what happens before it is kept here and picked up by it)
  const perfMark = t => { if (window.PERF) PERF.mark(t); else if (/[?&]perf\b/.test(location.search)) (window.PERF_EARLY = window.PERF_EARLY || []).push([performance.now(), t]); };
  const step = t => { window.qqStep = t; console.debug("[mzLab]", t); perfMark(t); };
  const phase = m => { window.qqPhaseCur = m ? { key: m.key, pct: m.pct, params: m.params } : null; if (m) step(m.key); if (window.ldRefresh && window.I18N) ldRefresh(); if (typeof AVVIO !== "undefined" && AVVIO.renderFase) AVVIO.renderFase(); };
  // the ONE clean slate («New session», «Start from scratch», the start banner and the 10-second button): the same wipe as the dialog's «Full reset» (reset.js),
  // which removes everything by name and reloads with ?riparti=1. A second call while it runs is ignored. Without reset.js the page just reloads.
  window.qqHardReset = () => {
    if (window.QQ_RESETTING) return Promise.resolve();
    if (!window.QQRESET) { location.reload(); return Promise.resolve(); }
    return window.QQRESET.run("all");
  };
  window.mzlabHardReset = window.qqHardReset;
  // the failure goes to the banner of AVVIO (avvio-progressivo.js: fixed, role=alert, «Copy details», «Full reset»); the page stays visible. If AVVIO is not loaded yet it picks window.qqFailInfo up
  const fail = (key, params) => { window.qqFailed = key; window.qqPhaseCur = null; window.qqFailInfo = { key, params }; if (typeof AVVIO !== "undefined") AVVIO.segnaFallito(window.qqFailInfo); };
  document.addEventListener("DOMContentLoaded", () => { step(window.qqStep); if (window.qqPhaseCur) ldRefresh(); });

  // a short message under the header (a polite live region)
  const notice = (key, params) => {
    const show = () => {
      let el = document.getElementById("rust-note");
      if (!el) { el = document.createElement("div"); el.id = "rust-note"; el.setAttribute("role", "status"); el.setAttribute("aria-live", "polite"); el.style.cssText = "position:fixed;left:50%;bottom:16px;transform:translateX(-50%);z-index:9999;max-width:90vw;padding:8px 14px;border-radius:8px;background:var(--panel,#fff);color:var(--ink,#25282c);border:1px solid var(--accent,#888);font:14px system-ui,sans-serif"; document.body.appendChild(el); }
      el.textContent = !window.I18N ? key : key === "error.memoryBudget" ? I18N.err({ error_key: key, params }).message : I18N.t(key, params); el.hidden = false;
      clearTimeout(/** @type {any} */ (el)._t); /** @type {any} */ (el)._t = setTimeout(() => { el.hidden = true; }, 9000);
    };
    if (document.body) show(); else document.addEventListener("DOMContentLoaded", show);
  };
  // The Rust engine (rust-bridge.js: the supervisor of rust-worker.js). A phone does not start it (telefono.js), an old browser cannot run it.
  // The engine starts when the page has been parsed, not while the head is still being read: the module graph and the .wasm would compete with
  // the scripts and styles of the first paint. Opening files waits for it anyway (AVVIO.pronto), so the shell is drawn first and the engine follows.
  window.addEventListener("mzlab:db-blocked", () => phase({ key: "load.blocked", pct: null }));
  const parsed = () => new Promise(ok => { if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", () => ok(null), { once: true }); else ok(null); });
  // the .wasm download starts with the module graph, not after it: its fetch() in rust-supervisor.js takes this response (same URL, cors)
  // only a hint: if it cannot be added, the engine still starts and fetches the .wasm itself
  const wasmEarly = () => { try { const l = document.createElement("link"); l.rel = "preload"; l.as = "fetch"; l.crossOrigin = "anonymous"; l.href = new URL("wasm/mzlab_wasm_bg.wasm", SRC).href; document.head.appendChild(l); } catch (_) { /* no preload */ } };
  const rustReady = modern && !window.MZLAB_PHONE
    ? parsed().then(() => { wasmEarly(); return import(new URL("rust-bridge.js", SRC).href); }).then(m => { const b = m.createBridge({ workerUrl: new URL("rust-worker.js", SRC), notify: notice, onBlocked: () => phase({ key: "load.blocked", pct: null }) }); b.start(); window.MZLAB_RUST = b; return b; })
        .catch(e => { window.MZLAB_RUST_ERR = String(e && e.message || e); console.debug("[mzLab] Rust engine not available", e); return null; })
    : Promise.resolve(null);
  // for a diagnosis: which version of the site this page is
  try { caches.keys().then(k => console.info("[mzLab] site cache:", k.filter(n => n.startsWith(NOMI.CACHE_APP)).join(", ") || "none", "| page:", document.lastModified)).catch(() => {}); } catch (_) { /* no Cache API */ }
  // the page waits for it to decide whether a Thermo .raw can be read
  window.MZLAB_RUST_READY = rustReady;
  // reset.js: the worker holds sync handles on the files
  window.qqEngineStop = () => { try { if (window.MZLAB_RUST) window.MZLAB_RUST.stop(); } catch (_) { /* gone already */ } };
  // the engine died at start though its .wasm came up (up !== false: a missing or broken .wasm is not a cache matter) and not for memory: the stored site copy is a suspect
  const engineCacheSuspect = async (/** @type {any} */ e) => {
    const r = window.MZLAB_RUST;
    return !!(e && e.dead && r && r.offKey !== "error.engineOffMemory" && await r.up !== false);
  };
  let lr = null;
  const lrInit = async () => {
    if (window.MZLAB_PHONE) throw new Error("phone: engine not started");
    if (!modern) { fail("load.err.oldBrowser"); throw new Error("this browser is too old"); }
    // the module of the loading and the count of the stored files start with the engine, not after it
    const lrModule = import(new URL("lr.js", SRC).href);
    lrModule.catch(() => {});
    // a private window (Safari, some Firefox) refuses the browser storage: say that, a reload would not help
    try { await navigator.storage.getDirectory(); } catch (_) { fail("load.err.noStorage"); throw new Error("the browser storage is refused (private window?)"); }
    // reopening takes about 0.15 s per stored file (an index each): the limit grows with the files in the storage, 240 samples took 36 s
    const stored = (async () => { let n = 0; try { for await (const _ of /** @type {any} */ (await (await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab")).getDirectoryHandle("files")).keys()) n++; } catch (_) { /* nothing stored yet */ } return n; })();
    const b = await rustReady;
    if (!b || b.disabled) { fail("load.err.engineMissing"); throw new Error("the calculation engine is not available in this browser"); }
    try {
      lr = (await lrModule).createLr({ rb: b });
      // the browser storage held by another tab: the loading screen says so
      // the limit runs from the end of the download of the engine (a slow first visit spends it on the .wasm); with nothing stored and no other tab holding the storage
      // there is nothing to reopen: a note, and the page keeps waiting. A failed download ends in the error at the limit; one that never ends starts the limit after 5 minutes
      const u = new URL(location.href), stall = stored.then(n => ({ n, ms: +(u.searchParams.get("ripresa") || 0) || 30000 + 1000 * n }));
      const within = p => { let done = false; p.then(() => { done = true; }, () => { done = true; });
        const late = (s, rej) => { if (done) return; if (s.n || s.up === false || (window.qqPhaseCur && window.qqPhaseCur.key === "load.blocked")) { rej(Object.assign(new Error("reopening the saved files timed out"), { key: "load.err.resumeTimeout" })); return; } phase({ key: "load.slow", pct: null }); setTimeout(() => late(s, rej), s.ms); };
        return Promise.race([p, Promise.all([stall, Promise.race([b.up, new Promise(r => setTimeout(r, 300000))])]).then(([s, up]) => new Promise((_, rej) => setTimeout(() => late({ ...s, up }, rej), s.ms)))]); };
      // ?riparti=1 (full reset, desktop cold start): files, indexes and notebook; ?riparti=file (reset «Files» that the old page could not finish): what «Files» removes
      const scope = ({ 1: "all", file: "files" })[u.searchParams.get("riparti") || ""] || null, fresh = !!scope;
      if (fresh) { await within(lr.reset(scope)); u.searchParams.delete("riparti"); u.searchParams.delete("ripn"); history.replaceState(null, "", u.pathname + u.search + u.hash); }
      await within(lr.boot(fresh)); window.MZLAB_LR = lr;
      window.qqStep = ""; phase(null); perfMark("motore pronto"); import(new URL("engine-recupera.js", SRC).href).then(m => m.recoverDone()).catch(() => {});
    }
    catch (e_) { const e = /** @type {Lanciato} */ (e_);
      window.MZLAB_LR_ERR = String(e && e.message || e); lr = null;
      if (window.ERRORI && window.ERRORI.segnala) window.ERRORI.segnala(e, "engine");
      // the technical text stays in MZLAB_LR_ERR and the console: the screen speaks the language of the page
      if (e && e.key) fail(e.key);
      else if (await engineCacheSuspect(e)) {
        console.error("[mzLab] engine start failed", e);
        phase({ key: "load.engine.refreshing", pct: null });
        const m = await import(new URL("engine-recupera.js", SRC).href);
        if (await m.recoverEngine() === "already") fail("load.err.engineStartPurged");
      } else { console.error("[mzLab] engine start failed", e); fail("load.err.engineStart"); }
      throw e;
    }
  };
  const ready = lrInit();
  ready.catch(() => {});
  const RUST_ROUTES = /^(\.\/)?api\/(chrom|xic|xicbin|spectra|spectrum|dda|dda_window|mrm|filters|scanlist|nearest_scan|scaninfo|scan|scanavg|scanbin|scan_axis|manint|mantraces|manvalley|manfollow|manauto|ms2tr|fileinfo|msntree|msn_tree|scan_stats|channels|masscal|msn_grafo|msnlib\/add|msnlib\/search)\b/;

  const realFetch = window.fetch.bind(window);
  const json = (obj, status = 200) => new Response(JSON.stringify(obj), { status, headers: { "Content-Type": "application/json" } });
  window.fetch = async (input, init = {}) => {
    const raw = typeof input === "string" ? input : /** @type {Request} */ (input).url;
    if (typeof raw !== "string" || !/^(\.\/)?api\//.test(raw)) return realFetch(input, init);
    // a request that names a node of the MSn tree carries the grouping of the tree
    const url = window.INF ? window.INF.qs(raw) : raw;
    if (/^(\.\/)?api\/(ping|bye)/.test(url)) return json({ ok: true });      // presence: not needed without a server
    const tr = /** @type {any} */ (window).MZLAB_TRACE, t0 = tr ? performance.now() : 0;
    if (tr) tr.push("fetch> " + url.slice(0, 80));
    try { return await fetchApi(url, init); }
    finally { if (tr) tr.push(`fetch< ${url.slice(0, 80)} ${Math.round(performance.now() - t0)}ms`); }
  };
  const fetchApi = async (url, init) => {
    await ready;
    const method = (init.method || "GET").toUpperCase();
    const rb = await rustReady;
    if (rb && (method === "GET" || method === "POST") && RUST_ROUTES.test(url)) {
      const r = await rb.handle(method, url, init);
      if (r) return new Response(r.buf || r.text, { status: r.status, headers: { "Content-Type": r.buf ? "application/octet-stream" : r.type || "application/json", "X-Py-Ms": String(r.ms || 0), "X-Engine": "rust", ...(r.disposition ? { "Content-Disposition": r.disposition } : {}) } });
    }
    const a = await lr.handle(method, url, init);
    if (a) return new Response(a.text, { status: a.status, headers: { "Content-Type": "application/json", "X-Engine": "rust" } });
    // a request the engine could not answer (tests: it must stay empty)
    (window.MZLAB_LR_MISS = window.MZLAB_LR_MISS || []).push(method + " " + url);
    return json({ error: "operation not available", error_key: rb && rb.disabled ? "load.err.engineLost" : "err.route.unavailable", params: {} }, 501);
  };
  // "the tab is open" signal: only for the local program
  /** @type {any} */ (window).EventSource = class { constructor() {} close() {} };
  navigator.sendBeacon = () => true;
})();
