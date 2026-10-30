// @ts-check
// Work area of the Dati view: shows one thing, decided from the state alone (no flag): the panels (a file shown or a panel open), the loading
// table (files open, being read or set aside, none shown) or the landing (no file at all); while files are added the landing and the table
// stand in for the panels. Called by applyView and checkGo (shell.js) and by every refit of the panels (syncWorkEmpty, lista-file.js).
"use strict";

const workQ = (/** @type {string} */ s) => /** @type {HTMLElement & { disabled: boolean }} */ (document.querySelector(s));
/** @returns {"opening" | "add" | "panels" | "table" | "landing"} */
function workState() {
  if (S.opening) return "opening";
  if (S.sess && S.adding) return "add";
  const fs = typeof E !== "undefined" ? E.files : [];
  if (S.sess && (fs.some((/** @type {FileAperto} */ f) => f.vis && !f.gone) || E.panels.some((/** @type {Pannello} */ p) => p.el))) return "panels";
  const pend = typeof E !== "undefined" && E.pending && E.pending.length > 0;
  return fs.some((/** @type {FileAperto} */ f) => !f.gone) || pend || ST.files.length || UPBUSY > 0 ? "table" : "landing";
}

// the data buttons of the toolbar wait for an open file (the calculators of the header never do); the file arrows and menu are then renderNav's
const TOOLS_OFF = ["#np-chrom", "#np-spec", "#np-map", "#np-xic", "#np-mrm", "#np-closeall", "#np-method"], NAV_OFF = ["#fprev", "#fsel", "#fnext"];
let _workSig = "";
/** Shows the part of the work area that the state asks for; nothing is written when nothing changed (`force` writes anyway). */
function syncWork(/** @type {boolean} */ force = false) {
  const w = workState(), ws = workQ("#v-workspace"), pend = typeof E !== "undefined" && E.pending && E.pending.length > 0;
  const none = typeof E === "undefined" || !E.files.some((/** @type {FileAperto} */ f) => !f.gone), sig = [w, none, pend, !!S.sess].join();
  if (sig === _workSig && !force) return;
  const was = ws.dataset.work;
  _workSig = sig; ws.dataset.work = w;
  workQ("#wdata").hidden = w === "panels" || w === "opening";
  workQ("#start").hidden = !(w === "landing" || w === "add" || (w === "table" && !S.sess));
  workQ("#wtable").hidden = !(w === "table" || w === "add");
  TOOLS_OFF.forEach(q => { const b = workQ(q); b.disabled = none; b.setAttribute("aria-disabled", String(none)); });
  NAV_OFF.forEach(q => { const b = workQ(q); if (none) b.disabled = true; b.setAttribute("aria-disabled", String(b.disabled)); });
  // the empty side list shows a plain hint (it still takes a drop: the dashed box of #fl-dz appears only while a drag is over it)
  workQ("#fl-hint").hidden = !(none && !pend);
  if (!workQ("#sb-pane").classList.contains("dz-over")) workQ("#fl-dz").hidden = true;
  // the side panel takes the width of the names once a file is shown (barra.js)
  if (was !== w && window.BARRA) BARRA.fitNames();
}

// before a session the side panel takes a drop as the landing does: into the loading table (with a session shell.js adds the files at once)
(() => {
  const dz = workQ("#dfiles"), early = () => S.view === "data" && !S.sess && !S.opening;
  const files = (/** @type {DragEvent} */ e) => [...((e.dataTransfer && e.dataTransfer.types) || [])].includes("Files");
  ["dragenter", "dragover"].forEach(t => dz.addEventListener(t, (/** @type {DragEvent} */ e) => { if (early() && files(e)) { e.preventDefault(); dropZone(true); } }));
  dz.addEventListener("drop", (/** @type {DragEvent} */ e) => {
    if (!early() || !files(e) || !e.dataTransfer) return;
    e.preventDefault(); e.stopPropagation(); dropZone(false);
    if (!CartCarica.fromDrop(e)) upload([...e.dataTransfer.files], "data");
  }, true);
})();
