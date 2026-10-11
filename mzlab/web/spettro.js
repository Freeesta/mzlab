"use strict";
// Tools for the mass spectrum (Full Scan and MS2) and for the time axes. Classic script, loaded after scroll.js: it uses the globals of explore.js
// (E, Q, EH, M, css, fmt, draw, uiSave, ctl, big, dlx, info, shown, tabPanels, IC_DL, IC_TAB).
//  - ruler: difference of m/z between two peaks (the program only subtracts; what the difference means is for the student)
//  - "Parametri dello spettro": absolute / % axis, threshold and number of the m/z labels, decimals
//  - table of the peaks, ready for Excel (copy as TSV + HTML, or .xlsx)
//  - zoom history (Ctrl/Cmd+Z), whole view (Backspace), linked time axes, one-line description of the scan

const IC_RULER = '<svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"><rect x="1.5" y="5" width="13" height="6" rx="1"/><path d="M4.5 5v2.4M7.5 5v3.2M10.5 5v2.4M13 5v1.6"/></svg>';
const IC_LINK = '<svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M6.8 9.2a2.6 2.6 0 0 0 3.7 0l2.4-2.4a2.6 2.6 0 0 0-3.7-3.7l-.7.7"/><path d="M9.2 6.8a2.6 2.6 0 0 0-3.7 0L3.1 9.2a2.6 2.6 0 0 0 3.7 3.7l.7-.7"/></svg>';

// y axis in % of the highest peak: the default is % for the MS2 spectra (the relative intensities of the fragments are what is compared), cps for Full Scan
const specRel = p => p.rel ?? false;
const SPEC_DEF = { thr: 5, nlab: 10, dec: 1 };

// ---------------------------------------------------------------- ruler: p.meas = { ref: m/z | null, list: [{a, b}] }
// a measure belongs to one view of one spectrum: a new zoom of the m/z axis, another scan or another file/level cancels it
const measKey = p => JSON.stringify([p.zoom, p.r0, p.r1, p.k, p.level, p.si, p.prec, p.all]);
function measClick(p, m) {
  p.meas = p.meas || { ref: null, list: [], key: measKey(p) };
  if (p.meas.ref == null) p.meas.ref = m;
  else if (Math.abs(m - p.meas.ref) > (p._a && p._a.hrp ? 0.001 : 0.04)) p.meas.list.push({ a: p.meas.ref, b: m });          // one more measure from the same reference
  draw(p); uiSave();
}
function measSet(p, m) { p.meas = p.meas || { ref: null, list: [], key: measKey(p) }; p.meas.ref = m; draw(p); uiSave(); }
function measClear(p) { p.meas = null; p.rul = false; const b = p.el.querySelector('[data-a="rul"]'); if (b) b.classList.remove("on"); draw(p); uiSave(); }
function drawMeas(p, g, X, Y, W) {
  const m = p.meas; if (!m || p._exp && false) return;
  if (m.key == null) m.key = measKey(p); else if (m.key !== measKey(p)) { p.meas = null; return; }       // zoom or scan changed: the measure is cancelled
  const ac = css("--accent"), x0 = M.l, x1 = W - M.r;
  g.save(); g.beginPath(); g.rect(x0, M.t - 1, x1 - x0, 400); g.clip();
  g.strokeStyle = g.fillStyle = ac; g.lineWidth = 1; g.font = fpx(12); g.textAlign = "center";
  if (m.ref != null && X(m.ref) >= x0 && X(m.ref) <= x1) {
    g.setLineDash([4, 3]); g.beginPath(); g.moveTo(X(m.ref), Y(0)); g.lineTo(X(m.ref), M.t + 2); g.stroke(); g.setLineDash([]);
    g.textAlign = "left"; g.fillText("rif.", X(m.ref) + 3, M.t + 10);
  }
  m.list.forEach((q, i) => {
    const xa = X(q.a), xb = X(q.b), y = M.t + 26 + (i % 6) * 17;
    g.setLineDash([2, 3]); g.globalAlpha = .55; g.beginPath(); g.moveTo(xa, y); g.lineTo(xa, Y(0)); g.moveTo(xb, y); g.lineTo(xb, Y(0)); g.stroke(); g.setLineDash([]); g.globalAlpha = 1;
    g.lineWidth = 1.4; g.beginPath(); g.moveTo(xa, y - 4); g.lineTo(xa, y + 4); g.moveTo(xa, y); g.lineTo(xb, y); g.moveTo(xb, y - 4); g.lineTo(xb, y + 4); g.stroke(); g.lineWidth = 1;
    const dd = Math.abs(q.b - q.a), t = "Δm/z " + dd.toFixed(p._a && p._a.hrp ? p._a.dec : 1) + (p._a && p._a.hrp ? ` (${(dd * 1000).toFixed(1)} mDa)` : ""), cx = Math.max(x0 + 40, Math.min(x1 - 40, (xa + xb) / 2));
    g.textAlign = "center"; g.lineWidth = 3; g.strokeStyle = css("--panel"); g.strokeText(t, cx, y - 5); g.fillStyle = ac; g.fillText(t, cx, y - 5); g.strokeStyle = ac; g.lineWidth = 1;
  });
  g.restore();
}
function specClick(p, px, py) {                  // a click on the spectrum (no drag): the ruler picks the nearest visible label, else the nearest peak
  if (!(p.rul || (p.meas && p.meas.ref != null)) || !p._a || !p._a.snap) return;
  let lb = null, bd = 34;                          // the m/z labels are the targets (start and end alike): the nearest one within 34 px, whatever the height of the click
  (p._a.lbls || []).forEach(l => { if (l.m == null) return; const d = Math.abs(l.x + l.w / 2 - px); if (d < bd) { bd = d; lb = l; } });
  if (lb) { measClick(p, lb.m); return; }
  const s = p._a.snap(px); if (s) measClick(p, s.m);
}
function measMenu(p, m) {                          // entries of the right-click menu of the spectrum
  const out = [{ label: I18N.t("spec.measure"), fn: () => measSet(p, m) }];
  if (p.meas && p.meas.list.length) p.meas.list.slice(-3).forEach(q => {
    const d = Math.abs(q.b - q.a).toFixed(p._a && p._a.hrp ? Math.min(4, p._a.dec) : 1);
    out.push({
      label: I18N.t("spec.measure.toLosses", { d }),
      tip: I18N.t("spec.measure.toLosses.tip"),
      fn: () => {
        if (window.BARRA && BARRA.isDataView()) BARRA.setTab("losses", { q: d });
        else if (window.QQQRef) QQQRef.open("ls", { q: d });
      }
    });
  });
  if (p.meas) out.push({ label: I18N.t("spec.measure.clear"), fn: () => measClear(p) });
  return out;
}

