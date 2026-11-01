// @ts-strict
// The file list is the only control of what is shown: the box = the file is drawn, a click on the row = the active file (spectrum, scans, arrows),
// Ctrl / Cmd + click = adds or removes the file, a double click on the name = only this file, Shift + click on a box = a range of boxes. (renderFileList in explore.js calls bindListaFile)
// name, colour, group, visibility and order of the files are one undoable action (comandi.js): the snapshot is the list of all the files
COMANDI.gestore("files.view", {
  // the panels are part of the snapshot: hiding a file closes the panels that only showed it, and the undo opens them again
  leggi: () => JSON.stringify({ f: E.files.map((/** @type {any} */ f) => [f.file, !!f.vis, f.label, f.color, !!f.colorSet, f.ugrp ?? null]), u: [E.ugBy ?? null, E.ugOrder ?? null, window.GRUPPI?.stg() ?? null],
    p: E.panels.map((/** @type {any} */ q) => [q.id, panelRec(q), { link: q.link, src: q.src, dda: q.dda, dock: q.dock, duo: q.duo }]) }),
  applica: (_p, snap) => {
    const { f: rows, p: panels, u } = JSON.parse(snap); [E.ugBy, E.ugOrder] = u || []; window.GRUPPI?.stg(u?.[2] ?? null);
    rows.forEach((/** @type {any[]} */ [file, vis, label, color, colorSet, ugrp]) => { const f = E.files.find((/** @type {any} */ x) => x.file === file); if (f) Object.assign(f, { vis, label, color, colorSet, ugrp: ugrp || undefined }); });
    rows.map((/** @type {any[]} */ r) => r[0]).filter((/** @type {string} */ n) => E.files.some((/** @type {any} */ f) => f.file === n)).forEach((/** @type {string} */ n, /** @type {number} */ i) => {      // the order back, one move at a time (reorderFiles renumbers the panels)
      if (E.files[i].file !== n) { const j = E.files.findIndex((/** @type {any} */ f) => f.file === n); reorderFiles(E.files[j].k, E.files[i].k, true); }
    });
    cleanupOrphanPanels();
    const made = /** @type {Map<number, any>} */ (new Map());
    panels.forEach((/** @type {any[]} */ [id, rec]) => {
      const live = E.panels.find((/** @type {any} */ q) => q.id === id);
      if (!live) { made.set(id, riapriPannello(rec)); return; }
      const k = E.files.findIndex((/** @type {any} */ f) => f.file === rec.k); if (rec.k && k >= 0 && live.k !== k) live.k = k;      // a spectrum that moved to another file comes back
    });
    const now = (/** @type {number | null} */ id) => id == null ? null : made.has(id) ? made.get(id).id : E.panels.some((/** @type {any} */ q) => q.id === id) ? id : null;
    made.forEach((q, id) => { const rel = panels.find((/** @type {any[]} */ x) => x[0] === id)[2]; for (const k of /** @type {const} */ (["link", "src", "dda", "dock", "duo"])) q[k] = now(rel[k]); });
    relayout(); fitHost(); renderFileList(); renderNav(); redrawAll();
  },
});
/** @param {() => void} fn */
const fileViewDo = fn => COMANDI.fai("files.view", COMANDI.GLOBALE, fn);
const setVisOf = (/** @type {any[]} */ fs, /** @type {boolean} */ on) => {
  fileViewDo(() => fs.forEach(f => sibsOf(f).forEach(g => { g.vis = on; })));
  if (!on) cleanupOrphanPanels(); else if (lrTog()) ensureLayout();
  renderFileList(); renderNav(); redrawAll(); uiSave();
};
function soloFile(/** @type {any} */ f0) {
  if (!f0) return;
  fileViewDo(() => { E.files.forEach(f => { f.vis = false; }); sibsOf(f0).forEach(f => { f.vis = true; }); });
  // the panels area shows before the layout is built, so the panels take its real width (not the estimate of hostWidth)
  syncWork();
  STATO.file(f0.k, "file"); if (f0.kind !== E.tab) setTab(f0.kind, false, "file");
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
  /** @type {any[]} */ let closed = [];
  const take = () => {
    const before = E.panels.map(q => /** @type {[any, any]} */ ([q, panelRec(q)]));
    f.gone = true; f.vis = false; if (window.MAPPA) MAPPA.dropFile(f.k);
    E.panels.filter(q => q.k === f.k && q.type !== "xic").forEach(q => { const o = tabFiles(q.tab).find(x => x.k !== f.k); if (o) q.k = o.k; });
    if (E.cur === f.k) STATO.file((tabFiles()[0] || { k: 0 }).k, "file-removed");
    // a panel left with no file of its tab closes (the undo opens it again)
    cleanupOrphanPanels();
    closed = before.filter(([q]) => !E.panels.includes(q)).map(([, rec]) => rec);
    if (closed.length) { relayout(); fitHost(); }
    show();
  };
  const back = () => {
    f.gone = false; f.vis = was.vis; was.ks.forEach(([q, k]) => { if (E.panels.includes(q)) q.k = k; });
    if (was.cur === f.k) STATO.file(f.k, "file-restored");
    closed.forEach(rec => riapriPannello(rec)); if (closed.length) { relayout(); fitHost(); }
    closed = [];
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
/** Removes every file of the list (or of `fs`, a user group named `what`) with ONE «Removed · Undo» entry that takes them all back (same mechanism as removeFile). */
function removeAllFiles(fs = E.files.filter(f => !f.gone), /** @type {string | null} */ what = null) {
  const rs = fs.map(fileRemoval); if (!rs.length) return;
  rs.forEach(r => r.take());
  // removed from a hidden side bar (palette, shortcut): bring it back so the notice and the empty list are in view
  if (window.BARRA && BARRA.isDataView() && !BARRA.open) BARRA.show("file");
  const all = (/** @type {"take" | "back"} */ k) => () => (k === "back" ? [...rs].reverse() : rs).forEach(r => r[k]());
  if (STORIA) STORIA.tolto({ what: what ?? I18N.t("files.removeAll.what", { n: rs.length }), dati: String(rs.length), undo: all("back"), redo: all("take"), scade: window.MZLAB_BROWSER ? () => rs.forEach(r => r.free()) : null });
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
  const L = Q("#flst"), fileOf = (/** @type {EventTarget | null} */ t, /** @type {string} */ sel) => { const nm = /** @type {HTMLElement | null} */ (/** @type {Element} */ (t).closest(sel)); return nm ? E.files[+(nm.dataset.k || 0)] : null; };
  // the × of a row: removes the file (Undo within 6 s)
  if (!L.dataset.cap) L.addEventListener("click", e => {
    const x = /** @type {HTMLElement | null} */ (/** @type {Element} */ (e.target).closest(".fl-x")); if (!x) return;
    e.stopPropagation(); e.preventDefault(); const g = E.files[+(x.dataset.rm || -1)]; if (g && !g.gone) removeFile(g);
  }, true);
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
  // the keyboard way to the right-click menus of a row or of a user group header (WCAG 2.1.1); F2 renames a user group
  if (!L.dataset.cap) L.addEventListener("keydown", e => {
    const t = /** @type {HTMLElement} */ (e.target), hd = /** @type {HTMLElement | null} */ (t.closest(".fgh[data-ug]")), row = t.closest(".fl:not(.pr)");
    if (!(e.key === "ContextMenu" || e.key === "F10" && e.shiftKey || e.key === "F2" && hd) || !(hd || row)) return;
    e.preventDefault(); e.stopPropagation();
    if (hd && e.key === "F2") { renameGroup(hd.dataset.ug || ""); return; }
    const r = t.getBoundingClientRect(), ev = { preventDefault() { /* already done */ }, stopPropagation() { /* already done */ }, clientX: r.left, clientY: r.bottom, kbd: true };
    if (hd) groupCtx(ev, hd.dataset.ug || ""); else fileCtx(ev, fileOf(row?.querySelector(".nm") || null, ".nm"));
  });
  // Ctrl+Shift+↑/↓ on a row moves its file (and the ticked ones) to the previous / next group (WCAG 2.5.7: the keyboard way of the drag)
  if (!L.dataset.cap) L.addEventListener("keydown", e => {
    const row = /** @type {HTMLElement} */ (e.target).closest(".fl:not(.pr)");
    if (!row || !(e.ctrlKey || e.metaKey) || !e.shiftKey || (e.key !== "ArrowUp" && e.key !== "ArrowDown")) return;
    e.preventDefault(); e.stopPropagation(); carica("gruppi-dnd.js").then(() => GDND.step(row, e.key === "ArrowDown" ? 1 : -1));
  });
  L.dataset.cap = "1";
  const by = document.getElementById("sb-by"); if (by && !by.onclick) by.onclick = () => carica("gruppi.js").then(() => carica("gruppi-per.js")).then(() => GRUPPIPER.open());
  // a user group header: its menu; a double click renames it (the first click folds and redraws the list, so the second click, not dblclick, carries it)
  L.querySelectorAll(".fgh[data-ug]").forEach(x => {
    const el = /** @type {HTMLElement} */ (x), fold = el.onclick, n = el.dataset.ug || "";
    el.oncontextmenu = e => groupCtx(e, n);
    el.onclick = e => { fold?.call(el, e); if (e.detail === 2 && !/** @type {Element} */ (e.target).closest("input")) renameGroup(n); };
  });
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
  sibsOf(f).forEach(g => { g.vis = true; }); STATO.file(f.k, "file"); syncWork();
  followSpectra(f);
  if (f.kind !== E.tab) setTab(f.kind, false, "file");
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

// user groups (both worlds): «Group files» puts the ticked files into a new «Group N» at once, renamed later from its header; a file is in one user group
// at most and the group takes the place of its automatic header (Full Scan, MS2...). The group is the field `ugrp` of the files (session, undo of files.view).
// the files of the side list: the bench lists only the experiments with spectra (its MRM files have their own list)
const flSide = () => E.files.filter(x => !x.gone && (lrTog() || (ESP.voce(x.kind) || { spettri: true }).spettri));
const ugFiles = (/** @type {string} */ n) => E.files.filter(x => !x.gone && x.ugrp === n);
const ugSet = (/** @type {any[]} */ fs, /** @type {string} */ v) => { fileViewDo(() => { fs.forEach(x => { x.ugrp = v || undefined; }); E.ugBy = { by: "custom" }; }); renderFileList(); uiSave(); };
// the physical files ticked in the list (a DDA file brings both its parts)
const ugBases = () => new Set(flSide().filter(x => x.vis).map(x => fileBase(x.file)));
function groupFiles() {
  const used = new Set(E.files.map(x => x.ugrp)); let n = 1;
  while (used.has(I18N.t("files.ugrp.name", { n }))) n++;
  carica("gruppi.js").then(() => GRUPPI.set([...ugBases()], I18N.t("files.ugrp.name", { n }), "new"));
}
/** True when the list has user groups and every one is a single file: their headers (a count of 1 each) are left out. @param {{ g: string, fs: any[] }[]} groups */
const ugFlat = groups => (E.ugBy && E.ugBy.by === "flat") || groups.some(G => G.g.startsWith("u:")) && groups.every(G => !G.g.startsWith("u:") || G.fs.length < 2);
// experiment types in the catalogue order; «None» = one headerless group
function ugArrange(/** @type {{ g: string, fs: any[] }[]} */ groups) {
  const at = (/** @type {{ g: string, fs: any[] }} */ G) => (ESP.voce(G.g.startsWith("hr.") ? G.g.slice(3) : G.fs[0].kind) || { ordine: 99 }).ordine, slots = groups.map((G, j) => G.g[0] == "u" ? -1 : j).filter(j => j >= 0), ts = slots.map(j => groups[j]).sort((a, b) => at(a) - at(b));
  slots.forEach((j, i) => { groups[j] = ts[i]; });
  if (E.ugBy && E.ugBy.by === "flat") groups.splice(0, groups.length, { g: "u:", fs: groups.flatMap(G => G.fs) });
}
/** «Group by» is shown only while some grouping gives a group of 2+ files (one the list already has, or a variable of the names); the chosen key stays and applies again when it is useful. */
let _byTick = 0;
function syncGroupBy() {
  const by = document.getElementById("sb-by"); if (!by) return;
  const my = ++_byTick, show = (/** @type {boolean} */ on) => { if (my !== _byTick || by.hidden === !on) return; by.hidden = !on; if (!on) window.GRUPPIPER?.close(false); };
  if (new Set(flSide().map((/** @type {any} */ f) => fileBase(f.file))).size < 2) return show(false);
  carica("gruppi.js").then(() => GRUPPI.useful()).then(show);
}
// rename and ungroup also reach the labels of the loading table's rows: gruppi.js writes both
const renameGroup = (/** @type {string} */ n) => carica("gruppi.js").then(() => GRUPPI.rename(n));
function groupCtx(/** @type {any} */ e, /** @type {string} */ n) {
  e.stopPropagation();
  menu(e, [{ label: I18N.t("files.ctx.rename"), fn: () => renameGroup(n) }, { label: I18N.t("files.ctx.ungroup"), fn: () => carica("gruppi.js").then(() => GRUPPI.ungroup(n)) }, "-", { label: I18N.t("files.ctx.removeGroup"), fn: () => removeAllFiles(ugFiles(n), n) }]);
}
/** Header of a user group: the markup of the automatic headers of renderFileList, plus the name for its menu (right click, Menu key, Shift+F10) and rename (double click, F2). */
function ugHead(/** @type {{ g: string, fs: any[] }} */ G, /** @type {boolean} */ open) {
  const n = G.g.slice(2);
  return `<div class="fgh" data-gt="${EH(G.g)}" data-ug="${EH(n)}" style="cursor:pointer"><input type="checkbox" class="gall" data-g="${EH(G.g)}" ${G.fs.every(f => f.vis) ? "checked" : ""} title="${I18N.t("files.showHideGroup")}"><span class="chv${open ? " on" : ""}" role="button" tabindex="0" aria-expanded="${open}" aria-label="${EH(I18N.t("files.ugrp.toggle", { g: n }))}"></span><span>${EH(n)}</span><em>${G.fs.length}</em></div>`;
}
/** Side bar width that holds the longest file name (label + box, colour square and padding); the default width when no file is open. */
let _nameCtx = /** @type {CanvasRenderingContext2D | null} */ (null);
function namesWidth(/** @type {number} */ dflt) {
  let names = E.files.filter((/** @type {any} */ f) => !f.gone).map((/** @type {any} */ f) => String(f.label || ""));
  // while the files are being opened the panel already has its final width: the labels are the ticked file names without extension
  if (!names.length && S.opening) names = ST.files.filter(usable).map((/** @type {FileRiga} */ f) => f.name.replace(/\.[^.]+$/, ""));
  if (!names.length) return dflt;
  const g = (_nameCtx ||= /** @type {CanvasRenderingContext2D} */ (document.createElement("canvas").getContext("2d"))), nm = document.querySelector("#flst .nm");
  g.font = nm ? getComputedStyle(nm).font : "600 13px system-ui, sans-serif";
  return Math.ceil(maxOf(names.map(n => g.measureText(n).width)) + 110);
}

// index of the file opened first: the first one that is not a blank (the list order stays), the first of all when every file is a blank
const isBlankFile = (/** @type {any} */ s) => s.type === "blank" || /(^|[^a-z0-9])(blk|blank|bianco)([^a-z0-9]|$)/i.test(String(s.file));
function firstOpenK(/** @type {any[]} */ session) {
  const i = session.findIndex(s => !isBlankFile(s));
  return i < 0 ? 0 : i;
}
// a DDA file is named after the file only: its engine label ends with «· Full Scan»
const noExt = (/** @type {string} */ s) => String(s).replace(/\.(mzml|mzxml|wiff2?|raw|d|dam|cdf)(?= · |$)/i, "");
// labels without extension and unique: the later twins get « (2)»…
const uniqueLabels = (/** @type {{ label: string }[]} */ fs) => { const n = /** @type {Record<string, number>} */ ({}); fs.forEach(f => { n[f.label] = (n[f.label] || 0) + 1; if (n[f.label] > 1) f.label += " (" + n[f.label] + ")"; }); };
const noFullScan = (/** @type {string} */ label, /** @type {any} */ dda) => dda ? String(label).replace(/ · Full Scan(?=[+−-]?$)/, "") : label;

// the live spectrum of a chromatogram shows the file the user picked in the list (unless the spectrum's own file menu was used)
function followSpectra(/** @type {any} */ f) {
  E.panels.forEach(q => { if (q.type === "spec" && q.link != null && q.dda == null && !q.all && !q.kSet && q.tab === f.kind && q.k !== f.k) { q.k = f.k; if (q.el && f.vis) { ctl(q); draw(q); } } });
}

// the work area follows the files shown and the panels open: panels, loading table or landing (area-lavoro.js, syncWork)
function syncWorkEmpty() { if (typeof syncWork === "function") syncWork(); }
