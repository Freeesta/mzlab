// @ts-check
// The file list is the only control of what is shown: the box = the file is drawn, a click on the row = the active file (spectrum, scans, arrows),
// Ctrl / Cmd + click = adds or removes the file, a double click on the name = only this file, Shift + click on a box = a range of boxes. (renderFileList in explore.js calls bindListaFile)
const setVisOf = (/** @type {any[]} */ fs, /** @type {boolean} */ on) => {
  fs.forEach(f => sibsOf(f).forEach(g => { g.vis = on; }));
  if (!on) cleanupOrphanPanels(); else if (lrTog()) ensureLayout();
  renderFileList(); renderNav(); redrawAll(); uiSave();
};
function soloFile(/** @type {any} */ f0) {
  if (!f0) return;
  E.files.forEach(f => { f.vis = false; }); sibsOf(f0).forEach(f => { f.vis = true; });
  STATO.file(f0.k, "file"); if (f0.kind !== E.tab) setTab(f0.kind);
  cleanupOrphanPanels(); ensureLayout(); renderFileList(); renderNav(); redrawAll(); uiSave(); scrollToBlock(f0.kind);
}
// The three steps of taking one file out: take (hide it), back (undo) and free (release its copy in OPFS and its indexes, only in the browser, when the notice closes without an undo).
function fileRemoval(/** @type {any} */ f) {
  const was = { vis: f.vis, cur: E.cur, ks: E.panels.filter(q => q.k === f.k && q.type !== "xic").map(q => /** @type {[any, number]} */ ([q, q.k])) };
  const src = (/** @type {any} */ g) => (g.file || "").split("#")[0];
  const show = () => {
    paintFiles(); renderFileList(); renderNav();
    // @ts-ignore toolbar() is also window.toolbar (a BarProp) for lib.dom
    toolbar(); E.panels.forEach(ctl); redrawAll(); uiSave();
  };
  const take = () => {
    f.gone = true; f.vis = false; if (window.MAPPA) MAPPA.dropFile(f.k);
    E.panels.filter(q => q.k === f.k && q.type !== "xic").forEach(q => { const o = tabFiles(q.tab).find(x => x.k !== f.k); if (o) q.k = o.k; });
    if (E.cur === f.k) STATO.file((tabFiles()[0] || { k: 0 }).k, "file-removed");
    show();
  };
  const back = () => {
    f.gone = false; f.vis = was.vis; was.ks.forEach(([q, k]) => { if (E.panels.includes(q)) q.k = k; });
    if (was.cur === f.k) STATO.file(f.k, "file-restored");
    show();
  };
  const free = () => {
    if (window.MZLAB_BROWSER && f.gone && !E.files.some(g => g !== f && !g.gone && src(g) === src(f))) post("api/remove", { name: src(f) }).catch(() => { /* not in the browser store: nothing to free */ });
  };
  return { take, back, free };
}
/** Removes a file from the session; no question: «Removed · Undo» takes it back (the stack of storia.js). In the browser the copy in OPFS and its derived indexes are freed when the notice has closed without an undo (a shared source, like a .wiff with several samples, stays while another sample uses it). */
function removeFile(/** @type {any} */ f) {
  const r = fileRemoval(f); r.take();
  if (STORIA) STORIA.tolto({ what: f.label, tab: f.kind, dati: f.label, undo: r.back, redo: r.take, scade: window.MZLAB_BROWSER ? r.free : null });
  else r.free();
}
/** Removes every file of the list with ONE «Removed · Undo» entry that takes them all back (same mechanism as removeFile). */
function removeAllFiles() {
  const rs = E.files.filter(f => !f.gone).map(fileRemoval); if (!rs.length) return;
  rs.forEach(r => r.take());
  // removed from a hidden side bar (palette, shortcut): bring it back so the notice and the empty list are in view
  if (window.BARRA && BARRA.isDataView() && !BARRA.open) BARRA.show("file");
  const all = (/** @type {"take" | "back"} */ k) => () => (k === "back" ? [...rs].reverse() : rs).forEach(r => r[k]());
  if (STORIA) STORIA.tolto({ what: I18N.t("files.removeAll.what", { n: rs.length }), dati: String(rs.length), undo: all("back"), redo: all("take"), scade: window.MZLAB_BROWSER ? () => rs.forEach(r => r.free()) : null });
  else rs.forEach(r => r.free());
}
let _lastBox = /** @type {number | null} */ (null);
function boxRange(/** @type {HTMLInputElement} */ x, /** @type {MouseEvent} */ e) {
  const all = /** @type {HTMLInputElement[]} */ ([...Q("#flst").querySelectorAll(".fl:not(.pr) > input[data-k]")]), i = all.indexOf(x), j = all.findIndex(b => +(b.dataset.k || -1) === _lastBox);
  _lastBox = +(x.dataset.k || -1);
  if (!e.shiftKey || j < 0 || i < 0) return false;
  setVisOf(all.slice(Math.min(i, j), Math.max(i, j) + 1).map(b => E.files[+(b.dataset.k || 0)]), x.checked); return true;
}
function bindListaFile() {
  /** @type {HTMLButtonElement} */ (Q("#rmall")).disabled = !E.files.some(f => !f.gone);
  const L = Q("#flst"), fileOf = (/** @type {EventTarget | null} */ t, /** @type {string} */ sel) => { const nm = /** @type {HTMLElement | null} */ (/** @type {Element} */ (t).closest(sel)); return nm ? E.files[+(nm.dataset.k || 0)] : null; };
  L.ondblclick = e => {
    // the colour square: the first click of a file that is not the active one rebuilds the list, so the handler lives here and not on the square
    const sw = /** @type {Element} */ (e.target).closest(".fl:not(.pr) > i");
    if (sw) { e.stopPropagation(); const g = fileOf(sw.parentElement?.querySelector(".nm"), ".nm"); if (g) pickFileColor(g); return; }
    const f = fileOf(e.target, ".fl:not(.pr) .nm"); if (f) { if (lrTog()) soloFile(f); else renameFile(f); } };
  // Ctrl / Cmd + click on a row (not on a box): adds the file to the visible ones or takes it away, the active file does not change
  if (!L.dataset.cap) L.addEventListener("click", e => {
    const row = /** @type {Element} */ (e.target).closest(".fl:not(.pr)");
    if (!(e.ctrlKey || e.metaKey) || !lrTog() || !row || /** @type {Element} */ (e.target).closest("input")) return;
    const f = fileOf(row.querySelector(".nm"), ".nm"); if (f) { e.stopPropagation(); setVisOf([f], !f.vis); }
  }, true);
  L.dataset.cap = "1";
  L.querySelectorAll(".fl:not(.pr) > input[data-k]").forEach(x => { x.onclick = e => { if (boxRange(/** @type {HTMLInputElement} */ (x), e)) e.stopPropagation(); }; });
}

