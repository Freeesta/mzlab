// @ts-strict
// The one clean slate. mzLab keeps what the student loaded (files and their indexes in OPFS, the session and the notebook in IndexedDB, the offline copy of the
// app in the Cache API and the service worker) and the preferences (localStorage) in THIS browser, so a reload - even Cmd/Ctrl+Shift+R, which only skips the HTTP
// cache - brings the files back. This file removes them, by NAME (what the browser lists, never a fixed list: other parts of the program add their own folders and
// databases), and reloads. Variants: "all" (everything, as if the program had never run here) and "files" (the files, their indexes and the session; the
// preferences, language, theme and the saved libraries stay), both offered by the question of ask(); also "cold" (below) and "engine" (engine-recupera.js: only the app caches and the worker, nothing the user stored). Loaded before the engine (browser.js) so that ?reset=all|files|1 acts before anything is opened.
const RESET = (function () {
  "use strict";
  const wait = (p, ms = 4000) => Promise.race([p, new Promise(r => setTimeout(r, ms))]).catch(() => {});
  // what the variant "files" leaves in OPFS: the spectral libraries the user saved
  const KEEP_OPFS = ["librerie"];
  const OPFS_MS = 10000;
  const KEEP_IDB = ["mzlab-libs", "mzlab-libdir"];                    // ... and their databases
  const KNOWN_IDB = [NOMI.DB, NOMI.DB_OLD, "mzlab-libs", "mzlab-libdir"];
  // "cold" (the desktop app, first page of a new run): the files and the session like "files", plus the working state kept in localStorage (map, peak and list
  // choices, sidebar layout...); what the user set on purpose stays: language, theme and settings, reading aids, the update notice, library settings, seen hints
  const KEEP_LS = /^((mzlab|qqq)\.(lang|prefs|a11y|set\.a11y|upd\.|reset\.note|lib\.|tour\.|telefono|logo\.)|(mzlab|qqq)-(theme|labels))/;
  // sessionStorage: present while this run of the app lives (a reload keeps it, a new start does not)
  const RUN_KEY = NOMI.KEY("run");
  const MODES = { 1: "all", all: "all", files: "files", cold: "cold" };
  const modeOf = q => MODES[q] || null;

  // one deadline for the whole OPFS step; the folders go in parallel and each one ends in rep.opfs or rep.errors
  async function opfs(mode, rep) {
    const t0 = performance.now(), limit = /** @type {any} */ (window.QQRESET).opfsMs || OPFS_MS;
    let root, names = /** @type {string[]} */ ([]);
    try {
      root = await navigator.storage.getDirectory();
      for await (const [n] of /** @type {any} */ (root).entries()) names.push(n);
    } catch (e_) { rep.errors.push("opfs: " + ((e_ && /** @type {Lanciato} */ (e_).name) || "error")); return; }
    await Promise.all(names.filter(n => mode === "all" || !KEEP_OPFS.includes(n)).map(async n => {
      // the engine worker of the page we came from may still hold a sync handle for a moment (NoModificationAllowedError): retry until the deadline
      for (;;) {
        try { await root.removeEntry(n, { recursive: true }); rep.opfs.push(n); return; } catch (e_) {
          const nm = (e_ && /** @type {Lanciato} */ (e_).name) || "error";
          if (nm !== "NoModificationAllowedError") { rep.errors.push("opfs " + n + ": " + nm); return; }
        }
        if (performance.now() - t0 >= limit) { rep.errors.push("opfs " + n + ": timeout"); return; }
        await new Promise(r => setTimeout(r, 250));
      }
    }));
  }
  const delDb = name => new Promise(r => { const q = indexedDB.deleteDatabase(name); q.onsuccess = q.onerror = () => r(true); q.onblocked = () => r(false); });
  async function idb(mode, rep) {
    // "files" keeps what the user made in the main database (drawing and notebook, saved lists, molecules): only files, manifest, open-files list and sample roles go (qqq-db.js)
    let keepMain = false;
    // the unconfirmed notebook flush (nb-pendiente.js) is a copy of the whole notebook: the next load would merge its file list back
    if (mode === "files") try { const raw = localStorage.getItem("mzlab.nbpending"), q = raw ? JSON.parse(raw) : null; if (q && typeof q === "object") { ["session", "ui", "sampleCols", "sampleEx"].forEach(k => { delete q[k]; }); localStorage.setItem("mzlab.nbpending", JSON.stringify(q)); } } catch (_) { /* none */ }
    if (mode === "files") keepMain = !!(await wait(import("./qqq-db.js").then(m => m.clearFilesKeepWork()).catch(() => false), 5000));
    if (keepMain) rep.idb.push(NOMI.DB + " (files only)");
    let names = KNOWN_IDB.slice();
    try { if (indexedDB.databases) names = [...new Set([...names, ...(await indexedDB.databases()).map(d => /** @type {string} */ (d.name))])]; } catch (e) { /* the fixed names only */ }
    // all databases at once, each with its own short timeout
    await Promise.all(names.filter(n => (mode === "all" || !KEEP_IDB.includes(n)) && !(keepMain && n === NOMI.DB)).map(async n => {
      const ok = await wait(delDb(n), 3000);
      // blocked: another connection is open; the deletion is queued and finishes when the page reloads
      (ok ? rep.idb : rep.blocked).push(n);
    }));
  }
  async function caches_(mode, rep) {
    await Promise.all((await caches.keys()).filter(k => mode === "all" || (mode === "engine" ? k.startsWith(NOMI.CACHE_APP) : k.startsWith(NOMI.CACHE_BIG) || k.startsWith("qqq-big-"))).map(async k => { if (await caches.delete(k)) rep.caches.push(k); }));
  }
  // safety net a little above the deadline: if opfs() itself hangs (a stuck call), say so
  async function opfsGuard(mode, rep) {
    const ms = /** @type {any} */ (window.QQRESET).opfsMs || OPFS_MS;
    if ((await wait(opfs(mode, rep).then(() => true), ms + 4000)) !== true) rep.errors.push("opfs: timeout");
  }
  async function wipe(mode = "all") {
    const rep = { mode, opfs: [], idb: [], blocked: [], caches: [], sw: 0, keys: 0, errors: /** @type {string[]} */ ([]) };
    // the engine worker holds sync handles on the files: it must go first
    try { if (window.qqEngineStop) window.qqEngineStop(); } catch (e) { /* no engine */ }
    // OPFS, IndexedDB, caches and the worker registration are independent: they go at the same time, each with its own timeout
    await Promise.all([
      mode !== "engine" && navigator.storage && /** @type {any} */ (navigator.storage).getDirectory ? opfsGuard(mode, rep) : null,
      mode !== "engine" && window.indexedDB ? wait(idb(mode, rep), 12000) : null,
      window.caches ? wait(caches_(mode, rep)) : null,
      (mode === "all" || mode === "engine") && navigator.serviceWorker ? wait((async () => { await Promise.all((await navigator.serviceWorker.getRegistrations()).map(async r => { await r.unregister(); rep.sw++; })); })(), 2500) : null,
    ]);
    if (mode === "engine") { /* the stored copy of the site only: files, notebook and preferences stay (engine-recupera.js) */ }
    else if (mode === "all") {
      try {
        localStorage.clear(); sessionStorage.clear();
      } catch (e) { rep.errors.push("storage"); }
    } else {
      try {
        const sk = NOMI.KEY("sessione");
        try { localStorage.removeItem(sk); } catch (_) {}
        if (mode === "cold") {
          const ks = []; for (let i = 0; i < localStorage.length; i++) { const k = localStorage.key(i); if (k && /^(qqq|mzlab)/i.test(k) && !KEEP_LS.test(k)) ks.push(k); }
          ks.forEach(k => localStorage.removeItem(k)); rep.keys = ks.length;
        }
        sessionStorage.clear();
      } catch (e) { rep.errors.push("storage"); }
    }
    window.QQRESET.last = rep;
    return rep;
  }
  // what the new page's engine must finish (browser.js → lr.reset): "1" everything (all, cold), "file" what «Files» removes, keeping what it keeps (qqq-db.js RESET_FILES).
  // «Files» needs it when this page could not: a database blocked by another connection, an OPFS folder still held (a sync handle of an index build, a conversion,
  // another tab), or the desktop app, whose files and notebook are in the native store (store.rs), out of this page's reach
  const backstop = (mode, rep) => mode !== "files" ? "1" : rep.blocked.length || rep.errors.some(e => /^opfs\b/.test(e)) || window.MZLAB_DESKTOP ? "file" : "";
  // wipe, then load the page again from the network without the ?reset parameter; ?riparti (backstop) makes the new engine finish the deletion; ?ripn=<nonce> tells browser.js that this page follows a reset (see isolateOnce)
  async function run(mode, opt = {}) {
    // "files" keeps the notebook (drawing...): its last change is saved first, because from here on the page may not save any more (explore.js nbSave)
    if (mode === "files" && typeof nbSave === "function") await wait(Promise.resolve(nbSave(true)), 3000);
    window.QQ_RESETTING = true;
    const msg = document.getElementById("ldmsg"); if (msg && window.I18N) msg.textContent = I18N.t("reset.busy");
    // «Files» in the desktop app is the same run: the wipe empties sessionStorage, and without the mark the reloaded page would be a new run (cold: notebook and lists gone)
    let inRun = false; try { inRun = !!sessionStorage.getItem(RUN_KEY); } catch (e) { /* none */ }
    let rep;
    try { rep = await wipe(mode); } catch (e) { window.QQ_RESETTING = false; throw e; }
    if (opt.mark || (mode === "files" && inRun)) { try { sessionStorage.setItem(RUN_KEY, "1"); } catch (e) { /* none */ } }
    if (opt.reload === false) return rep;
    const u = new URL(location.href); if (mode === "all") { u.search = "?riparti=1&ripn=" + Date.now().toString(36); u.hash = ""; } else if (mode === "engine") u.searchParams.set("ripn", Date.now().toString(36)); else { u.searchParams.delete("reset"); u.searchParams.delete("riparti"); const back = backstop(mode, rep); if (back) u.searchParams.set("riparti", back); }
    // the same address (a "#/data" hash and no parameter to drop) would only be a same-page jump: reload for real
    if (u.href === location.href) location.reload(); else location.replace(u.href);
    return rep;
  }

  // ---- «Start a new session»: the one entry to every reset (header button, reminder): a short question with two ways to clear (the milder one first) and a close X (Esc too)
  function ask() {
    const T = (k, o) => I18N.t(k, o), btn = (m, cls, lab) => `<button type="button" class="rs-big ${cls}" data-reset="${m}"><b>${lab}</b></button>`;
    document.getElementById("resetdlg")?.remove();
    const d = document.createElement("dialog"); d.id = "resetdlg"; d.setAttribute("aria-labelledby", "rs-t");
    d.innerHTML = `<h3 id="rs-t">${T("reset.title")}</h3>${btn("files", "rs-blue", T("reset.files.btn"))}${btn("all", "rs-red", T("reset.all.btn"))}<button type="button" class="x rs-x" data-reset="" aria-label="${T("reset.close")}" title="${T("reset.close")}">&times;</button>`;
    document.body.appendChild(d);
    d.addEventListener("click", e => { const b = /** @type {HTMLElement | null} */ (/** @type {HTMLElement} */ (e.target).closest("button[data-reset]")); if (!b) return; if (b.dataset.reset) { d.querySelectorAll("button").forEach(x => { x.disabled = true; }); run(b.dataset.reset); } else d.close(); });
    d.addEventListener("close", () => d.remove());
    d.showModal(); /** @type {HTMLElement | null} */ (d.querySelector(".rs-x"))?.focus();
  }
  // ---- once: «the files stay in the browser» (off: no popup)
  function remind() {
  }
  // the desktop app calls this once at start (desktop-shim.js): the first page of a new run starts clean; a reload of the page in the same run keeps everything.
  // Without sessionStorage nothing is reset (it could never be told apart from a reload).
  function cold() {
    try {
      if (sessionStorage.getItem(RUN_KEY)) return false;
      sessionStorage.setItem(RUN_KEY, "1");               // probe: a storage that refuses writes is never treated as a new run
      sessionStorage.removeItem(RUN_KEY);
    } catch (e) { return false; }
    run("cold", { mark: true });
    return true;
  }
  window.QQRESET = { wipe, run, ask, remind, modeOf, cold, last: null, opfsMs: OPFS_MS };
  window.MZRESET = window.QQRESET;

  // ?reset=1 | all | files: acts at once (the engine of browser.js does not start while this runs)
  const q = modeOf(new URLSearchParams(location.search).get("reset"));
  if (q) { window.QQ_RESETTING = true; run(q); }
  return window.QQRESET;
})();
