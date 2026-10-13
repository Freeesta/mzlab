// @ts-strict
// The one clean slate. mzLab keeps what the student loaded (files and their indexes in OPFS, the session and the notebook in IndexedDB, the offline copy of the
// app in the Cache API and the service worker) and the preferences (localStorage) in THIS browser, so a reload - even Cmd/Ctrl+Shift+R, which only skips the HTTP
// cache - brings the files back. This file removes them, by NAME (what the browser lists, never a fixed list: other parts of the program add their own folders and
// databases), and reloads. Two variants: "all" (everything, as if the program had never run here) and "files" (the files, their indexes and the session; the
// preferences, language, theme and the saved libraries stay). Loaded before the engine (browser.js) so that ?reset=all|files|1 acts before anything is opened.
(() => {
  "use strict";
  const wait = (p, ms = 4000) => Promise.race([p, new Promise(r => setTimeout(r, ms))]).catch(() => {});
  const KEEP_OPFS = ["librerie"];                                     // what the variant "files" leaves in OPFS: the spectral libraries the user saved
  const KEEP_IDB = ["mzlab-libs", "mzlab-libdir"];                    // ... and their databases
  const KNOWN_IDB = ["qqq_lab", "mzlab-libs", "mzlab-libdir"];
  const MODES = { 1: "all", all: "all", files: "files" };
  const modeOf = q => MODES[q] || null;

  async function opfs(mode, rep) {
    const root = await navigator.storage.getDirectory(), names = [];
    for await (const [n] of /** @type {any} */ (root).entries()) names.push(n);
    for (const n of names) {
      if (mode === "files" && KEEP_OPFS.includes(n)) continue;
      try { await root.removeEntry(n, { recursive: true }); rep.opfs.push(n); } catch (e) { rep.errors.push("opfs " + n + ": " + (e && e.name)); }
    }
  }
  const delDb = name => new Promise(r => { const q = indexedDB.deleteDatabase(name); q.onsuccess = q.onerror = () => r(true); q.onblocked = () => r(false); });
  async function idb(mode, rep) {
    let names = KNOWN_IDB.slice();
    try { if (indexedDB.databases) names = [...new Set([...names, ...(await indexedDB.databases()).map(d => /** @type {string} */ (d.name))])]; } catch (e) { /* the fixed names only */ }
    for (const n of names) {
      if (mode === "files" && KEEP_IDB.includes(n)) continue;
      const ok = await wait(delDb(n), 3000);
      (ok ? rep.idb : rep.blocked).push(n);                           // blocked: another connection is open; the deletion is queued and finishes when the page reloads
    }
  }
  async function caches_(mode, rep) {
    for (const k of await caches.keys()) {
      if (mode === "files" && !k.startsWith("qqq-big-")) continue;
      if (await caches.delete(k)) rep.caches.push(k);
    }
  }
  async function wipe(mode = "all") {
    const rep = { mode, opfs: [], idb: [], blocked: [], caches: [], sw: 0, keys: 0, errors: /** @type {string[]} */ ([]) };
    try { if (window.qqEngineStop) window.qqEngineStop(); } catch (e) { /* no engine */ }              // the engine worker holds sync handles on the files: it must go first
    if (!window.QQQ_BROWSER) await wait(fetch("api/new", { method: "POST", body: JSON.stringify({ fresh: true }) }));      // the development server keeps the session in memory
    if (navigator.storage && /** @type {any} */ (navigator.storage).getDirectory) await wait(opfs(mode, rep));
    if (window.indexedDB) await wait(idb(mode, rep), 12000);
    if (window.caches) await wait(caches_(mode, rep));
    if (mode === "all") {
      if (navigator.serviceWorker) await wait((async () => { for (const r of await navigator.serviceWorker.getRegistrations()) { await r.unregister(); rep.sw++; } })());
      try {
        const ks = []; for (let i = 0; i < localStorage.length; i++) { const k = localStorage.key(i); if (k && /^(qqq|mzlab)/i.test(k)) ks.push(k); }
        ks.forEach(k => localStorage.removeItem(k)); rep.keys = ks.length; sessionStorage.clear();
      } catch (e) { rep.errors.push("storage"); }
    } else { try { sessionStorage.clear(); } catch (e) { /* none */ } }
    window.QQRESET.last = rep;
    return rep;
  }
  // wipe, then load the page again from the network without the ?reset parameter; ?riparti=1 makes the new engine delete saved files itself when a database was blocked
  async function run(mode, opt = {}) {
    window.QQ_RESETTING = true;
    const msg = document.getElementById("ldmsg"); if (msg && window.I18N) msg.textContent = I18N.t("reset.busy");
    const rep = await wipe(mode);
    if (opt.reload === false) return rep;
    const u = new URL(location.href); u.searchParams.delete("reset"); u.searchParams.delete("riparti"); if (rep.blocked.length || mode === "all") u.searchParams.set("riparti", "1");
    location.replace(u.href);
    return rep;
  }

  // ---- the question: what is lost, in words, one button per variant
  function ask(only) {
    const T = (k, o) => I18N.t(k, o), modes = only ? [only] : ["all", "files"];
    document.getElementById("resetdlg")?.remove();
    const d = document.createElement("dialog"); d.id = "resetdlg"; d.setAttribute("aria-labelledby", "resetdlg-t");
    d.innerHTML = `<h3 id="resetdlg-t">${T("reset.title")}</h3><p>${T("reset.intro")}</p>` +
      modes.map(m => `<div class="rs-opt"><button type="button" class="${m === "all" ? "danger" : ""}" data-reset="${m}">${I18N.t(`reset.${m}.btn`)}</button><span class="sm">${I18N.t(`reset.${m}.desc`)}</span></div>`).join("") +
      `<div class="rs-end"><button type="button" data-reset="">${T("reset.cancel")}</button></div>`;
    document.body.appendChild(d);
    d.addEventListener("click", e => { const b = /** @type {HTMLElement | null} */ (/** @type {HTMLElement} */ (e.target).closest("button[data-reset]")); if (!b) return; if (b.dataset.reset) { d.querySelectorAll("button").forEach(x => { x.disabled = true; }); run(b.dataset.reset); } else d.close(); });
    d.addEventListener("close", () => d.remove());
    d.showModal(); const first = d.querySelector("button[data-reset]:not([data-reset=''])"); if (first) first.focus();
  }
  // ---- once: «the files stay in the browser» (a reload does not remove them; this says how)
  function remind() {
    try { if (localStorage.getItem("qqq.reset.note")) return; localStorage.setItem("qqq.reset.note", "1"); } catch (e) { return; }
    if (document.getElementById("resetnote")) return;
    const n = document.createElement("div"); n.id = "resetnote"; n.setAttribute("role", "status");
    n.innerHTML = `<span>${I18N.t("reset.note")}</span><button type="button" data-n="reset">${I18N.t("reset.note.btn")}</button><button type="button" data-n="x" aria-label="${I18N.t("reset.note.close")}">×</button>`;
    n.addEventListener("click", e => { const b = /** @type {HTMLElement | null} */ (/** @type {HTMLElement} */ (e.target).closest("button[data-n]")); if (!b) return; if (b.dataset.n === "reset") ask(); n.remove(); });
    document.body.appendChild(n);
  }
  window.QQRESET = { wipe, run, ask, remind, modeOf, last: null };

  // ?reset=1 | all | files: acts at once (the engine of browser.js does not start while this runs)
  const q = modeOf(new URLSearchParams(location.search).get("reset"));
  if (q) { window.QQ_RESETTING = true; run(q); }
})();
