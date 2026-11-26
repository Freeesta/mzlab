// @ts-check
// Work area of the Dati view, decided from the state: the panels (a file shown or a panel open) or ONE page, the drop area with the sample table under it
// (files being read keep their progress in the side list). That page is the landing (first visit, all files removed, every panel closed) or «+» / «Sample table»
// (S.adding); «Sample table» only scrolls to the table, «Back to work!» (shell.js) leaves the page and gives back the scroll.
// Called by applyView, renderFiles and upload (shell.js) and by every refit of the panels (syncWorkEmpty, lista-file.js).
"use strict";

// Below 900 px the toggle of the side panel (#sb-toggle, barra.js) is the way to the file list: it says «Files (N)», N = rows of the sample table.
// paint() returns true when it wrote that label; false leaves barra.js to write the usual one. The button has no aria-label: its name is whichever of these texts shows (WCAG 2.5.3 Label in Name).
const SHEETLBL = {
  paint(/** @type {HTMLElement} */ tg, /** @type {HTMLElement} */ tl, /** @type {boolean} */ narrow) {
    tg.dataset.lbl = "1";      // from now on .tl holds the label (app.css shows .tln before)
    if (!narrow || S.view !== "data") { delete tg.dataset.sheet; return false; }
    const txt = I18N.t("cmd.panel.files", { n: ST.files.length });
    delete tl.dataset.i18n; tl.textContent = txt; tg.dataset.sheet = "1";
    return true;
  },
  refresh() { const tg = document.getElementById("sb-toggle"), tl = tg && tg.querySelector(".tl"); if (tg && tl) this.paint(tg, /** @type {HTMLElement} */ (tl), window.innerWidth < 900); }
};
const workQ = (/** @type {string} */ s) => /** @type {HTMLElement & { disabled: boolean }} */ (document.querySelector(s));
// every panel was closed with the X of the toolbar: the files stay ticked but the landing shows until a panel or a file is opened again
let closedAll = false;
/** @returns {"opening" | "add" | "panels" | "landing"} */
function workState() {
  if (S.opening) return "opening";
  if (S.sess && S.adding) return "add";
  const fs = typeof E !== "undefined" ? E.files : [];
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
  SHEETLBL.refresh();
  const w = workState(), ws = workQ("#v-workspace"), up = UPBUSY > 0 || OPENNOW.busy() > 0;
  const none = typeof E === "undefined" || !E.files.some((/** @type {FileAperto} */ f) => !f.gone), tab = up || !none, sig = [w, none, up, !!S.sess, tab, workRows(typeof E !== "undefined" ? E.files : [])].join();
  // the card of the manual integrations follows the state written here (integrazioni-manuali.js); the provisional TIC of a first loading too (tic-provvisorio.js)
  const simtab = () => { /** @type {any} */ (window).MANINT?.sync(); /** @type {any} */ (window).TICPROV?.sync(); };
  if (sig === _workSig && !force) { simtab(); return; }
  const was = ws.dataset.work;
  _workSig = sig; ws.dataset.work = w;
  // the landing forgets «Sample table»; once empty it waits again for the first file to be shown
  const rows = workRows(typeof E !== "undefined" ? E.files : []), land = w === "landing" && rows;
  workQ("#wdata").hidden = w === "panels" || w === "opening";
  // «Back to work!» waits for a file to go back to, and for the reading to end (it would push the drop area down while the files load)
  workQ("#backwork").hidden = !(S.sess && !none && !up && (w === "add" || w === "landing"));
  workQ("#start").hidden = !(w === "landing" || w === "add");
  workQ("#wtable").hidden = !(w === "add" || land);
  // never inert, never a toggle: it shows whenever the table has rows and only brings the table into view (see below)
  const sb = workQ("#samptab"); sb.hidden = !tab; sb.removeAttribute("aria-pressed"); sb.disabled = false;
  // the add buttons scroll sideways in a narrow window: with every button disabled the strip has no stop of its own, so the keyboard reaches it directly (axe scrollable-region-focusable)
  const ga = workQ("#g-add"); if (none) ga.tabIndex = 0; else ga.removeAttribute("tabindex");
  TOOLS_OFF.forEach(q => { const b = workQ(q); b.disabled = none; b.setAttribute("aria-disabled", String(none)); });
  NAV_OFF.forEach(q => { const b = workQ(q); if (none) b.disabled = true; b.setAttribute("aria-disabled", String(b.disabled)); });
  // the empty side list shows a plain hint (it still takes a drop: the dashed box of #fl-dz appears only while a drag is over it)
  workQ("#fl-hint").hidden = !(none && !up);
  if (!workQ("#sb-pane").classList.contains("dz-over")) workQ("#fl-dz").hidden = true;
  // the content fades in once in place of the landing (opacity only, transizioni.css)
  // the class leaves when the fade ends (a long task on a slow machine must not remove it before the first frame, or the fade never starts); the timer is only the fallback
  if ((was === "landing" || was === "add") && w === "panels") {
    const v = workQ("#v-data"), end = (/** @type {AnimationEvent} */ e) => { if (e.animationName === "tr-fade") done(); }, done = () => { v.classList.remove("opn-rev"); v.removeEventListener("animationend", end); };
    v.classList.add("opn-rev"); v.addEventListener("animationend", end); setTimeout(done, 2000);
  }
  // the side panel takes the width of the names once a file is shown (barra.js)
  if (was !== w && window.BARRA) BARRA.fitNames();
  simtab();
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
// «Sample table» of the side panel: the drop area and the table are one page. From the panels the button opens that page (the scroll of the panels is kept for «Back to work!»);
// on the page it only scrolls the table under the header (smooth, instant with reduced motion) and puts the focus on its heading: nothing is re-rendered or switched.
function tableToView() {
  const t = workQ("#wtable"), h = /** @type {HTMLElement | null} */ (t.querySelector(".samphead b")) || t;
  if (S.sess && workState() === "panels") {
    STATO.aggiunta(true, "sample-table");
    if (window.innerWidth < 900 && window.BARRA) BARRA.close(false);
    applyView(); window.scrollTo(0, 0);
  }
  h.tabIndex = -1;
  t.scrollIntoView({ behavior: FRAME.behavior(), block: "start" });
  h.focus({ preventScroll: true });
}
workQ("#samptab").onclick = tableToView;
// a click on a file (side list or name in the table) leaves the one page for the panels: the file's own handler shows it, the state follows
const leavePage = (/** @type {MouseEvent} */ e) => {
  if (!S.adding || !S.sess || !/** @type {HTMLElement} */ (e.target).closest(".fl:not(.up):not(.pr), [data-open]")) return;
  STATO.aggiunta(false, "open-file"); queueMicrotask(() => applyView());
};
workQ("#flst").addEventListener("click", leavePage, true);
workQ("#flist").addEventListener("click", leavePage, true);

// the provisional TIC (tic-provvisorio.js) is loaded at the first scan table with its TIC, not at start-up; the events of the wait are handed over
{ /** @type {any[] | null} */ let q = null;
  addEventListener("mzlab-index", (/** @type {any} */ e) => { const d = e && e.detail, W = /** @type {any} */ (window); if (W.TICPROV || !(d && (q || d.early_tic))) return;
    if (q) { q.push(d); return; }
    q = [d]; loadScript("m:tic-provvisorio.js").then(() => { W.TICPROV.feed(q); q = null; }, () => { q = null; }); }); }
