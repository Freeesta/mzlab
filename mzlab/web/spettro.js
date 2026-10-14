// @ts-strict
"use strict";
// Tools for the mass spectrum (Full Scan and MS2) and for the time axes. Classic script, loaded after scroll.js: it uses the globals of explore.js
// (E, Q, EH, M, css, fmt, draw, uiSave, ctl, big, dlx, info, shown, tabPanels, IC_DL, IC_TAB).
//  - ruler: difference of m/z between two peaks (the program only subtracts; what the difference means is for the student)
//  - "Parametri dello spettro": absolute / % axis, threshold and number of the m/z labels, decimals
//  - table of the peaks, ready for Excel (copy as TSV + HTML, or .xlsx)
//  - zoom history (Ctrl/Cmd+Z), whole view (Backspace), linked time axes, one-line description of the scan

const IC_RULER = '<svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"><rect x="1.5" y="5" width="13" height="6" rx="1"/><path d="M4.5 5v2.4M7.5 5v3.2M10.5 5v2.4M13 5v1.6"/></svg>';
const IC_SYNC = '<svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M1.5 5.5c1.5 0 1.5-3 3-3s1.5 3 3 3 1.5-1.5 2.5-1.5M1.5 13c1.5 0 1.5-3 3-3s1.5 3 3 3 1.5-1.5 2.5-1.5"/><path d="M11.5 1.5v13" stroke-dasharray="1.6 1.6"/><path d="M13 7.5 11.5 9 10 7.5" /></svg>';

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
// Adduct exchanges between two peaks: the short symbol on the chart, the full form in the tooltip. One list for the spectrum ruler and the map (mappa.js).
const AD_PAIRS = /** @type {Record<string, [number, string, string][]>} */ ({
  positive: [[21.9819, "Na⁺/H⁺", "[M+Na]⁺ / [M+H]⁺"], [17.0265, "NH₄⁺/H⁺", "[M+NH₄]⁺ / [M+H]⁺"], [37.9559, "K⁺/H⁺", "[M+K]⁺ / [M+H]⁺"]],
  negative: [[35.9767, "[M+Cl]⁻/[M−H]⁻", "[M+Cl]⁻ / [M−H]⁻"], [45.9929, "[M+HCOO]⁻/[M−H]⁻", "[M+HCOO]⁻ / [M−H]⁻"]]
});
/** @param {number} dm difference in m/z @param {boolean} hr @param {string=} polarity "positive" | "negative" | unknown (then the positive list, as the map always did) */
function adductPair(dm, hr, polarity) {
  const v = Math.abs(dm), tol = hr ? 0.003 : 0.5, hit = (AD_PAIRS[polarity === "negative" ? "negative" : "positive"]).find(a => Math.abs(v - a[0]) <= tol);
  return hit ? { short: hit[1], full: hit[2] } : null;
}
const specPol = p => { const f = p._a && p._a.data && p._a.data[0] && p._a.data[0].f; return f ? f.polarity : undefined; };
// opens the sidebar tab of the neutral losses filtered on |dm|, or the adducts when dm is a usual adduct exchange
function dmOpen(dm, hr, polarity, losses) {
  const ad = !losses && !!adductPair(dm, hr, polarity), q = String(+Math.abs(dm).toFixed(hr ? 4 : 1));
  if (window.BARRA && BARRA.isDataView()) BARRA.setTab(ad ? "adducts" : "losses", ad ? {} : { q });
  else if (window.QQQRef) QQQRef.open(ad ? "ad" : "ls", ad ? {} : { q });
}
/** right click on the label of a measure: straight to the losses or adducts tab; true when the click was on a label */
function measCtx(p, px, py) {
  const l = (p._a && p._a.lbls || []).find(b => b.dm != null && px >= b.x && px <= b.x + b.w && py >= b.y && py <= b.y + b.h);
  if (l) dmOpen(l.dm, l.hr, specPol(p));
  return !!l;
}
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
    const dd = Math.abs(q.b - q.a), hr = !!(p._a && p._a.hrp), ap = adductPair(dd, hr, specPol(p)), mid = (xa + xb) / 2;
    const num = dd.toFixed(hr ? p._a.dec : 1) + (hr ? ` (${(dd * 1000).toFixed(1)} mDa)` : "");
    /** @type {[string, string][]} */ const parts = ap ? [[ap.short, ""]] : [["Δ", ""], ["m/z", "italic "], [" " + num, ""]];             // «m/z» is a symbol of a quantity: italic, with the Δ and the number upright
    const ws = parts.map(([t, st]) => { g.font = fpx(12, st); return g.measureText(t).width; }), wt = ws.reduce((u, v) => u + v, 0);
    const bx = Math.max(x0 + 2, Math.min(x1 - wt - 2, mid - wt / 2)); let cx = bx;
    g.textAlign = "left"; g.lineJoin = "round";
    parts.forEach(([t, st], j) => { g.font = fpx(12, st); g.lineWidth = 3; g.strokeStyle = css("--panel"); g.strokeText(t, cx, y - 5); g.fillStyle = ac; g.fillText(t, cx, y - 5); cx += ws[j]; });
    g.font = fpx(12); g.strokeStyle = ac; g.lineWidth = 1;
    const fv = `<i>m/z</i> ${num}`;                                                                    // the label is a target of the hover box and of the right click (eventi.js)
    (p._a.lbls = p._a.lbls || []).push({ x: bx - 3, y: y - 5 - 13 * fz(), w: wt + 6, h: 5 + 13 * fz(), dm: dd, ap, hr, tip: ap ? `<b>${ap.full}</b> · Δ${fv}<div class="sm">${I18N.t("spec.dm.tip.ad")}</div>` : `Δ${fv}<div class="sm">${I18N.t("spec.dm.tip.ls")}</div>` });
  });
  g.restore();
}
function specClick(p, px, py) {                  // a click on the spectrum (no drag): the ruler picks the nearest visible label, else the nearest peak
  if (!(p.rul || (p.meas && p.meas.ref != null)) || !p._a || !p._a.snap) return;
  /** @type {any} */ let lb = null, bd = 34;                          // the m/z labels are the targets (start and end alike): the nearest one within 34 px, whatever the height of the click
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
      fn: () => dmOpen(Math.abs(q.b - q.a), !!(p._a && p._a.hrp), specPol(p), true)
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
document.addEventListener("keydown", e => {
  const a = E.active; if (!a || !a.el || !Q("#dpanels").offsetParent || a.type === "map" && false) return;
  if (/** @type {HTMLElement} */ (e.target).closest && /** @type {HTMLElement} */ (e.target).closest("input,textarea,select,[contenteditable='true']")) return;
  if (document.querySelector("dialog[open]")) return;
  const chosen = a.isel != null && a.ints && a.ints.find(i => i.id === a.isel);
  if ((e.key === "Delete" || e.key === "Backspace") && chosen && !e.ctrlKey && !e.metaKey) { e.preventDefault(); delInts(a, [chosen]); return; }      // with a peak selected, Delete/Backspace remove it (before "whole view")
  if (e.key === "Backspace" && !e.ctrlKey && !e.metaKey && !e.altKey) { e.preventDefault(); pushZh(a); a.zoom = null; a.zoomY = null; if (a.type === "spec") a.lock = null; draw(a); uiSave(); }
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
  p.tl = !p.tl; const b = p.el.querySelector('[data-a="tlink"]'); if (b) { b.classList.toggle("on", p.tl); b.setAttribute("aria-pressed", String(!!p.tl)); }
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

// ---------------------------------------------------------------- actions of the right-click menu of the spectrum (registry azioni/1, azioni.js)
// c = { vista: "spec", p, a: p._a, m: the picked m/z or null, has, lab: m/z as text, e: the event, px, py, pol }; the modules add theirs (dda.js, liste.js, libreria.js…)
/** @typedef {{ vista: string, p: any, a: any, m: number, has: boolean, lab: string, e: any, px: number, py: number, pol: any }} CtxSpec */
(() => {
  const S = ["spec"], T = I18N.t.bind(I18N);
  /** @param {any} o */
  const reg = o => AZIONI.registra({ viste: S, ...o });
  reg({ id: "spec.copy", gruppo: "value", visibile: /** @param {any} c */ c => !c.a.hrp, disponibile: /** @param {any} c */ c => c.has,
    nome: () => T("ctx.spec.copy"), aria: /** @param {any} c */ c => T("ctx.spec.copy.aria", { mz: c.lab }), esegui: /** @param {any} c */ c => { try { navigator.clipboard.writeText(c.lab).catch(() => {}); } catch (_) { /* clipboard unavailable */ } } });
  // low resolution: the product-ion spectrum of a precursor peak of the Full Scan
  reg({ id: "spec.extractMs2", gruppo: "xic", visibile: /** @param {any} c */ c => c.has && !!ms2OfPeak(c.p, c.m), nome: () => T("ctx.spec.extractMs2"), aria: /** @param {any} c */ c => T("ctx.spec.extractMs2.aria", { mz: c.lab }),
    esegui: /** @param {any} c */ c => { const e2 = ms2OfPeak(c.p, c.m); if (e2) extractMs2(c.p, e2); } });
  reg({ id: "spec.xic", gruppo: "xic", disponibile: /** @param {any} c */ c => c.has, nome: /** @param {any} c */ c => T("ctx.spec.xic", { mz: c.lab }), esegui: /** @param {any} c */ c => xicDirect(c.m, c.p) });
  // the m/z on an XIC already open: one row per XIC, or one row that lists them when they are more than 3
  reg({ id: "spec.overlay", gruppo: "xic", voci: /** @param {any} c */ c => {
    const xs = tabPanels().filter(q => q.type === "xic"), tip = T("ctx.spec.overlay.tip");
    /** @param {any} q */
    const ov = q => ({ nome: T("ctx.spec.overlay", { name: xicName(q), n: q.num || "?" }), tip, dim: !c.has, esegui: () => xicDirect(c.m, c.p, q) });
    if (xs.length > 3) return [{ nome: T("ctx.spec.overlayMany"), tip, dim: !c.has, esegui: () => menu({ preventDefault() {}, clientX: c.e.clientX, clientY: c.e.clientY }, xs.map(ov).map(o => ({ label: o.nome, tip, fn: o.esegui }))) }];
    return xs.map(ov);
  } });
  reg({ id: "spec.measure", gruppo: "tools", voci: /** @param {any} c */ c => measMenu(c.p, c.m).map((o, i) => ({ nome: o.label, tip: o.tip, dim: i === 0 && !c.has, esegui: o.fn })) });
  reg({ id: "spec.annotate", gruppo: "tools", disponibile: /** @param {any} c */ c => c.has, nome: () => T("ctx.spec.annotate"),
    esegui: /** @param {any} c */ async c => { const v = await ask(T("ctx.spec.annotate.prompt", { mz: c.lab }), ""); if (v) { c.p.anns.push({ x: c.m, text: v }); draw(c.p); } } });
  reg({ id: "spec.iso", gruppo: "tools", nome: () => T("ctx.spec.iso"), esegui: /** @param {any} c */ c => isoAsk(c.p) });
  reg({ id: "spec.isoRes", gruppo: "tools", visibile: /** @param {any} c */ c => !!c.p.iso && !!window.HR && !!c.a.hrp, nome: () => T("ctx.spec.isoRes"), esegui: /** @param {any} c */ c => isoRes(c.p) });
  reg({ id: "spec.isoOff", gruppo: "tools", visibile: /** @param {any} c */ c => !!c.p.iso, nome: () => T("ctx.spec.isoOff"), esegui: /** @param {any} c */ c => { c.p.iso = null; draw(c.p); } });
  // a spectrum that follows its chromatogram can be frozen; a frozen one can follow it again
  /** @param {any} p */
  const srcOf = p => p.src && E.panels.find(q => q.id === p.src && q.el);
  reg({ id: "spec.freeze", gruppo: "link", ordine: 2, visibile: /** @param {any} c */ c => !!c.p.link, nome: () => T("ctx.spec.freeze"), esegui: /** @param {any} c */ c => { freezeSpec(c.p); uiSave(); } });
  reg({ id: "spec.relink", gruppo: "link", ordine: 2, visibile: /** @param {any} c */ c => !c.p.link && !!srcOf(c.p), nome: () => T("ctx.spec.relink"), esegui: /** @param {any} c */ c => { const s = srcOf(c.p); if (s) liveSpec(c.p, s); } });
})();
// «Isotopic pattern…»: the formula (with its charge, or with the adduct) whose pattern is drawn over the spectrum, zoomed on it
/** @param {any} p */
async function isoAsk(p) {
  const a = p._a, v = await ask(I18N.t("ctx.spec.iso.prompt", { adduct: EH(defAdduct(a.data.map(/** @param {any} x */ x => x.f))) }), p.iso ? p.iso.formula + " " + p.iso.ad : "");
  if (!v) return;
  const mm = v.match(/^\s*(\S+)\s*(\[.*\].*)?$/); if (!mm) return;
  let ftxt = mm[1], adt = mm[2];
  const chg = ftxt.match(/^(.+?)([+\-−])$/);               // a formula written with its charge (C10H14N+, as copied from the drawing): the ion as it is, no proton added
  if (chg) { ftxt = chg[1]; if (!adt) adt = chg[2] === "+" ? "[M]+" : "[M]-"; }
  try { const r = await getFormula(ftxt, ""); p.iso = { formula: r.formula, ad: (adt || defAdduct(a.data.map(/** @param {any} x */ x => x.f))).trim() };
    const ion = QQQRef.ionCounts(p.iso.formula, p.iso.ad), pat = QQQRef.isoPattern(ion.n, ion.z);
    p.zoom = [pat[0].mz - 4, pat[pat.length - 1].mz + 4]; p._isoKey = JSON.stringify(p.zoom); draw(p); }   // zoom on the pattern (double click: whole spectrum); a later change of zoom switches the simulation off
  catch (e) { info(I18N.t("ctx.spec.iso.fail", { message: EH(/** @type {any} */ (e).message) })); }
}
// «Resolution of the pattern…» (high resolution): a number or «auto»
/** @param {any} p */
async function isoRes(p) {
  const v = await ask(I18N.t("ctx.spec.isoRes.prompt"), p.iso.R != null ? String(p.iso.R) : "auto");
  if (v == null) return; const t = String(v).trim().replace(",", ".");
  if (t === "auto") delete p.iso.R; else { const r = parseFloat(t); if (!(r >= 0)) { info(I18N.t("ctx.spec.isoRes.number")); return; } p.iso.R = r; }
  draw(p);
}
