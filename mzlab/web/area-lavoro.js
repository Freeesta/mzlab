// @ts-check
// Work area of the Dati view: shows one thing, decided from the state: the panels (a file shown or a panel open), the sample table (files open
// or set aside, none shown, or «Sample table» of the side panel pressed: S.table) or the landing (no file, or files still being read: their
// progress is the side list's, the landing does not move). The landing is the only empty state: first visit, all files removed, «+».
// Called by applyView, renderFiles and upload (shell.js) and by every refit of the panels (syncWorkEmpty, lista-file.js).
"use strict";

const workQ = (/** @type {string} */ s) => /** @type {HTMLElement & { disabled: boolean }} */ (document.querySelector(s));
/** @returns {"opening" | "add" | "panels" | "table" | "landing"} */
function workState() {
  if (S.opening) return "opening";
  if (S.sess && S.adding) return "add";
  const fs = typeof E !== "undefined" ? E.files : [], left = workRows(fs);
  if (S.sess && S.table && left) return "table";
  if (S.sess && (fs.some((/** @type {FileAperto} */ f) => f.vis && !f.gone) || E.panels.some((/** @type {Pannello} */ p) => p.el))) return "panels";
  if (UPBUSY > 0 || OPENNOW.busy() > 0) return "landing";
  return fs.some((/** @type {FileAperto} */ f) => !f.gone) || left ? "table" : "landing";
}
// rows of the sample table that are not removed files (a removed file keeps its row unticked until its notice closes: all removed = the landing)
function workRows(/** @type {FileAperto[]} */ fs) {
  const src = (/** @type {FileAperto} */ f) => String(f.file).split("#")[0];
  const gone = new Set(fs.filter(f => f.gone).map(src)), open = new Set(fs.filter(f => !f.gone).map(src));
  return ST.files.some((/** @type {FileRiga} */ r) => open.has(r.name) || !gone.has(r.name));
}

// the data buttons of the toolbar wait for an open file (the calculators of the header never do); the file arrows and menu are then renderNav's
const TOOLS_OFF = ["#np-chrom", "#np-spec", "#np-map", "#np-xic", "#np-mrm", "#np-closeall", "#np-method"], NAV_OFF = ["#fprev", "#fsel", "#fnext"];
let _workSig = "";
/** Shows the part of the work area that the state asks for; nothing is written when nothing changed (`force` writes anyway). */
function syncWork(/** @type {boolean} */ force = false) {
  const w = workState(), ws = workQ("#v-workspace"), up = UPBUSY > 0 || OPENNOW.busy() > 0;
  const none = typeof E === "undefined" || !E.files.some((/** @type {FileAperto} */ f) => !f.gone), tab = w !== "add" && (up || !none), sig = [w, none, up, !!S.sess, tab].join();
  if (sig === _workSig && !force) return;
  const was = ws.dataset.work;
  _workSig = sig; ws.dataset.work = w;
  // the empty landing forgets «Sample table»: the next loading opens its first file again
  if (w === "landing") S.table = false;
  workQ("#wdata").hidden = w === "panels" || w === "opening";
  workQ("#start").hidden = !(w === "landing" || w === "add" || (w === "table" && !S.sess));
  workQ("#wtable").hidden = !(w === "table" || w === "add");
  const sb = workQ("#samptab"); sb.hidden = !tab; sb.setAttribute("aria-pressed", String(w === "table"));
  TOOLS_OFF.forEach(q => { const b = workQ(q); b.disabled = none; b.setAttribute("aria-disabled", String(none)); });
  NAV_OFF.forEach(q => { const b = workQ(q); if (none) b.disabled = true; b.setAttribute("aria-disabled", String(b.disabled)); });
  // the empty side list shows a plain hint (it still takes a drop: the dashed box of #fl-dz appears only while a drag is over it)
  workQ("#fl-hint").hidden = !(none && !up);
  if (!workQ("#sb-pane").classList.contains("dz-over")) workQ("#fl-dz").hidden = true;
  // the side panel takes the width of the names once a file is shown (barra.js)
  if (was !== w && window.BARRA) BARRA.fitNames();
}

// «Sample table» of the side panel: the table in place of the panels and back; a click on a file of the side list goes back to the panels
workQ("#samptab").onclick = () => { S.table = !S.table; syncWork(); };
workQ("#flst").addEventListener("click", (/** @type {MouseEvent} */ e) => {
  const t = /** @type {HTMLElement} */ (e.target);
  if (S.table && t.closest(".fl:not(.up)")) { S.table = false; queueMicrotask(() => syncWork()); }
}, true);

// before a session the side panel takes a drop as the landing does (with a session shell.js takes it): the files open as soon as they are read
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
