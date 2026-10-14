// @ts-check
// The file list is the only control of what is shown: the box = the file is drawn, a click on the row = the active file (spectrum, scans, arrows),
// Ctrl / Cmd + click = adds or removes the file, a double click on the name = only this file, Shift + click on a box = a range of boxes. (renderFileList in explore.js calls bindListaFile)
const setVisOf = (/** @type {any[]} */ fs, /** @type {boolean} */ on) => {
  fs.forEach(f => sibsOf(f).forEach(g => { g.vis = on; }));
  if (!on) cleanupOrphanPanels(); else if (lrTog()) ensureLayout();
  renderFileList(); redrawAll(); uiSave();
};
function soloFile(/** @type {any} */ f0) {
  if (!f0) return;
  E.files.forEach(f => { f.vis = false; }); sibsOf(f0).forEach(f => { f.vis = true; });
  STATO.file(f0.k, "file"); if (f0.kind !== E.tab) setTab(f0.kind);
  cleanupOrphanPanels(); ensureLayout(); renderFileList(); renderNav(); redrawAll(); uiSave(); scrollToBlock(f0.kind);
}
let _lastBox = /** @type {number | null} */ (null);
function boxRange(/** @type {HTMLInputElement} */ x, /** @type {MouseEvent} */ e) {
  const all = /** @type {HTMLInputElement[]} */ ([...Q("#flst").querySelectorAll(".fl:not(.pr) > input[data-k]")]), i = all.indexOf(x), j = all.findIndex(b => +(b.dataset.k || -1) === _lastBox);
  _lastBox = +(x.dataset.k || -1);
  if (!e.shiftKey || j < 0 || i < 0) return false;
  setVisOf(all.slice(Math.min(i, j), Math.max(i, j) + 1).map(b => E.files[+(b.dataset.k || 0)]), x.checked); return true;
}
function bindListaFile() {
  const L = Q("#flst"), fileOf = (/** @type {EventTarget | null} */ t, /** @type {string} */ sel) => { const nm = /** @type {HTMLElement | null} */ (/** @type {Element} */ (t).closest(sel)); return nm ? E.files[+(nm.dataset.k || 0)] : null; };
  L.ondblclick = e => { const f = fileOf(e.target, ".fl:not(.pr) .nm"); if (f) { if (lrTog()) soloFile(f); else renameFile(f); } };
  L.onclickcapture = e => {          // Ctrl / Cmd + click on a row (not on a box): adds the file to the visible ones or takes it away, the active file does not change
    const row = /** @type {Element} */ (e.target).closest(".fl:not(.pr)");
    if (!(e.ctrlKey || e.metaKey) || !lrTog() || !row || /** @type {Element} */ (e.target).closest("input")) return;
    const f = fileOf(row.querySelector(".nm"), ".nm"); if (f) { e.stopPropagation(); setVisOf([f], !f.vis); }
  };
  L.querySelectorAll(".fl:not(.pr) > input[data-k]").forEach(x => { x.onclick = e => { if (boxRange(/** @type {HTMLInputElement} */ (x), e)) e.stopPropagation(); }; });
}
