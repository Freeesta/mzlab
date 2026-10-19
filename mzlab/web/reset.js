// @ts-strict
// The one clean slate. mzLab keeps what the student loaded (files and their indexes in OPFS, the session and the notebook in IndexedDB, the offline copy of the
// app in the Cache API and the service worker) and the preferences (localStorage) in THIS browser, so a reload - even Cmd/Ctrl+Shift+R, which only skips the HTTP
// cache - brings the files back. This file removes them, by NAME (what the browser lists, never a fixed list: other parts of the program add their own folders and
// databases), and reloads. Variants: "all" (everything, as if the program had never run here) and "files" (the files, their indexes and the session; the
// preferences, language, theme and the saved libraries stay), both offered by the question of ask(); also "cold" and "cold" (below). Loaded before the engine (browser.js) so that ?reset=all|files|1 acts before anything is opened.
const RESET = (function () {
  "use strict";
  const wait = (p, ms = 4000) => Promise.race([p, new Promise(r => setTimeout(r, ms))]).catch(() => {});
  // what the variant "files" leaves in OPFS: the spectral libraries the user saved
  const KEEP_OPFS = ["librerie"];
  const KEEP_IDB = ["mzlab-libs", "mzlab-libdir"];                    // ... and their databases
  const KNOWN_IDB = [NOMI.DB, typeof NOMI !== "undefined" && NOMI.DB_OLD ? NOMI.DB_OLD : "qqq_lab", "mzlab-libs", "mzlab-libdir"];
  // "cold" (the desktop app, first page of a new run): the files and the session like "files", plus the working state kept in localStorage (map, peak and list
  // choices, sidebar layout...); what the user set on purpose stays: language, theme and settings, reading aids, the update notice, library settings, seen hints
  const KEEP_LS = /^((mzlab|qqq)\.(lang|prefs|a11y|set\.a11y|upd\.|reset\.note|lib\.|tour\.|telefono|logo\.)|(mzlab|qqq)-(theme|labels))/;
  // sessionStorage: present while this run of the app lives (a reload keeps it, a new start does not)
  const RUN_KEY = typeof NOMI !== "undefined" ? NOMI.KEY("run") : "mzlab.run";
  const RUN_KEY_OLD = typeof NOMI !== "undefined" && NOMI.OLD_KEY ? NOMI.OLD_KEY("run") : "qqq.run";
  const MODES = { 1: "all", all: "all", files: "files", cold: "cold" };
  const modeOf = q => MODES[q] || null;

  async function opfs(mode, rep) {
    const root = await navigator.storage.getDirectory(), names = [];
    for await (const [n] of /** @type {any} */ (root).entries()) names.push(n);
    for (const n of names) {
      if (mode !== "all" && KEEP_OPFS.includes(n)) continue;
      try { await root.removeEntry(n, { recursive: true }); rep.opfs.push(n); } catch (e) { rep.errors.push("opfs " + n + ": " + (e && e.name)); }
    }
  }
  const delDb = name => new Promise(r => { const q = indexedDB.deleteDatabase(name); q.onsuccess = q.onerror = () => r(true); q.onblocked = () => r(false); });
  async function idb(mode, rep) {
    let names = KNOWN_IDB.slice();
    try { if (indexedDB.databases) names = [...new Set([...names, ...(await indexedDB.databases()).map(d => /** @type {string} */ (d.name))])]; } catch (e) { /* the fixed names only */ }
    for (const n of names) {
      if (mode !== "all" && KEEP_IDB.includes(n)) continue;
      const ok = await wait(delDb(n), 3000);
      // blocked: another connection is open; the deletion is queued and finishes when the page reloads
      (ok ? rep.idb : rep.blocked).push(n);
    }
  }
  async function caches_(mode, rep) {
    for (const k of await caches.keys()) {
      if (mode !== "all" && !k.startsWith(NOMI.CACHE_BIG) && !k.startsWith("qqq-big-")) continue;
      if (await caches.delete(k)) rep.caches.push(k);
    }
  }
  async function wipe(mode = "all") {
    const rep = { mode, opfs: [], idb: [], blocked: [], caches: [], sw: 0, keys: 0, errors: /** @type {string[]} */ ([]) };
    // the engine worker holds sync handles on the files: it must go first
    try { if (window.qqEngineStop) window.qqEngineStop(); } catch (e) { /* no engine */ }
    // the development server keeps the session in memory
    if (!window.QQQ_BROWSER) await wait(fetch("api/new", { method: "POST", body: JSON.stringify({ fresh: true }) }));
    if (navigator.storage && /** @type {any} */ (navigator.storage).getDirectory) await wait(opfs(mode, rep));
    if (window.indexedDB) await wait(idb(mode, rep), 12000);
    if (window.caches) await wait(caches_(mode, rep));
    if (mode === "all") {
      if (navigator.serviceWorker) await wait((async () => { for (const r of await navigator.serviceWorker.getRegistrations()) { await r.unregister(); rep.sw++; } })());
      try {
        localStorage.clear(); sessionStorage.clear();
      } catch (e) { rep.errors.push("storage"); }
    } else {
      try {
        const sk = typeof NOMI !== "undefined" ? NOMI.KEY("sessione") : "mzlab.sessione";
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
  // wipe, then load the page again from the network without the ?reset parameter; ?riparti=1 makes the new engine delete saved files itself when a database was blocked
  async function run(mode, opt = {}) {
    window.QQ_RESETTING = true;
    const msg = document.getElementById("ldmsg"); if (msg && window.I18N) msg.textContent = I18N.t("reset.busy");
    let rep;
    try { rep = await wipe(mode); } catch (e) { window.QQ_RESETTING = false; throw e; }
    if (opt.mark) { try { sessionStorage.setItem(RUN_KEY, "1"); sessionStorage.setItem(RUN_KEY_OLD, "1"); } catch (e) { /* none */ } }
    if (opt.reload === false) return rep;
    const u = new URL(location.href); if (mode === "all") { u.search = "?riparti=1"; u.hash = ""; } else { u.searchParams.delete("reset"); u.searchParams.delete("riparti"); if (rep.blocked.length || mode !== "files") u.searchParams.set("riparti", "1"); }
    // the same address (a "#/dati" hash and no parameter to drop) would only be a same-page jump: reload for real
    if (u.href === location.href) location.reload(); else location.replace(u.href);
    return rep;
  }

  // ---- «Start a new session»: the one entry to every reset (header button, reminder): a short question with two ways to clear and Cancel (Esc too)
  function ask() {
    const T = (k, o) => I18N.t(k, o), btn = (m, cls, lab, sub) => `<button type="button" class="rs-big ${cls}" data-reset="${m}"><b>${lab}</b><span>${sub}</span></button>`;
    document.getElementById("resetdlg")?.remove();
    const d = document.createElement("dialog"); d.id = "resetdlg"; d.setAttribute("aria-labelledby", "rs-t");
    d.innerHTML = `<h3 id="rs-t">${T("reset.title")}</h3>${btn("all", "rs-red", T("reset.all.btn"), T("reset.all.sub"))}${btn("files", "rs-blue", T("reset.files.btn"), T("reset.files.sub"))}<div class="rs-end"><button type="button" class="rs-cancel" data-reset="">${T("reset.cancel")}</button></div>`;
    document.body.appendChild(d);
    d.addEventListener("click", e => { const b = /** @type {HTMLElement | null} */ (/** @type {HTMLElement} */ (e.target).closest("button[data-reset]")); if (!b) return; if (b.dataset.reset) { d.querySelectorAll("button").forEach(x => { x.disabled = true; }); run(b.dataset.reset); } else d.close(); });
    d.addEventListener("close", () => d.remove());
    d.showModal(); /** @type {HTMLElement | null} */ (d.querySelector(".rs-cancel"))?.focus();
  }
  // ---- once: «the files stay in the browser» (disattivato: nessun popup)
  function remind() {
  }
  // the desktop app calls this once at start (desktop-shim.js): the first page of a new run starts clean; a reload of the page in the same run keeps everything.
  // Without sessionStorage nothing is reset (it could never be told apart from a reload).
  function cold() {
    try {
      if (sessionStorage.getItem(RUN_KEY) || sessionStorage.getItem(RUN_KEY_OLD)) return false;
      sessionStorage.setItem(RUN_KEY, "1");
      sessionStorage.setItem(RUN_KEY_OLD, "1");
      sessionStorage.removeItem(RUN_KEY);
      sessionStorage.removeItem(RUN_KEY_OLD);
    } catch (e) { return false; }
    run("cold", { mark: true });
    return true;
  }
  window.QQRESET = { wipe, run, ask, remind, modeOf, cold, last: null };
  window.MZRESET = window.QQRESET;

  // ?reset=1 | all | files: acts at once (the engine of browser.js does not start while this runs)
  const q = modeOf(new URLSearchParams(location.search).get("reset"));
  if (q) { window.QQ_RESETTING = true; run(q); }
  return window.QQRESET;
})();