// ---------------------------------------------------------------- parameters of the spectrum
function specParams(p, btn) {
  const old = p.el.querySelector(".sp-pop"); if (old) { old.remove(); return; }
  const pop = document.createElement("div"); pop.className = "sp-pop";
  const cur = { rel: specRel(p), thr: p.thr ?? SPEC_DEF.thr, nlab: p.nlab ?? SPEC_DEF.nlab, dec: p.dec ?? (p._a ? p._a.dec : SPEC_DEF.dec) };
  pop.innerHTML = `<label title="${I18N.t("spec.par.thr.title")}">${I18N.t("spec.par.thr", { input: `<input data-s="thr" type="number" min="0" max="100" step="1" value="${cur.thr}">` })}</label>
    <label title="${I18N.t("spec.par.nlab.title")}">${I18N.t("spec.par.nlab", { input: `<input data-s="nlab" type="number" min="1" max="60" step="1" value="${cur.nlab}">` })}</label>
    <label>${I18N.t("spec.par.dec", { select: `<select data-s="dec">${(p._a && p._a.hrp ? [0, 1, 2, 3, 4, 5] : [0, 1, 2]).map(n => `<option ${cur.dec === n ? "selected" : ""}>${n}</option>`).join("")}</select>` })}</label>
    ${(p._a && p._a.data || []).some(x => x.d.pmz) ? `<label title="${I18N.t("spec.par.sticks.title")}">${I18N.t("spec.par.sticks", { select: `<select data-s="sticks"><option value="0" ${p.sticks ? "" : "selected"}>${I18N.t("spec.par.sticks.profile")}</option><option value="1" ${p.sticks ? "selected" : ""}>${I18N.t("spec.par.sticks.sticks")}</option></select>` })}</label>` : ""}
    <button data-s="reset" title="${I18N.t("spec.par.reset.title")}">${I18N.t("spec.par.reset")}</button>`;
  const pr = p.el.getBoundingClientRect(), br = btn.getBoundingClientRect();
  p.el.appendChild(pop); pop.style.left = Math.max(4, Math.min(br.left - pr.left, p.el.clientWidth - pop.offsetWidth - 6)) + "px"; pop.style.top = br.bottom - pr.top + 4 + "px";
  const apply = () => { draw(p); uiSave(); };
  pop.querySelectorAll("[data-s]").forEach(x => {
    const k = x.dataset.s;
    if (k === "reset") x.onclick = () => { p.sticks = false; p.thr = SPEC_DEF.thr; p.nlab = SPEC_DEF.nlab; p.dec = null; pop.remove(); specParams(p, btn); apply(); };
    else x.onchange = () => {
      if (k === "sticks") p.sticks = x.value === "1";
      else p[k] = Math.max(k === "nlab" ? 1 : 0, Math.min(k === "thr" ? 100 : k === "nlab" ? 60 : 5, Math.round(+x.value || 0)));
      apply();
    };
  });
  const away = e => { if (!pop.contains(e.target) && !btn.contains(e.target)) { pop.remove(); document.removeEventListener("mousedown", away, true); } };
  setTimeout(() => document.addEventListener("mousedown", away, true), 0);
}

