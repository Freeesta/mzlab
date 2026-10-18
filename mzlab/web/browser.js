// @ts-strict
// Browser version of mzLab (GitHub Pages): there is no server. The calls the page makes to api/... are answered by the Rust engine
// (WebAssembly: rust-bridge.js, rust-worker.js, lr.js). The student's files stay in this browser (OPFS and IndexedDB, to resume after a
// reload): nothing is uploaded anywhere.
// This script is added to index.html only by tools/build_site.py; the local program does not load it.
(() => {
  window.MZLAB_BROWSER = true;
  window.QQQ_BROWSER = true; // kept from the old name: backward compatibility
  const SRC = /** @type {HTMLScriptElement} */ (document.currentScript).src;
  // an old browser cannot run the engine: say it clearly instead of staying blank
  // OPFS keeps the files and their indexes
  const modern = !!(window.Worker && window.WebAssembly && window.indexedDB && window.Promise && window.Response && navigator.storage && navigator.storage.getDirectory);
  try { if ("serviceWorker" in navigator && !window.QQ_RESETTING) navigator.serviceWorker.register(new URL("../sw.js", SRC)).catch(() => {}); } catch (_) { /* offline copy is optional */ }
  // while the engine loads the page shows the same loading screen as everywhere else (#loading, funny phrases);
  // the phase of the start (window.qqPhaseCur = {key, pct}: a catalog key and the percentage when known) replaces the phrase in the single status line of #loading
  // (ldRefresh() in index.html); an ERROR replaces it too, clearly, in the language of the page
  window.qqStep = "Preparing the calculation engine...";
  // for the ?perf meter (perf.js loads later: what happens before it is kept here and picked up by it)
  const perfMark = t => { if (window.PERF) PERF.mark(t); else if (/[?&]perf\b/.test(location.search)) (window.PERF_EARLY = window.PERF_EARLY || []).push([performance.now(), t]); };
  const step = t => { window.qqStep = t; console.debug("[mzLab]", t); perfMark(t); };
  const phase = m => { window.qqPhaseCur = m ? { key: m.key, pct: m.pct, params: m.params } : null; if (m) step(m.key); if (window.ldRefresh && window.I18N) ldRefresh(); };
  // the ONE clean slate («New session» and «Start from scratch»): saved files, notebook, user lists, caches (app, engine, libraries) and service worker are removed, then the page reloads
  // from the network; ?riparti=1 makes the new worker delete the saved files itself if the database could not be deleted now (another tab holds it)
  const wait = p => Promise.race([p, new Promise(r => setTimeout(r, 4000))]).catch(() => {});
  window.qqHardReset = async () => {
    window.qqEngineStop();
    // the files and indexes in OPFS
    await wait((async () => { const d = await navigator.storage.getDirectory(); await d.removeEntry("mzlab", { recursive: true }); })());
    await wait(new Promise((/** @type {any} */ r) => { const q = indexedDB.deleteDatabase(NOMI.DB); q.onsuccess = q.onerror = q.onblocked = () => r(); }));
    if (NOMI.DB_OLD) await wait(new Promise((/** @type {any} */ r) => { const q = indexedDB.deleteDatabase(NOMI.DB_OLD); q.onsuccess = q.onerror = q.onblocked = () => r(); }));
    await wait((async () => { for (const k of await caches.keys()) if (k.startsWith(NOMI.CACHE_PREFIX) || k.startsWith("qqq-") || k === "mzlab-libs") await caches.delete(k); })());
    await wait((async () => { for (const r of await navigator.serviceWorker.getRegistrations()) await r.unregister(); })());
    try { localStorage.clear(); sessionStorage.clear(); } catch (_) {}
    const u = new URL(location.href); u.search = "?riparti=1"; u.hash = ""; location.replace(u.href);
  };
  window.mzlabHardReset = window.qqHardReset;
  const restart = () => window.qqHardReset();
  // i18n.js loads after this script: the message is a catalog key, written when the page (and I18N) is there
  const fail = (key, params, again) => { window.qqFailed = key; window.qqPhaseCur = null; if (typeof AVVIO !== "undefined") AVVIO.segnaFallito(key); const show = () => { const m = document.getElementById("ldmsg"); if (m && window.I18N) m.textContent = I18N.t(key, params); if (again && window.ldAction && window.I18N) ldAction(I18N.t("load.restart"), restart); }; show(); document.addEventListener("DOMContentLoaded", show); };
  document.addEventListener("DOMContentLoaded", () => { step(window.qqStep); if (window.qqPhaseCur) ldRefresh(); });

  // a short message under the header (a polite live region)
  const notice = key => {
    const show = () => {
      let el = document.getElementById("rust-note");
      if (!el) { el = document.createElement("div"); el.id = "rust-note"; el.setAttribute("role", "status"); el.setAttribute("aria-live", "polite"); el.style.cssText = "position:fixed;left:50%;bottom:16px;transform:translateX(-50%);z-index:9999;max-width:90vw;padding:8px 14px;border-radius:8px;background:var(--panel,#fff);color:var(--ink,#25282c);border:1px solid var(--accent,#888);font:14px system-ui,sans-serif"; document.body.appendChild(el); }
      el.textContent = window.I18N ? I18N.t(key) : key; el.hidden = false;
      clearTimeout(/** @type {any} */ (el)._t); /** @type {any} */ (el)._t = setTimeout(() => { el.hidden = true; }, 9000);
    };
    if (document.body) show(); else document.addEventListener("DOMContentLoaded", show);
  };
  // The Rust engine (rust-bridge.js: the supervisor of rust-worker.js). A phone does not start it (telefono.js), an old browser cannot run it.
  const rustReady = modern && !window.QQQ_PHONE
    ? import(new URL("rust-bridge.js", SRC).href).then(m => { const b = m.createBridge({ workerUrl: new URL("rust-worker.js", SRC), notify: notice, onBlocked: () => phase({ key: "load.blocked", pct: null }) }); b.start(); window.MZLAB_RUST = b; return b; })
        .catch(e => { window.MZLAB_RUST_ERR = String(e && e.message || e); console.debug("[mzLab] Rust engine not available", e); return null; })
    : Promise.resolve(null);
  // for a diagnosis: which version of the site this page is
  try { caches.keys().then(k => console.info("[mzLab] site cache:", k.filter(n => n.startsWith(NOMI.CACHE_APP)).join(", ") || "none", "| page:", document.lastModified)).catch(() => {}); } catch (_) { /* no Cache API */ }
  // the page waits for it to decide whether a Thermo .raw can be read
  window.MZLAB_RUST_READY = rustReady;
  // reset.js: the worker holds sync handles on the files
  window.qqEngineStop = () => { try { if (window.MZLAB_RUST) window.MZLAB_RUST.stop(); } catch (_) { /* gone already */ } };
  let lr = null;
  const lrInit = async () => {
    if (window.QQQ_PHONE) throw new Error("phone: engine not started");
    if (!modern) { fail("load.err.oldBrowser"); throw new Error("this browser is too old"); }
    const b = await rustReady;
    if (!b || b.disabled) { fail("load.err.engineMissing"); throw new Error("the calculation engine is not available in this browser"); }
    try {
      lr = (await import(new URL("lr.js", SRC).href)).createLr({ rb: b });
      // the browser storage held by another tab: the loading screen says so
      const u = new URL(location.href), stall = +(u.searchParams.get("ripresa") || 0) || 30000;
      const within = p => Promise.race([p, new Promise((_, rej) => setTimeout(() => rej(Object.assign(new Error("reopening the saved files timed out"), { key: "load.err.resumeTimeout" })), stall))]);
      const fresh = u.searchParams.get("riparti") === "1";
      if (fresh) { await within(lr.reset()); u.searchParams.delete("riparti"); history.replaceState(null, "", u.pathname + u.search + u.hash); }
      await within(lr.boot(fresh)); window.MZLAB_LR = lr;
      window.qqStep = ""; phase(null); perfMark("motore pronto");
    }
    catch (e) {
      window.MZLAB_LR_ERR = String(e && e.message || e); lr = null;
      if (window.ERRORI && window.ERRORI.segnala) window.ERRORI.segnala(e, "engine");
      // the technical text stays in MZLAB_LR_ERR and the console: the screen speaks the language of the page
      if (e && e.key) fail(e.key, null, true); else { console.error("[mzLab] engine start failed", e); fail("load.err.engineStart", null, true); }
      throw e;
    }
  };
  const ready = lrInit();
  ready.catch(() => {});
  const RUST_ROUTES = /^(\.\/)?api\/(chrom|xic|spectra|spectrum|dda|mrm|filters|scanlist|nearest_scan|scaninfo|scan|scanavg|scanbin|scan_axis|ms2tr|fileinfo|msntree|msn_albero)\b/;

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
    if (rb && method === "GET" && RUST_ROUTES.test(url)) {
      const r = await rb.handle(method, url);
      if (r) return new Response(r.buf || r.text, { status: r.status, headers: { "Content-Type": r.buf ? "application/octet-stream" : "application/json", "X-Py-Ms": String(r.ms || 0), "X-Engine": "rust" } });
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
