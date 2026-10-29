// @ts-strict
// Context menu of a file row (rename, colour, remove) and the quick file chooser of a graph header.
// Used by renderFileList (explore.js) and the graph headers; classic script, shares the page globals.
// right click on a file of the list: rename (only a label, the file on disk is not touched), colour, remove from the session
// Okabe-Ito + 2: distinguishable with colour-blindness
const FILE_COLORS = ["#0072B2", "#E69F00", "#009E73", "#D55E00", "#CC79A7", "#56B4E9", "#F0E442", "#000000", "#7a3b9b", "#8a8a8a"];
async function renameFile(f) { const v = await ask(I18N.t("files.rename.prompt"), f.label); if (v) { fileViewDo(() => { f.label = v; }); renderFileList(); renderNav(); redrawAll(); uiSave(); } }
// double click on the colour square: the colour picker opens straight away. The browser's own picker hangs from the (invisible) input: the input sits at the square,
// moved so that the picker (about COLOR_BOX) keeps COLOR_MARGIN px from every edge of the window
const COLOR_MARGIN = 16, COLOR_BOX = { w: 280, h: 340 };
function colorSpot(r, w, h) {
  const x = Math.max(COLOR_MARGIN, Math.min(r.left, innerWidth - COLOR_MARGIN - w));
  const y = Math.max(COLOR_MARGIN, Math.min(r.bottom + 4, innerHeight - COLOR_MARGIN - h));
  return { x, y };
}
function pickFileColor(f) {
  const sw = Q(`#flst .fl .nm[data-k="${f.k}"]`)?.closest(".fl")?.querySelector(":scope > i"), r = sw ? sw.getBoundingClientRect() : { left: 0, bottom: 0 }, at = colorSpot(r, COLOR_BOX.w, COLOR_BOX.h);
  const i = document.createElement("input"); i.type = "color"; i.value = /^#[0-9a-f]{6}$/i.test(f.color) ? f.color : "#0072B2"; i.setAttribute("aria-label", I18N.t("files.color.custom")); i.style.cssText = `position:fixed;left:${at.x}px;top:${at.y}px;width:1px;height:1px;opacity:0;border:0;padding:0`;
  i.onchange = () => { fileViewDo(() => { f.color = i.value; f.colorSet = true; }); renderFileList(); redrawAll(); uiSave(); };
  i.addEventListener("change", () => i.remove()); document.body.appendChild(i);
  try { i.showPicker(); } catch (_) { i.click(); }
}
function colorFile(f, ev) {
  Q("#colpop")?.remove();
  const d = document.createElement("div"); d.id = "colpop";
  d.setAttribute("role", "dialog"); d.setAttribute("aria-label", I18N.t("files.color.of", { label: f.label }));
  const cur = String(f.color).toLowerCase();
  // the swatches are real buttons: h`` returns a plain string, so a nested h`` would be escaped as text; wrap it in raw()
  d.innerHTML = h`<div class="sm" style="margin-bottom:4px">${raw(I18N.t("files.color.of", { label: EH(f.label) }))}</div><div class="sw">${raw(FILE_COLORS.map(c => h`<button type="button" data-c="${c}" style="background:${c}" title="${c}" aria-label="${I18N.t("files.color.swatch", { c })}" aria-pressed="${c.toLowerCase() === cur}"></button>`).join(""))}</div><label class="sm">${raw(I18N.t("files.color.custom"))} <input type="color" value="${/^#[0-9a-f]{6}$/i.test(f.color) ? f.color : "#1f77b4"}"></label>`;
  const set = c => { fileViewDo(() => { f.color = c; f.colorSet = true; }); d.remove(); renderFileList(); redrawAll(); uiSave(); };
  d.querySelectorAll("[data-c]").forEach(b => b.onclick = () => set(b.dataset.c));
  d.querySelector("input").onchange = e => set(e.target.value);
  d.addEventListener("keydown", e => { if (e.key === "Escape") { e.stopPropagation(); d.remove(); } });
  document.body.appendChild(d); (d.querySelector("[aria-pressed=true]") || d.querySelector("button"))?.focus(); const bx = d.getBoundingClientRect(); d.style.left = Math.max(COLOR_MARGIN, Math.min(ev.clientX, innerWidth - COLOR_MARGIN - bx.width)) + "px"; d.style.top = Math.max(COLOR_MARGIN, Math.min(ev.clientY, innerHeight - COLOR_MARGIN - bx.height)) + "px";
  setTimeout(() => document.addEventListener("mousedown", function h(e) { if (!d.contains(/** @type {Node} */ (e.target))) { d.remove(); document.removeEventListener("mousedown", h); } }), 0);
}
function fileCtx(e, f) {
  if (!f) return; e.stopPropagation();
  const hr = grpOf(f).startsWith("hr."), own = hr && f.ugrp;
  menu(e, [{ label: I18N.t("files.ctx.rename"), fn: () => renameFile(f) }, ...(hr ? [{ label: I18N.t("files.ctx.group"), fn: () => groupFiles(f) }] : []), ...(own ? [{ label: I18N.t("files.ctx.ungroup"), fn: () => groupFiles(f, true) }] : []), { label: I18N.t("files.ctx.color"), fn: () => colorFile(f, e) }, { label: I18N.t("files.ctx.resetColors"), fn: resetColors }, ...(/\.mzml$/i.test(f.file) ? [{ label: I18N.t("files.ctx.validate"), fn: () => /** @type {any} */ (window).VALIDA_MZML.show(f.file) }] : []), "-", { label: I18N.t("files.ctx.remove"), fn: () => removeFile(f) }]);
}
// quick file chooser opened from a graph header: tick/untick files (same switch as the list on the left), click a name to show only that file
function fileMenu(btn) {
  Q("#fpop")?.remove();
  const d = document.createElement("div"); d.id = "fpop";
  const paint = () => {
    const gs = []; tabFiles().forEach(f => { const g = grpOf(f); let G = gs.find(x => x.g === g); if (!G) gs.push(G = { g, fs: [] }); G.fs.push(f); });
    // @ts-ignore grpLabel is a top-level const of explore.js (classic script): the page project sees it, the strict project does not
    d.innerHTML = h`<div class="fpb"><button data-all="1">${raw(I18N.t("files.pop.all"))}</button><button data-all="0">${raw(I18N.t("files.pop.none"))}</button></div>` + gs.map(G => h`<div class="fgh"><span>${grpLabel(G.g)}</span></div>${raw(G.fs.map(f => h`<label><input type="checkbox" data-k="${f.k}" ${raw(f.vis ? "checked" : "")}><i style="background:${f.color}"></i><span class="fn" data-k="${f.k}">${f.label}</span></label>`).join(""))}`).join("");
    const done = () => { renderFileList(); renderNav(); redrawAll(); uiSave(); };
    d.querySelectorAll("[data-all]").forEach(b => b.onclick = () => { fileViewDo(() => tabFiles().forEach(f => f.vis = b.dataset.all === "1")); if (b.dataset.all === "0") cleanupOrphanPanels(); paint(); done(); });
    d.querySelectorAll("input").forEach(i => i.onchange = () => { fileViewDo(() => { E.files[+i.dataset.k].vis = i.checked; }); if (!i.checked) cleanupOrphanPanels(); done(); });
    d.querySelectorAll(".fn").forEach(n => n.onclick = e => { e.preventDefault(); fileViewDo(() => tabFiles().forEach(f => f.vis = f.k === +n.dataset.k)); cleanupOrphanPanels(); paint(); done(); });
  };
  paint(); document.body.appendChild(d);
  const r = btn.getBoundingClientRect(); d.style.left = Math.min(r.left, innerWidth - 240) + "px"; d.style.top = r.bottom + 4 + "px";
  setTimeout(() => document.addEventListener("mousedown", function h(e) { if (!d.contains(/** @type {Node} */ (e.target))) { d.remove(); document.removeEventListener("mousedown", h); } }), 0);
}