// ---------------------------------------------------------------- zoom history, whole view (keys)
function pushZh(p) {
  const s = { zoom: p.zoom ? [...p.zoom] : null, zoomY: p.zoomY ? [...p.zoomY] : null }, h = p.zh = p.zh || [], l = h[h.length - 1];
  if (l && JSON.stringify({ zoom: l.zoom, zoomY: l.zoomY }) === JSON.stringify(s)) return;
  s.n = ++HSEQ; h.push(s); if (h.length > 15) h.shift();
}
function undoZoom(p) {
  const h = p.zh; if (!h || !h.length) return;
  const s = h.pop(); p.zoom = s.zoom; p.zoomY = s.zoomY; if (p.type === "spec") p.lock = null; draw(p); uiSave();
}
function undoLast(p) {                                                    // Ctrl/Cmd+Z: the last thing done, whether a zoom or an integration
  const z = p.zh && p.zh[p.zh.length - 1], i = p.ih && p.ih[p.ih.length - 1];
  if (i && (!z || i.n > (z.n || 0))) undoInt(p); else undoZoom(p);
}
document.addEventListener("keydown", e => {
  const a = E.active; if (!a || !a.el || !Q("#dpanels").offsetParent || a.type === "map" && false) return;
  if (e.target.closest && e.target.closest("input,textarea,select,[contenteditable='true']")) return;
  if (document.querySelector("dialog[open]")) return;
  const chosen = a.isel != null && a.ints && a.ints.find(i => i.id === a.isel);
  if ((e.key === "Delete" || e.key === "Backspace") && chosen && !e.ctrlKey && !e.metaKey) { e.preventDefault(); delInts(a, [chosen]); return; }      // with a peak selected, Delete/Backspace remove it (before "whole view")
  if (e.key === "Backspace" && !e.ctrlKey && !e.metaKey && !e.altKey) { e.preventDefault(); pushZh(a); a.zoom = null; a.zoomY = null; if (a.type === "spec") a.lock = null; draw(a); uiSave(); }
  else if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "z") { e.preventDefault(); undoLast(a); }
  else if (e.key === "Escape" && a.type === "spec" && (a.meas || a.rul)) measClear(a);
});

// ---------------------------------------------------------------- linked time axes: the panels with the chain on share the interval of time
function syncT(p) {
  const key = JSON.stringify(p.zoom || null); if (p._zs === key) return; p._zs = key;
  if (fsPanel()) return;                                         // full screen: the other panels keep their zoom
  tabPanels().filter(q => q !== p && q.tl && q.type !== "spec" && q.type !== "map" && JSON.stringify(q.zoom || null) !== key)
    .forEach(q => { q.zoom = p.zoom ? [...p.zoom] : null; q._zs = key; draw(q); });
}
function toggleTl(p) {
  p.tl = !p.tl; const b = p.el.querySelector('[data-a="tlink"]'); if (b) b.classList.toggle("on", p.tl);
  if (p.tl) { const o = tabPanels().find(q => q !== p && q.tl && q.type !== "spec" && q.type !== "map"); if (o) { p.zoom = o.zoom ? [...o.zoom] : null; p._zs = JSON.stringify(p.zoom || null); draw(p); } }
  uiSave();
}

// ---------------------------------------------------------------- one line under the spectrum: which scan it is
function scanLine(p, data, d0) {
  const f = data[0].f, parts = [], i = p.si != null ? p.si : d0.i0, n = d0.n;       // only what the title does not say: which scan(s), and for MS2 the precursor and the energy
  parts.push(d0.scans === 1 && i != null ? I18N.t("spec.scanLine.one", { i: i + 1, of: n ? "/" + n : "" }) : I18N.t("spec.scanLine.avg", { n: d0.scans }));
  if (p.level === 2) { const ce = p.prec != null ? ((f.ms2_exps || []).find(x => Math.abs(x.prec - p.prec) < 0.6) || {}).ce : null; parts.push(I18N.t("spec.scanLine.prec", { prec: p.prec != null ? p.prec : "?" }) + (ce != null ? I18N.t("spec.scanLine.ce", { ce }) : "")); }
  return parts.join(" · ");
}
