// Browser version of mzLab (GitHub Pages): there is no server. The calls the page makes to api/... are answered by a
// Web Worker that runs the same Python code inside Pyodide. The student's files stay in the memory of this page (and in
// the browser's own storage, to resume after a reload): nothing is uploaded anywhere.
// This script is added to index.html only by tools/build_site.py; the local program does not load it.
(() => {
  window.MZLAB_BROWSER = true;
  window.QQQ_BROWSER = true; // kept from the old name: backward compatibility
  const SRC = document.currentScript.src;
  // an old browser cannot run the engine: say it clearly instead of staying blank
  const modern = !!(window.Worker && window.WebAssembly && window.indexedDB && window.Promise && window.Response);
  try { navigator.storage && navigator.storage.persist && navigator.storage.persist(); } catch (_) { /* optional: keeps the files from being evicted */ }
  try { if ("serviceWorker" in navigator) navigator.serviceWorker.register(new URL("../sw.js", SRC)).catch(() => {}); } catch (_) { /* offline copy is optional */ }
  let worker;
  // on a smartphone (telefono.js) the engine is not started: the phone sees only the Teoria and downloads nothing heavy
  // ?riparti=1 (the button «start from scratch») deletes the saved files, not the notebook; ?ripresa=ms shortens the wait for a stuck database (tests)
  const wurl = new URL("browser-worker.js", SRC);
  { const q = new URLSearchParams(location.search); if (q.get("riparti") === "1") wurl.searchParams.set("riparti", "1"); if (/^\d+$/.test(q.get("ripresa") || "")) wurl.searchParams.set("r", q.get("ripresa")); }
  try { if (modern && !window.QQQ_PHONE) worker = new Worker(wurl, { type: "module" }); } catch (_) { worker = null; }
  const pending = new Map();
  let seq = 0, isReady = false, failed = null;

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
    await wait(new Promise(r => { const q = indexedDB.deleteDatabase("qqq_lab"); q.onsuccess = q.onerror = q.onblocked = () => r(); }));
    await wait((async () => { for (const k of await caches.keys()) if (k.startsWith("qqq-") || k === "mzlab-libs") await caches.delete(k); })());
    await wait((async () => { for (const r of await navigator.serviceWorker.getRegistrations()) await r.unregister(); })());
    const u = new URL(location.href); u.search = "?riparti=1"; location.replace(u.href);
  };
  const restart = () => window.qqHardReset();
  // i18n.js loads after this script: the message is a catalog key, written when the page (and I18N) is there
  const fail = (key, params, again) => { window.qqFailed = key; window.qqPhaseCur = null; const show = () => { const m = document.getElementById("ldmsg"); if (m) m.textContent = window.I18N ? I18N.t(key, params) : key; if (again && window.ldAction && window.I18N) ldAction(I18N.t("load.restart"), restart); }; show(); document.addEventListener("DOMContentLoaded", show); };
  document.addEventListener("DOMContentLoaded", () => { step(window.qqStep); if (window.qqPhaseCur) ldRefresh(); });

  const ready = new Promise((res, rej) => {
    if (!worker && window.QQQ_PHONE) return rej(new Error("phone: engine not started"));
    if (!worker) { const t = "this browser is too old"; failed = t; fail("load.err.oldBrowser"); return rej(new Error(t)); }
    worker.onmessage = ev => {
      const m = ev.data;
      if (m.type === "step") return phase(m);
      if (m.type === "blocked") { window.qqPhaseCur = { key: "load.blocked", pct: null }; if (window.ldRefresh && window.I18N) ldRefresh(); return; }   // replaced by the next step once the other tab lets go
      if (m.type === "ready") { isReady = true; window.qqStep = ""; phase(null); perfMark("motore pronto"); try { const u = new URL(location.href); if (u.searchParams.has("riparti")) { u.searchParams.delete("riparti"); history.replaceState(null, "", u.href); } } catch (_) { /* optional */ } return res(); }
      if (m.type === "fatal") { failed = m.text; if (m.key === "load.err.resumeTimeout") fail(m.key, null, true); else fail("load.err.engineStart", { text: m.text }); return rej(new Error(m.text)); }
      const p = pending.get(m.id); if (!p) return; pending.delete(m.id); p(m);
    };
    worker.onerror = e => { failed = e.message || "worker error"; fail("load.err.worker", { text: failed }); rej(new Error(failed)); };
  });
  ready.catch(() => {});

  // Rust engine (?motore=rust, or localStorage qqq.motore = "rust"; ?motore=python forces it off): TIC, XIC and single scans are answered by
  // rust-worker.js when it can answer exactly as Python does (rust-bridge.js decides); without the option nothing of it is loaded.
  const rustOn = (() => {
    const m = /[?&]motore=(\w+)/.exec(location.search);
    if (m) return m[1] === "rust";
    try { return localStorage.getItem("qqq.motore") === "rust"; } catch (_) { return false; }
  })();
  const notice = key => {                                                    // a short message under the header (a polite live region)
    const show = () => {
      let el = document.getElementById("rust-note");
      if (!el) { el = document.createElement("div"); el.id = "rust-note"; el.setAttribute("role", "status"); el.setAttribute("aria-live", "polite"); el.style.cssText = "position:fixed;left:50%;bottom:16px;transform:translateX(-50%);z-index:9999;max-width:90vw;padding:8px 14px;border-radius:8px;background:var(--panel,#222);color:var(--fg,#fff);border:1px solid var(--accent,#888);font:14px system-ui,sans-serif"; document.body.appendChild(el); }
      el.textContent = window.I18N ? I18N.t(key) : key; el.hidden = false;
      clearTimeout(el._t); el._t = setTimeout(() => { el.hidden = true; }, 9000);
    };
    if (document.body) show(); else document.addEventListener("DOMContentLoaded", show);
  };
  const rustReady = rustOn && !window.QQQ_PHONE && modern
    ? import(new URL("rust-bridge.js", SRC).href).then(m => { const b = m.createBridge({ workerUrl: new URL("rust-worker.js", SRC), notify: notice }); b.start(); window.MZLAB_RUST = b; return b; })
        .catch(e => { console.debug("[mzLab] Rust engine not available", e); return null; })
    : Promise.resolve(null);
  window.MZLAB_RUST_READY = rustReady;                                       // the page waits for it to decide whether a Thermo .raw can be read
  const RUST_ROUTES = /^(\.\/)?api\/(chrom|xic|spectra)\b/;

  const realFetch = window.fetch.bind(window);
  const json = (obj, status = 200) => new Response(JSON.stringify(obj), { status, headers: { "Content-Type": "application/json" } });
  window.fetch = async (input, init = {}) => {
    const url = typeof input === "string" ? input : input.url;
    if (typeof url !== "string" || !/^(\.\/)?api\//.test(url)) return realFetch(input, init);
    if (/^(\.\/)?api\/(ping|bye)/.test(url)) return json({ ok: true });      // presence: not needed without a server
    await ready;
    const method = (init.method || "GET").toUpperCase();
    const rb = await rustReady;
    if (rb && method === "GET" && RUST_ROUTES.test(url)) {
      const r = await rb.handle(method, url);
      if (r) return new Response(r.text, { status: r.status, headers: { "Content-Type": "application/json", "X-Py-Ms": String(r.ms || 0), "X-Engine": "rust" } });
      rb.stats.python++;
    }
    if (rb && method === "POST" && /^(\.\/)?api\/upload/.test(url)) rb.noteUpload(new URL(url, "http://x/").searchParams.get("name"), init.body);
    let body = null, blob = null;
    const BIG = 50 << 20;      // a big file goes to the worker as a Blob (no copy: it is mounted there, see browser-worker.js)
    if (method === "POST" && /^(\.\/)?api\/upload/.test(url) && typeof Blob !== "undefined" && init.body instanceof Blob && init.body.size > BIG) blob = init.body;
    else if (init.body != null) body = typeof init.body === "string" ? new TextEncoder().encode(init.body) : new Uint8Array(await new Response(init.body).arrayBuffer());
    const id = ++seq;
    const post = rb && method === "POST" && r_ok_post(url) ? new TextDecoder().decode(body || new Uint8Array(0)) : null;     // read before the buffer is moved
    const r = await new Promise(res => { pending.set(id, res); worker.postMessage({ id, method, url, body, blob }, body ? [body.buffer] : []); });
    if (post !== null && r.status === 200) rustSession(rb, url, post);
    if (r.buf) return new Response(r.buf, { status: r.status, headers: { "Content-Type": r.ctype || "application/octet-stream", "X-Py-Ms": String(r.ms || 0) } });
    return new Response(r.text, { status: r.status, headers: { "Content-Type": "application/json", "X-Py-Ms": String(r.ms || 0) } });   // X-Py-Ms: time in Python, for the ?perf meter
  };
  const r_ok_post = url => /^(\.\/)?api\/(explore|remove|new)\b/.test(url);
  const rustSession = (rb, url, text) => {                                  // keeps the Rust engine's files in step with the session of Python
    try {
      const j = text ? JSON.parse(text) : {};
      if (/api\/explore/.test(url)) rb.noteExplore(j);
      else if (/api\/remove/.test(url)) { rb.noteRemove(j.name); rb.noteExplore(null); }
      else rb.noteNew();
    } catch (_) { /* optional */ }
  };
  window.EventSource = class { constructor() {} close() {} };                  // "the tab is open" signal: only for the local program
  navigator.sendBeacon = () => true;
})();
