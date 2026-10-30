// @ts-check
// Work area of the Dati view, decided from the state: the panels (a file shown or a panel open), the sample table («Sample table» of the side panel pressed:
// S.table) or the landing: the drop area, with the sample table under it once it has rows (files being read keep their progress in the side list). The landing
// is the only empty state: first visit, all files removed, every panel closed, «+» (S.adding); «Back to work!» (shell.js) leaves it.
// Called by applyView, renderFiles and upload (shell.js) and by every refit of the panels (syncWorkEmpty, lista-file.js).
"use strict";

const workQ = (/** @type {string} */ s) => /** @type {HTMLElement & { disabled: boolean }} */ (document.querySelector(s));
// every panel was closed with the X of the toolbar: the files stay ticked but the landing shows until a panel or a file is opened again
let closedAll = false;
/** @returns {"opening" | "add" | "panels" | "table" | "landing"} */
function workState() {
  if (S.opening) return "opening";
  if (S.sess && S.adding) return "add";
  const fs = typeof E !== "undefined" ? E.files : [], left = workRows(fs);
  if (S.sess && S.table && left) return "table";
  const built = E.panels.some((/** @type {Pannello} */ p) => p.el);
  if (built) closedAll = false;
  if (S.sess && (built || (!closedAll && fs.some((/** @type {FileAperto} */ f) => f.vis && !f.gone)))) return "panels";
  return "landing";
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
  const none = typeof E === "undefined" || !E.files.some((/** @type {FileAperto} */ f) => !f.gone), tab = w !== "add" && (up || !none), sig = [w, none, up, !!S.sess, tab, workRows(typeof E !== "undefined" ? E.files : [])].join();
  if (sig === _workSig && !force) return;
  const was = ws.dataset.work;
  _workSig = sig; ws.dataset.work = w;
  // the landing forgets «Sample table»; once empty it waits again for the first file to be shown
  const rows = workRows(typeof E !== "undefined" ? E.files : []), land = w === "landing" && rows;
  if (w === "landing") S.table = false;
  workQ("#wdata").hidden = w === "panels" || w === "opening";
  // «Back to work!» waits for a file to go back to
  workQ("#backwork").hidden = !(S.sess && !none && (w === "add" || w === "landing"));
  workQ("#start").hidden = !(w === "landing" || w === "add" || (w === "table" && !S.sess));
  workQ("#wtable").hidden = !(w === "table" || w === "add" || land);
  // on the landing the table is already in view under the drop area: the button stays in place, pressed and inert
  const sb = workQ("#samptab"); sb.hidden = !tab; sb.setAttribute("aria-pressed", String(w === "table" || land)); sb.disabled = land;
  TOOLS_OFF.forEach(q => { const b = workQ(q); b.disabled = none; b.setAttribute("aria-disabled", String(none)); });
  NAV_OFF.forEach(q => { const b = workQ(q); if (none) b.disabled = true; b.setAttribute("aria-disabled", String(b.disabled)); });
  // the empty side list shows a plain hint (it still takes a drop: the dashed box of #fl-dz appears only while a drag is over it)
  workQ("#fl-hint").hidden = !(none && !up);
  if (!workQ("#sb-pane").classList.contains("dz-over")) workQ("#fl-dz").hidden = true;
  // the content fades in once in place of the landing (opacity only, transizioni.css)
  if (was === "landing" && w === "panels") { const v = workQ("#v-data"); v.classList.add("opn-rev"); setTimeout(() => v.classList.remove("opn-rev"), 400); }
  // the side panel takes the width of the names once a file is shown (barra.js)
  if (was !== w && window.BARRA) BARRA.fitNames();
}

/** The X of the toolbar closed every panel (pannelli.js): the landing takes the work area; `false` gives it back to the panels. */
function workClosedAll(/** @type {boolean} */ on = true) { closedAll = on; syncWork(); }
// a click on a file row after the X: the file's panel comes back (a click on its tick box goes through setVisOf, which builds the layout)
workQ("#flst").addEventListener("click", (/** @type {MouseEvent} */ e) => {
  const t = /** @type {HTMLElement} */ (e.target), nm = closedAll && t.closest(".fl:not(.up):not(.pr)") ? t.closest(".fl")?.querySelector(".nm") : null;
  if (!nm) return;
  closedAll = false;
  if (!t.closest("input")) queueMicrotask(() => { if (!E.panels.some((/** @type {Pannello} */ p) => p.el)) soloFile(E.files[+((/** @type {HTMLElement} */ (nm)).dataset.k || 0)]); });
}, true);
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
