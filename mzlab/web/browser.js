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
  try { if ("serviceWorker" in navigator && !window.QQ_RESETTING) navigator.serviceWorker.register(new URL("../sw.js", SRC)).catch(() => {}); } catch (_) { /* offline copy is optional */ }
  let worker;
  // on a smartphone (telefono.js) the engine is not started: the phone sees only the Teoria and downloads nothing heavy
  // ?riparti=1 (the button «start from scratch») deletes the saved files, not the notebook; ?ripresa=ms shortens the wait for a stuck database and ?opfs=0 keeps the files in memory and IndexedDB (tests)
  // The index builder: the pool of Rust workers by default (it falls back to Python when it cannot run); ?pool=N sets the workers, ?pool=0 or localStorage qqq.pool = "0" force Python (the reference).
  const poolMode = q => {
    const v = q.get("pool");
    if (/^\d+$/.test(v || "")) return v;
    try { return localStorage.getItem("qqq.pool") === "0" ? "0" : "auto"; } catch (_) { return "auto"; }
  };
  const wurl = new URL("browser-worker.js", SRC);
  { const q = new URLSearchParams(location.search); if (q.get("riparti") === "1") wurl.searchParams.set("riparti", "1"); if (/^\d+$/.test(q.get("ripresa") || "")) wurl.searchParams.set("r", q.get("ripresa")); if (q.get("opfs") === "0") wurl.searchParams.set("opfs", "0"); wurl.searchParams.set("pool", poolMode(q)); if (window.MZLAB_DESKTOP) wurl.searchParams.set("desk", "1"); }
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
    window.qqEngineStop();
    await wait((async () => { const d = await navigator.storage.getDirectory(); await d.removeEntry("mzlab", { recursive: true }); })());     // the files and indexes in OPFS
    await wait(new Promise(r => { const q = indexedDB.deleteDatabase("qqq_lab"); q.onsuccess = q.onerror = q.onblocked = () => r(); }));
    await wait((async () => { for (const k of await caches.keys()) if (k.startsWith("qqq-") || k === "mzlab-libs") await caches.delete(k); })());
    await wait((async () => { for (const r of await navigator.serviceWorker.getRegistrations()) await r.unregister(); })());
    const u = new URL(location.href); u.search = "?riparti=1"; location.replace(u.href);
  };
  const restart = () => window.qqHardReset();
  // i18n.js loads after this script: the message is a catalog key, written when the page (and I18N) is there
  const fail = (key, params, again) => { window.qqFailed = key; window.qqPhaseCur = null; const show = () => { const m = document.getElementById("ldmsg"); if (m) m.textContent = window.I18N ? I18N.t(key, params) : key; if (again && window.ldAction && window.I18N) ldAction(I18N.t("load.restart"), restart); }; show(); document.addEventListener("DOMContentLoaded", show); };
  document.addEventListener("DOMContentLoaded", () => { step(window.qqStep); if (window.qqPhaseCur) ldRefresh(); });

  // The Pyodide worker. Started at once, or (with the flow without Pyodide, below) the first time a request needs Python.
  const startPython = () => {
  try { if (modern && !window.QQQ_PHONE && !window.QQ_RESETTING) worker = new Worker(wurl, { type: "module" }); } catch (_) { worker = null; }       // ?reset=: reset.js wipes first, the engine starts on the reloaded page
  window.qqEngineStop = () => { try { if (worker) worker.terminate(); } catch (_) { /* gone already */ } };      // reset.js: the worker holds sync handles on the files
  return new Promise((res, rej) => {
    if (!worker && window.QQQ_PHONE) return rej(new Error("phone: engine not started"));
    if (!worker) { const t = "this browser is too old"; failed = t; fail("load.err.oldBrowser"); return rej(new Error(t)); }
    worker.onmessage = ev => {
      const m = ev.data;
      if (m.type === "step") return phase(m);
      if (m.type === "blocked") { window.qqPhaseCur = { key: "load.blocked", pct: null }; if (window.ldRefresh && window.I18N) ldRefresh(); return; }   // replaced by the next step once the other tab lets go
      if (m.type === "ready") { isReady = true; window.qqStep = ""; phase(null); perfMark("motore pronto"); try { const u = new URL(location.href); if (u.searchParams.has("riparti")) { u.searchParams.delete("riparti"); history.replaceState(null, "", u.href); } } catch (_) { /* optional */ } return res(); }
      if (m.type === "fatal") { failed = m.text; if (m.key === "load.err.resumeTimeout") fail(m.key, null, true); else fail("load.err.engineStart", { text: m.text }); return rej(new Error(m.text)); }
      if (m.type === "dead") { recover(); return; }                                                                    // Python lost its footing outside a request (background work, a promise)
      if (m.type === "pong") { const q = [...pending.keys()].filter(k => String(k).startsWith("ping")); q.forEach(k => { const f = pending.get(k); pending.delete(k); f(m); }); return; }
      if (m.type === "notice") return notice(m.key);                                                                   // e.g. error.memoryBudget: the index of this file is built by Python
      if (m.type === "idx") { (window.MZLAB_IDX = window.MZLAB_IDX || []).push(m); return; }                          // which engine built the index of a file (tests, ?perf)
      const p = pending.get(m.id); if (!p) return; pending.delete(m.id); p(m);
    };
    worker.onerror = e => { failed = e.message || "worker error"; fail("load.err.worker", { text: failed }); rej(new Error(failed)); };
  });
  };
  // A tab that comes back after a freeze or a discard may hold a Python that is no longer usable (NoGilError). The engine is checked when the page
  // is shown again; if it does not answer well, or a request comes back as "dead", the page reloads itself ONCE (the files and the notebook reopen from the
  // browser's storage) instead of showing the error. If it happens again within a minute, a message in the language of the page is shown.
  const recover = () => {
    let last = 0; try { last = +sessionStorage.getItem("qqq.recovered") || 0; } catch (_) { /* optional */ }
    if (Date.now() - last < 60000 || window.MZLAB_NO_RECOVER) { notice("load.err.engineLost"); return false; }
    try { sessionStorage.setItem("qqq.recovered", String(Date.now())); } catch (_) { /* optional */ }
    window.MZLAB_RECOVERING = true;
    try { if (worker) worker.terminate(); } catch (_) { /* gone */ }
    location.reload();
    return true;
  };
  const check = () => {
    if (!isReady || !worker || window.MZLAB_RECOVERING) return;
    const id = "ping" + (++seq);
    const t = setTimeout(() => { if (pending.delete(id)) recover(); }, 8000);          // a worker that does not even answer a ping is gone
    pending.set(id, m => { clearTimeout(t); if (!m.ok) recover(); });
    worker.postMessage({ ping: id });
  };
  const isGil = x => /NoGilError|GIL not held|PyProxy/i.test(String(x && (x.name + " " + x.message) || x));       // the error as the page sees it, from any promise or script
  window.addEventListener("unhandledrejection", e => { if (isGil(e.reason)) { e.preventDefault(); recover(); } });
  window.addEventListener("error", e => { if (isGil(e.error || e.message)) { e.preventDefault(); recover(); } });
  document.addEventListener("visibilitychange", () => { if (document.visibilityState === "visible") check(); });
  window.addEventListener("pageshow", e => { if (e.persisted) check(); });
  document.addEventListener("resume", check);
  window.MZLAB_ENGINE_CHECK = check;                                          // tests
  let ready = null;                    // the engine is up: Pyodide, or (flow without Pyodide) the Rust engine

  // Rust engine (?motore=rust, or localStorage qqq.motore = "rust"; ?motore=python forces it off): TIC, XIC and single scans are answered by
  // rust-worker.js when it can answer exactly as Python does (rust-bridge.js decides); without the option nothing of it is loaded.
  const rustOn = (() => {
    const m = /[?&]motore=(\w+)/.exec(location.search);
    if (m) return m[1] === "rust";
    try { const v = localStorage.getItem("qqq.motore"); if (v) return v === "rust"; } catch (_) { /* default */ }
    return true;                                                         // Rust is the default engine; ?motore=python (or qqq.motore = "python") goes back to Python
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
  // The flow of the low-resolution files without Pyodide (lr.js; opt-in: ?lr=1 or localStorage qqq.lr = "1"; needs the Rust engine): upload, list of the
  // files, session and notebook are answered by the Rust engine; Pyodide starts (once, with a message) the first time a request needs it, and the session
  // is replayed into it. Without the Rust engine, or on any trouble, it is the classic way.
  const lrOn = rustOn && modern && !window.QQQ_PHONE && (() => {
    const m = /[?&]lr=(\w+)/.exec(location.search);
    if (m) return m[1] === "1";
    try { const v = localStorage.getItem("qqq.lr"); if (v) return v === "1"; } catch (_) { /* default */ }
    return true;                                                         // on by default with the Rust engine; ?lr=0 (or qqq.lr = "0") starts Pyodide first
  })();
  let lr = null, pyP = null;
  const lrInit = async () => {
    const b = await rustReady;
    if (!b || b.disabled) return startPython();
    try {
      lr = (await import(new URL("lr.js", SRC).href)).createLr({ rb: b });
      const u = new URL(location.href), stall = +u.searchParams.get("ripresa") || 30000;       // the browser storage held by another tab: Python's loading screen says so
      const within = p => Promise.race([p, new Promise((_, rej) => setTimeout(() => rej(new Error("reopening the saved files timed out")), stall))]);
      if (u.searchParams.get("riparti") === "1") { await within(lr.reset()); u.searchParams.delete("riparti"); history.replaceState(null, "", u.pathname + u.search + u.hash); }
      await within(lr.boot()); window.MZLAB_LR = lr;
    }
    catch (e) { console.debug("[mzLab] flow without Pyodide not available", e); window.MZLAB_LR_ERR = String(e && e.message || e); lr = null; return startPython(); }
  };
  const needPython = () => pyP || (pyP = (async () => {
    notice("lr.python.loading");
    if (lr) lr.freeze();                                                 // no more answers from here while Python starts: the session to replay is final
    await startPython();
    const keep = lr && lr.session().samples;
    lr = null; window.MZLAB_LR_DONE = true;                              // from now on the classic way: Python has the files (OPFS) and gets the session
    if (keep && keep.length) {
      const body = new TextEncoder().encode(JSON.stringify({ samples: keep, append: false }));
      await new Promise(res => { const id = ++seq; pending.set(id, res); worker.postMessage({ id, method: "POST", url: "api/explore", body }, [body.buffer]); });
    }
  })());
  ready = lrOn ? lrInit() : startPython();
  ready.catch(() => {});
  const RUST_ROUTES = /^(\.\/)?api\/(chrom|xic|spectra|spectrum|dda|mrm|filters|scanlist|nearest_scan|scaninfo|scan|scanavg)\b/;

  const realFetch = window.fetch.bind(window);
  const json = (obj, status = 200) => new Response(JSON.stringify(obj), { status, headers: { "Content-Type": "application/json" } });
  window.fetch = async (input, init = {}) => {
    const url = typeof input === "string" ? input : input.url;
    if (typeof url !== "string" || !/^(\.\/)?api\//.test(url)) return realFetch(input, init);
    if (/^(\.\/)?api\/(ping|bye)/.test(url)) return json({ ok: true });      // presence: not needed without a server
    const tr = window.MZLAB_TRACE, t0 = tr ? performance.now() : 0;
    if (tr) tr.push("fetch> " + url.slice(0, 80));
    try { return await fetchApi(url, init); }
    finally { if (tr) tr.push(`fetch< ${url.slice(0, 80)} ${Math.round(performance.now() - t0)}ms`); }
  };
  const fetchApi = async (url, init) => {
    await ready;
    const method = (init.method || "GET").toUpperCase();
    const rb = await rustReady;
    if (lr) {                                                            // flow without Pyodide: the Rust engine first, Python only when it must
      if (rb && method === "GET" && RUST_ROUTES.test(url)) {
        const r = await rb.handle(method, url);
        if (r) return new Response(r.text, { status: r.status, headers: { "Content-Type": "application/json", "X-Py-Ms": String(r.ms || 0), "X-Engine": "rust" } });
      }
      const a = lr && await lr.handle(method, url, init);
      if (a) return new Response(a.text, { status: a.status, headers: { "Content-Type": "application/json", "X-Engine": "rust" } });
      (window.MZLAB_LR_MISS = window.MZLAB_LR_MISS || []).push(method + " " + url);             // what sent the engine to Python (tests)
      await needPython();
    }
    if (rb && method === "GET" && RUST_ROUTES.test(url)) {
      const r = await rb.handle(method, url);
      if (r) return new Response(r.text, { status: r.status, headers: { "Content-Type": "application/json", "X-Py-Ms": String(r.ms || 0), "X-Engine": "rust" } });
      rb.stats.python++;
    }
    if (rb && method === "POST" && /^(\.\/)?api\/upload/.test(url)) rb.noteUpload(new URL(url, "http://x/").searchParams.get("name"), init.body);
    let body = null, blob = null;
    const BIG = 50 << 20;      // an mzML (or any big file) goes to the worker as a Blob (no copy: the worker keeps it in OPFS and mounts it, see browser-worker.js)
    if (method === "POST" && /^(\.\/)?api\/upload/.test(url) && typeof Blob !== "undefined" && init.body instanceof Blob && (init.body.size > BIG || /\.mzml$/i.test(new URL(url, "http://x/").searchParams.get("name") || ""))) blob = init.body;
    else if (init.body != null) body = typeof init.body === "string" ? new TextEncoder().encode(init.body) : new Uint8Array(await new Response(init.body).arrayBuffer());
    const id = ++seq;
    const post = rb && method === "POST" && r_ok_post(url) ? new TextDecoder().decode(body || new Uint8Array(0)) : null;     // read before the buffer is moved
    const r = await new Promise(res => { pending.set(id, res); worker.postMessage({ id, method, url, body, blob }, body ? [body.buffer] : []); });
    if (r.dead) { if (recover()) return new Promise(() => {}); r.text = JSON.stringify({ error: "engine stopped", error_key: "load.err.engineLost" }); }   // reloading, or (a second time within a minute) the message in the language of the page
    if (post !== null && r.status === 200) rustSession(rb, url, post, r.text);
    if (r.buf) return new Response(r.buf, { status: r.status, headers: { "Content-Type": r.ctype || "application/octet-stream", "X-Py-Ms": String(r.ms || 0) } });
    return new Response(r.text, { status: r.status, headers: { "Content-Type": "application/json", "X-Py-Ms": String(r.ms || 0) } });   // X-Py-Ms: time in Python, for the ?perf meter
  };
  const r_ok_post = url => /^(\.\/)?api\/(explore|remove|new)\b/.test(url);
  const rustSession = (rb, url, text, answer) => {                                  // keeps the Rust engine's files in step with the session of Python
    try {
      const j = text ? JSON.parse(text) : {};
      if (/api\/explore/.test(url)) {                                    // one slot per ITEM of the session (a mixed file makes several), not per file of the request
        let s = null; try { s = JSON.parse(answer).session; } catch (_) { /* the answer is not the session */ }
        rb.noteExplore(Array.isArray(s) ? { samples: s.map(it => ({ file: it.file })) } : j);
      }
      else if (/api\/remove/.test(url)) { rb.noteRemove(j.name); rb.noteExplore(null); }
      else rb.noteNew();
    } catch (_) { /* optional */ }
  };
  window.EventSource = class { constructor() {} close() {} };                  // "the tab is open" signal: only for the local program
  navigator.sendBeacon = () => true;
})();