// The file list is the only control of what is shown: E.browse is not a mode the student sets but follows from it. With ONE file ticked in the tab the graphs follow it
// (and it is the active file); with several they are overlaid and the active file (E.cur) drives the spectrum, the scans and the arrows; E.cur is always one of the ticked files.
function syncBrowse() {
  const tf = tabFiles(), v = tf.filter(f => f.vis);
  E.browse = v.length === 1 || tf.length === 1;
  const act = v.length === 1 ? v[0] : tf.length === 1 ? tf[0] : v.find(f => f.k === E.cur) || v[0];
  if (act && act.k !== E.cur) STATO.file(act.k, "browse");
  paintScanOf();
}
/** In the header of a chromatogram that drives a spectrum: whose scans are shown, when several files are overlaid (the active one). */
function paintScanOf() {
  E.panels.forEach(q => {
    if (!q.el) return;
    q.el.querySelectorAll(".pscan").forEach(x => x.remove());
    const m = /** @type {any} */ (q)._mate, f = E.files[E.cur];
    if (q.type !== "chrom" || !m || m.type !== "spec" || !f || tabFiles(q.tab || E.tab).filter(g => g.vis).length < 2) return;
    const t = document.createElement("span"); t.className = "pscan"; t.textContent = I18N.t("panel.scanOf", { f: f.label });
    (q.el.querySelector(".ppair") || q.el.querySelector(".pnum") || q.el.firstChild)?.after(t);
  });
}
/** A click on a trace of overlaid chromatograms makes that file the active one (the spectrum and the scans are then its own). */
function pickTrace(/** @type {Pannello} */ p, /** @type {number} */ x, /** @type {number} */ py) {
  const a = p._a; if (!a || !a.sr || a.sr.length < 2 || a.cas || a.stk) return;
  let best = null, bd = 14;
  a.sr.forEach(r => { const f = E.files[r.k]; if (!f || !scanFiles([f]).length || !f.vis) return; const d = Math.abs(a.Y(r.ys[nearIdx(r.x, x)]) - py); if (d < bd) { bd = d; best = r.k; } });
  if (best != null && best !== E.cur) { E.panels.forEach(s => { if (s.type === "spec" && s.link === p.id) s.kSet = false; }); STATO.file(best, "file"); renderFileList(); renderNav(); uiSave(); }
}
/** Makes file k the active one (ticked if it was not); with one file shown, the new one replaces it (the arrows and the chooser of the bar). */
function activeFile(k) {
  const f = E.files[k]; if (!f) return;
  const alone = E.browse; if (alone) tabFiles(f.kind).forEach(g => { g.vis = false; });
  sibsOf(f).forEach(g => { g.vis = true; }); STATO.file(f.k, "file");
  if (f.kind !== E.tab) setTab(f.kind);
  cleanupOrphanPanels(); ensureLayout(); renderFileList(); renderNav(); redrawAll(); uiSave();
}
/** ‹ ›: the next file of the tab, among the ticked ones when several are shown. */
function goFile(d) {
  const tf = tabFiles(), v = tf.filter(f => f.vis), ring = v.length > 1 ? v : tf; if (tf.length < 2) return;
  activeFile(ring[(Math.max(0, ring.findIndex(f => f.k === E.cur)) + d + ring.length) % ring.length].k);
}
/** Old sessions saved the mode «only the selected»: the same view is one ticked file per type, the one that was current. */
function migrateBrowse(/** @type {any} */ u) {
  if (!u.browse) return;
  const keep = new Set(Object.values(E.curBy || {}).concat(u.cur || 0));
  E.files.forEach(f => { if (f.vis && !keep.has(f.k) && E.files.some(g => g.kind === f.kind && keep.has(g.k) && g.vis)) f.vis = false; });
}
