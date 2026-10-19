// @ts-strict
// Chart drawing: chromatograms (TIC, XIC, corrections), hover line, zoom, RT x m/z map. Split from explore.js: classic script, loaded after pannelli.js.
// ------------------------------------------------------------------ chart drawing
// ---- corrections applied to the traces (fondo.js): the difference with a reference, then ONE background on the difference. Nothing is clipped here.
// ---- internal blank: a stretch of the trace chosen by the student (for example 0-2 min, where nothing elutes) gives the background level, which is
// subtracted from the whole trace. p.fondo.ib = { mode: "mean" | "median" | "line", a: [t0, t1], b: [t0, t1] (line only: second stretch) } (fondo.js).
// Not a blank-file subtraction: one number (or a straight line), not a profile in time. The level is computed on each trace (each file) on its own.
const ibkRange = (a, d = 1) => `${+a[0].toFixed(d)}-${+a[1].toFixed(d)} min`;
const ibkRanges = ib => ib.mode === "line" && ib.b ? I18N.t("ibk.and", { a: ibkRange(ib.a), b: ibkRange(ib.b) }) : ibkRange(ib.a);
const fondoText = p => (ibOf(p) ? ibkText(ibOf(p)) : p.fondo && p.fondo.mode === "auto" ? I18N.t("fondo.auto.text", { w: p.fondo.w }) : ibkText(null));
const corrText = p => { const r = window.SUBREF ? SUBREF.text(p) : "", f = p.fondo && p.fondo.mode !== "none"; return r ? (f ? fondoText(p) + " · " : "") + r : fondoText(p); };
const ibkStat = ib => I18N.t(`ibk.${ib.mode}`);
const ibkText = ib => !ib ? I18N.t("ibk.no") : I18N.t("ibk.yes", { ranges: ibkRanges(ib), stat: ibkStat(ib), clip: "" });
// ---- reference of the session (sottrazione.js): the difference sample minus reference is computed by the API (mzlab/differenza.py), trace by trace
async function diffTrace(p, s, o0) {
  const f = E.files[s.k]; if (!f) return null;
  const o = { ...o0, clip: false };                               // the numbers are never clipped: «negativi a 0» is only a way of drawing
  if (p.type === "xic") {
    const tr = p.traces.find(t => t.id === s.tid) || {}, a = tr.ion && window.HR ? HR.xicArgs(f, tr, p) : [s.mz, tr.w ?? p.tol];
    const j = await memo(`dx${s.k}|${a[0]}|${a[1]}|${f.lv}${SUBREF.qs(o)}`, () => J(`api/xic?k=${s.k}&mz=${a[0]}&tol=${a[1]}&level=${f.lv}${SUBREF.qs(o)}`));
    return j.traces[0];
  }
  if (p.type === "chrom") {
    const pda = p.kind === "pda", lv = p.flv ?? f.lv, prec = !pda && f.kind === "ms2" ? p.prec : null;
    return memo(`dc${s.k}|${p.kind}|${lv}|${pda ? "" : p.mz0 ?? ""}|${pda ? "" : p.mz1 ?? ""}|${prec ?? ""}|${p.filt ?? ""}${SUBREF.qs(o)}`,
      () => J(`api/chrom?k=${s.k}&kind=${p.kind}&level=${lv}` + (!pda && p.mz0 != null ? `&mz0=${p.mz0}` : "") + (!pda && p.mz1 != null ? `&mz1=${p.mz1}` : "") + (prec ? `&prec=${prec}` : "") + FT(p.filt) + SUBREF.qs(o)));
  }
  const j = await memo(`dm${s.k}${SUBREF.qs(o)}`, () => J(`api/mrm?k=${s.k}${SUBREF.qs(o)}`));
  const t = j.files[0].transitions.find(q => `${q.q1}>${q.q3}` === s.ion);
  return t && !t.nodiff ? t : null;
}
async function corrected(p, all) {
  const so = window.SUBREF && p.type !== "spec" && p.type !== "map" ? SUBREF.eff(p) : null, fo = p.fondo;
  if (!so && (!fo || fo.mode === "none")) return all;
  const out = [], others = so ? all.some(s => s.k !== so.k) : true;
  for (const s of all) {
    if (so && others && s.k === so.k) continue;                                    // the reference itself is the thing being subtracted (alone, it stays on the graph)
    let y = s.y.slice(), x = s.x; const note = []; let cb = false;
    if (so && s.k !== so.k) {
      const d = await diffTrace(p, s, so);
      if (d && d.rt.length) { x = d.rt; y = d.y; note.push(I18N.t(p.bk !== "" && p.bk != null && E.files[+p.bk] ? "corr.note.blank" : "corr.note.sub")); cb = true; }
    }
    const r = FONDO.apply(x, y, fo), yraw = y;
    if (r) { y = r.y; note.push(I18N.t(fo.mode === "tract" ? "corr.note.ibk" : "corr.note.baseline")); }
    out.push({ ...s, x, y, corr: note.join(" "), ibl: r ? r.ibl : null, bl: r && r.bl || null, yraw: r ? yraw : null, ze: r ? FONDO.zoneEnd(x, fo) : -Infinity, cb, cf: !!r });
  }
  return out;
}
async function seriesOf(p) { return corrected(p, await rawSeries(p)); }
// the kinds of chromatogram that come from an analog trace of the file: the pump pressure ("press"), any other device trace ("an:<id>") and a series of the instrument log ("reg", registro-serie.js)
const isAnalog = k => k === "press" || k === "reg" || (typeof k === "string" && k.startsWith("an:"));
const analogOf = (f, kind) => (f.analog || []).find(a => (kind === "press" ? a.kind === "pressure" : a.id === String(kind).slice(3)));
async function analogSeries(p) {
  if (p.kind === "reg") return window.REGISTRO ? REGISTRO.series(p, shownFor(p)) : [];
  const r = await Promise.all(shownFor(p).filter(f => analogOf(f, p.kind)).map(f => {
    const a = analogOf(f, p.kind);
    return memo(`an${f.k}|${a.i}`, () => J(`api/analog?k=${f.k}&i=${a.i}&n=2000`)).then(d => ({ x: d.t, y: d.y, color: f.color, dash: fdash(f), name: f.label, k: f.k, key: `a|${f.k}|${a.i}`, ion: a.id, time: f.time, unit: d.unit, unitNote: d.unit_note }));
  }));
  return r.filter(s => s.x.length);
}
// MS2 transitions (unit resolution), the counterpart of the MRM ones: the list comes from the scans, the chromatogram of a pair is the fragment window in the scans of its precursor
const MS2TR_MAX = 12;
async function ms2trSeries(p) {
  const mf = shownFor(p).filter(f => f.kind === "ms2" && !(window.HR && HR.isHr(f, f.lv)));
  const data = await Promise.all(mf.map(f => getMs2Tr(f.k).then(tr => ({ f, tr }))));
  const seen = new Map();
  data.forEach(({ tr }) => tr.forEach(t => { const key = `${t.q1}>${t.q3}`; if (!seen.has(key)) seen.set(key, { key, q1: t.q1, q3: t.q3, y: 0 }); seen.get(key).y += t.y; }));
  const list = [...seen.values()].sort((a, b) => a.q1 - b.q1 || a.q3 - b.q3).slice(0, MS2TR_MAX * 4);
  if (JSON.stringify(list.map(t => t.key)) !== JSON.stringify((p._trs || []).map(t => t.key))) { p._trs = list; ctl(p); }
  if (!p.tr) return null;
  const sel = p.tr === "__all" ? list.slice(0, MS2TR_MAX) : list.filter(t => t.key === p.tr);
  const out = await Promise.all(mf.flatMap((f, fi) => sel.map(t => getChrom(f.k, "tic", f.lv, t.q3 - 0.5, t.q3 + 0.5, t.q1, p.filt).then(d => ({
    x: d.rt, y: d.y, k: f.k, key: `c|${f.k}|tr|${t.key}`, ion: t.key, time: f.time, color: PAL[list.indexOf(t) % PAL.length],
    dash: f.type === "blank" ? [4, 3] : fi > 0 ? DASHES[fi % 4] : [], name: mf.length > 1 ? `${t.key} \u00b7 ${f.label}` : t.key, tid_label: t.key })))));
  return out.filter(s => s.x.length);
}
async function rawSeries(p) {
  if (p.type === "chrom" && isAnalog(p.kind)) return analogSeries(p);
  if (p.type === "chrom" && p.ranges && p.ranges.length && window.BANCO) return BANCO.rangeSeries(p);       // stacked graphs of the bench (banco.js)
  if (p.type === "chrom" && (p.tab || E.tab) === "ms2" && !isAnalog(p.kind) && p.kind !== "pda") { const s = await ms2trSeries(p); if (s) return s; }
  if (p.type === "chrom") {
    let fl = shownFor(p);
    if (p.kind !== "pda" && p.prec != null) fl = fl.filter(f => f.kind !== "ms2" || (f.precursors || []).some(v => Math.abs(v - p.prec) < 0.6));   // an MS2 experiment is a precursor: only the files that have it
    const r = await Promise.all(fl.map(f => getChrom(f.k, p.kind, p.flv ?? f.lv, p.kind === "pda" ? null : p.mz0, p.kind === "pda" ? null : p.mz1, p.kind !== "pda" && f.kind === "ms2" ? p.prec : null, p.filt).then(d => ({ x: d.rt, y: d.y, color: f.color, dash: fdash(f), name: f.label, k: f.k, key: `c|${f.k}|${p.kind}`, ion: p.kind.toUpperCase(), time: f.time }))));
    const sh = p.kind === "pda" && window.DISP ? DISP.pdaShift(p) : 0;       // «Allinea PDA e massa»: the PDA trace is shown later by the delay of the detector in series (display only)
    if (sh) r.forEach(s => { s.x = Array.from(s.x, v => v + sh); });
    return r.filter(s => s.x.length);
  }
  const files = shownFor(p);
  if (p.type === "xic") {
    const ts = p.traces, out = [];
    scanFiles(files).forEach((f, fi) => ts.forEach((t, ti) => out.push(getXic(f.k, ...(t.ion && window.HR ? HR.xicArgs(f, t, p) : [t.mz, t.w ?? p.tol]), f.lv).then(d => ({
      x: d.rt, y: d.y, k: f.k, mz: t.mz, tid: t.id, key: `x|${f.k}|${t.mz}`, ion: t.label, time: f.time,
      color: ts.length === 1 ? f.color : PAL[ti % PAL.length], dash: f.type === "blank" ? [4, 3] : ts.length > 1 ? (fi > 0 ? DASHES[fi % 4] : []) : fdash(f),
      name: ts.length === 1 ? f.label : `${t.label} · ${f.label}`, tid_label: t.label })))));
    return Promise.all(out);
  }
  // MRM
  const mf = files.filter(f => f.kind === "mrm"), out = [], seen = new Map();
  const data = await Promise.all(mf.map(f => getMrm(f.k).then(tr => ({ f, tr }))));
  data.forEach(({ tr }) => tr.forEach(t => { const key = `${t.q1}>${t.q3}`; if (!seen.has(key)) seen.set(key, { key, name: t.name || "" }); }));
  const list = [...seen.values()];
  if (JSON.stringify(list) !== JSON.stringify(p._trs)) { p._trs = list; ctl(p); }
  data.forEach(({ f, tr }, fi) => tr.forEach(t => {
    const key = `${t.q1}>${t.q3}`;
    const matchTr = !p.tr || (Array.isArray(p.tr) ? p.tr.includes(key) : p.tr === key);
    if (!matchTr) return;
    const ti = list.findIndex(l => l.key === key), single = !!p.tr && !Array.isArray(p.tr);
    const multiTr = Array.isArray(p.tr) && p.tr.length > 1;
    out.push({ x: t.rt, y: t.y, k: f.k, key: `m|${f.k}|${t.id}`, ion: key, time: f.time, color: (single && mf.length > 1) ? f.color : PAL[ti % PAL.length],
      dash: f.type === "blank" ? [4, 3] : (!single && !multiTr && fi > 0) ? DASHES[fi % 4] : (multiTr && fi > 0) ? DASHES[fi % 4] : (single && mf.length > 1) ? fdash(f) : [], name: mf.length > 1 ? `${key} · ${f.label}` : `${key} ${t.name || ""}`.trim(), tid_label: `${key} ${t.name || ""}`.trim() });
  }));
  return out;
}
function axes(g, W, H, x0, x1, ymax, yfmt, o = {}) {
  // o.log: logarithmic y axis from o.lo to ymax; o.stack: no numeric y axis (each trace has its own baseline)
  // o.xt / o.yt: axis titles. No horizontal grid: only the two axes with their ticks (cleaner figures for the report)
  const X = v => M.l + (v - x0) / (x1 - x0) * (W - M.l - M.r), ph = H - M.t - M.b, y0 = o.ymin || 0;   // y0 < 0: signals that go below zero (PDA baseline)
  const Y = o.log ? v => H - M.b - (Math.log10(Math.max(v, o.lo)) - Math.log10(o.lo)) / (Math.log10(ymax) - Math.log10(o.lo)) * ph : v => H - M.b - (v - y0) / (ymax - y0) * ph;
  g.clearRect(0, 0, W, H);
  const yt = o.stack ? [] : o.log ? Array.from({ length: Math.max(0, Math.floor(Math.log10(ymax)) - Math.ceil(Math.log10(o.lo)) + 1) }, (_, i) => Math.pow(10, Math.ceil(Math.log10(o.lo)) + i)) : nice(y0, ymax, Math.max(5, Math.round(ph / (32 * fz()))));
  frame(g, W, H, nice(x0, x1, o.xn || Math.max(8, Math.round((W - M.l - M.r) / (75 * fz())))).map(t => [X(t), +t.toFixed(2)]), yt.map(t => [Y(t), yfmt(t)]), o.xt, o.yt);
  if (y0 < 0) { g.save(); g.strokeStyle = css("--line"); g.setLineDash([4, 3]); g.beginPath(); g.moveTo(M.l, Math.round(Y(0)) + .5); g.lineTo(W - M.r, Math.round(Y(0)) + .5); g.stroke(); g.restore(); }   // zero line
  return { X, Y };
}
// axis lines, tick marks, tick labels and titles (shared by all the plots)
function frame(g, W, H, xt, yt, xtitle, ytitle) {
  const ink = css("--muted"); g.save(); g.lineWidth = 1; g.strokeStyle = ink; g.fillStyle = ink; g.font = fpx(12);
  g.beginPath(); g.moveTo(M.l + .5, M.t); g.lineTo(M.l + .5, H - M.b + .5); g.lineTo(W - M.r, H - M.b + .5); g.stroke();
  let needL = 0; for (const [, lab] of yt) needL = Math.max(needL, g.measureText(lab).width);
  g.canvas._needL = Math.ceil(needL + 6 + 4 + 14 * fz());                  // room for the numbers (right aligned at M.l - 6) and the title; the panels of a view take the widest
  g.textAlign = "right"; for (const [y, lab] of yt) { g.beginPath(); g.moveTo(M.l - 4, Math.round(y) + .5); g.lineTo(M.l, Math.round(y) + .5); g.stroke(); g.fillText(lab, M.l - 6, y + 3.5 * fz()); }
  g.textAlign = "center"; for (const [x, lab] of xt) { if (x < M.l - 1 || x > W - M.r + 1) continue; g.beginPath(); g.moveTo(Math.round(x) + .5, H - M.b); g.lineTo(Math.round(x) + .5, H - M.b + 4); g.stroke(); g.fillText(lab, x, H - M.b + 4 + 11 * fz()); }
  g.fillStyle = css("--ink"); g.font = fpx(12);
  const fnt = t => t === "m/z" ? fpx(12, "italic ") : fpx(12);           // m/z is always in italics
  if (xtitle) { g.font = fnt(xtitle); g.fillText(xtitle, (M.l + W - M.r) / 2, H - 6 * fz()); }
  if (ytitle) { g.font = fnt(ytitle); g.translate(1 + 12 * fz(), (M.t + H - M.b) / 2); g.rotate(-Math.PI / 2); g.fillText(ytitle, 0, 0); }
  g.restore(); g.font = fpx(12); g.lineWidth = 1;
}
// an annotation: a small framed label (panel background, accent border, 12 px text) joined to its point by a thin line; it stays inside the plot and goes to the side when there is no room above
function drawAnn(p, g, a, px, py, W, gap) {
  if (a.x < p._a.x0 || a.x > p._a.x1) return;                       // the annotated point is outside the zoomed range: neither label nor line (they come back when it is in view)
  g.save(); g.font = fpx(12); const tw = g.measureText(a.text).width, w = tw + 12, h = Math.round(20 * fz());
  let bx = px - w / 2, by = py - gap - h;
  if (by < M.t + 1) { by = Math.max(M.t + 1, Math.min(py - h / 2, p._a.H - M.b - h - 1)); bx = px + 14; if (bx + w > W - M.r) bx = px - 14 - w; }       // no room above: to the side
  bx = Math.max(M.l + 2, Math.min(W - M.r - w - 2, bx));
  const ex = Math.max(bx, Math.min(bx + w, px)), ey = by + h < py ? by + h : (by > py ? by : by + h / 2);                       // where the thin line meets the box
  g.strokeStyle = css("--muted"); g.lineWidth = 1; g.setLineDash([3, 3]); g.beginPath(); g.moveTo(px, py - 4); g.lineTo(ex, ey); g.stroke(); g.setLineDash([]);       // thin, grey, dashed, 4 px above the tip: never the colour of the trace (it would look like a taller peak)
  const r = 4; g.beginPath(); g.moveTo(bx + r, by); g.lineTo(bx + w - r, by); g.quadraticCurveTo(bx + w, by, bx + w, by + r); g.lineTo(bx + w, by + h - r); g.quadraticCurveTo(bx + w, by + h, bx + w - r, by + h); g.lineTo(bx + r, by + h); g.quadraticCurveTo(bx, by + h, bx, by + h - r); g.lineTo(bx, by + r); g.quadraticCurveTo(bx, by, bx + r, by); g.closePath();
  g.fillStyle = css("--panel"); g.fill(); g.strokeStyle = css("--muted"); g.stroke();
  g.fillStyle = css("--ink"); g.textAlign = "left"; g.fillText(a.text, bx + 6, by + 14);
  g.restore();
  (p._a.lbls = p._a.lbls || []).push({ x: bx, y: by, w, h, ann: a, tip: `<b>${EH(a.text)}</b><div class="sm">${I18N.t("draw.ann.tip")}</div>` });
}
// ---- the stacked chromatograms of a view: same time range, same left and right edge of the plot (the y axes stay independent)
const isLine = q => q.type === "chrom" || q.type === "xic" || q.type === "mrm";
const linked = p => tabPanels(p.tab).filter(q => isLine(q) && q.tl && q.el && !q._exp);
function unionTime(p, lo, hi) {
  linked(p).forEach(q => { if (q !== p && q._own) { lo = Math.min(lo, q._own[0]); hi = Math.max(hi, q._own[1]); } });
  return [lo, hi];
}
function syncView(p) {
  if (!isLine(p) || !p._a) return;
  p._needL = p.cv._needL || 0; p._needR = p.cv._needR || 0;
  const all = tabPanels(p.tab).filter(q => isLine(q) && q.el && !q._exp), z = fz();
  const L = Math.max(Math.round(66 * z), ...all.map(q => q._needL || 0)), R = Math.max(14, ...all.map(q => q._needR || 0));
  const marg = L !== (window.MARG_L || 0) || R !== (window.MARG_R || 0);
  window.MARG_L = L; window.MARG_R = R;
  all.forEach(q => { if (q === p) return; const st = q.tl && p.tl && q._u && p._u && (q._u[0] !== p._u[0] || q._u[1] !== p._u[1]); if (marg || st) drawSoon(q); });
  if (marg && (M.l !== L || M.r !== R)) drawSoon(p);                       // this panel was drawn with the old margins
}
const XT_RT = I18N.t("axis.rt"), YT_I = I18N.t("axis.intensity");
// title of the y axis: what the corrected trace is (blank file, background or both)
const yTitleI = sr => { const b = sr.some(s => s.cb), f = sr.some(s => s.cf); return b || f ? I18N.t(b && f ? "axis.sampBlankFondo" : b ? "axis.sampBlank" : "axis.intFondo") : YT_I; };
const analogTitle = (p, sr) => p.kind === "reg" ? REGISTRO.title(p, sr) : (p.kind === "press" ? I18N.t("axis.pressure") : `${(analogOf(E.files[sr[0].k] || {}, p.kind) || {}).id || p.kind.slice(3)}`) + (sr[0] && sr[0].unit && p.kind !== "press" ? ` (${sr[0].unit})` : "");
async function draw(p) {
  if (!p.cv.clientWidth || p.cv.clientWidth < 30) return;
  if (p.type === "map") {                                          // gestures come faster than frames: at most one map redraw per frame, the last request wins
    const t = performance.now();
    if (p._drawAt != null && t - p._drawAt < 12) { if (!p._drawQ) { p._drawQ = 1; FRAME.next(() => { p._drawQ = 0; draw(p); }); } return; }
    p._drawAt = t;
  }
  try { const r = p.type === "spec" ? await drawSpec(p) : p.type === "map" ? await drawMap(p) : await drawLines(p); if (r === false) return; afterDraw(p); syncView(p); if (p.tl) syncT(p); uiSave(); } catch (e) { if (p.rd) p.rd.textContent = I18N.t("draw.error", { message: e.message }); }
}
const MSG = { xic: I18N.t("msg.xic"), chrom: I18N.t("msg.chrom"), mrm: I18N.t("msg.mrm") };
const isHid = (p, s) => !!(p.hid && (p.hid[s.key] || (s.mz != null && p.hid["t" + s.mz])));

// the one "Correzione" menu of a chromatogram / XIC / MRM panel: none | a blank file | the background of a stretch (mean) | automatic baseline. One at a time.
function setCorr(p, v) {
  pushCampi(p, "storia.bianco");
  if (p.type === "map") return MAPPA.setCorr(p, v);
  p.bg = v === "" ? (S.ref.on[p.tab || E.tab] ? "x" : "") : v === "tract" ? "w" : +v.slice(1);       // the spectrum has no baseline: none | a file | the average of a stretch of time (its two ends are typed in the header)
  ctl(p); draw(p); uiSave();
}
// chromatogram / XIC / MRM: two independent choices, always applied in the same order (the file first, then the background on the difference)
function setSub(p, v) { pushCampi(p, "storia.bianco"); p.bk = v[0] === "f" ? +v.slice(1) : ""; ctl(p); draw(p); uiSave(); }
function setFondo(p, v) {
  pushCampi(p, "storia.fondo"); const f = p.fondo; p._pickIbk = false;
  if (v === "snip") { p.fondo = { ...f, mode: "auto", w: f.w || 1, ib: null }; }
  else if (v === "tract") {
    if (f.ib) f.mode = "tract";
    else if (p.sel && p.sel[1] > p.sel[0]) { ibkSet(p, "mean", [...p.sel]); p.sel = null; }
    else { p._pickIbk = true; info(I18N.t("ibk.pick")); }
  } else p.fondo = { ...f, mode: "none", w: f.w || 1, ib: null };
  ctl(p); draw(p); uiSave();
}
const ibOf = p => (p.fondo && p.fondo.mode === "tract" ? p.fondo.ib : null);       // the stretch chosen by the student, if the background is «tratto»
const ibkLevels = l => fmtA(l.lvl) + (l.lvl2 != null ? " " + I18N.t("ibk.andShort") + " " + fmtA(l.lvl2) : "") + " cps";
function ibkChip(p) {
  const fo = p.fondo, ib = ibOf(p), auto = !!fo && fo.mode === "auto"; if (!ib && !auto) return;
  const s0 = p._a && p._a.sr && p._a.sr[0], l0 = s0 && s0.ibl, bl = s0 && s0.bl, zd = fo.zd != null ? fo.zd : FONDO.ZD;
  const chip = document.createElement("span"); chip.className = "ibk";
  chip.title = I18N.t(ib ? "ibk.chip.title" : "fondo.chip.auto.title");
  const main = ib ? I18N.t("ibk.chip", { ranges: ibkRanges(ib), levels: l0 ? ibkLevels(l0) : "", stat: ibkStat(ib) }) : bl ? I18N.t("fondo.chip.auto", { min: fmtA(bl.min), max: fmtA(bl.max) }) : "";
  chip.innerHTML = `${main} <label title="${I18N.t("fondo.zone.title")}">${I18N.t("fondo.zone.label")} <input type="number" data-ib="zd" min="0" step="0.1" value="${+zd.toFixed(2)}" style="width:4.2em" aria-label="${I18N.t("fondo.zone.title")}"> min</label> <button data-ib="det" title="${I18N.t("fondo.zone.detect.title")}">${I18N.t("fondo.zone.detect")}</button> <label><input type="checkbox" data-ib="clip" ${p.clipDraw ? "checked" : ""}> ${I18N.t("ibk.clip.label")}</label>` +
    (ib ? ` <button data-ib="spec" title="${I18N.t("ibk.spec.title")}">${I18N.t("ibk.spec")}</button>` : "") + ` <button data-ib="off">${I18N.t("ibk.off")}</button>`;
  chip.querySelector("[data-ib=clip]").onchange = e => { p.clipDraw = e.target.checked; draw(p); uiSave(); };
  chip.querySelector("[data-ib=zd]").onchange = e => { fo.zd = Math.max(0, +e.target.value || 0); draw(p); uiSave(); };
  chip.querySelector("[data-ib=det]").onclick = () => { const z = s0 ? FONDO.front(s0.x, s0.yraw || s0.y) : null; if (z == null) return info(I18N.t("fondo.zone.none")); fo.zd = z; draw(p); uiSave(); };
  const sp = chip.querySelector("[data-ib=spec]"); if (sp) sp.onclick = () => ibkSpectra(p);
  chip.querySelector("[data-ib=off]").onclick = () => { setFondo(p, ""); };
  (p.corrbar || p.leg).appendChild(chip);
}
// the spectra of this chromatogram: subtract the mean spectrum of the blank stretch (same file), as Analyst/MassLynx do
function ibkSpectra(p) {
  const ib = ibOf(p); if (!ib) return;
  const sp = E.panels.filter(q => q.type === "spec" && (q.link === p.id || q.src === p.id));
  if (!sp.length) return info(I18N.t("ibk.noSpectra"));
  sp.forEach(q => { q.bg = "w"; q.bw0 = ib.a[0]; q.bw1 = ib.a[1]; ctl(q); draw(q); }); uiSave();
}
function ibkSet(p, mode, a, b) {
  if (!(a && a[1] > a[0])) return info(I18N.t("ibk.badRange"));
  pushCampi(p, "storia.fondo"); p.fondo = { mode: "tract", w: p.fondo.w || 1, ib: { mode, a: [a[0], a[1]], b: b ? [b[0], b[1]] : null } };
  p._ibkA = null; draw(p); uiSave();
}
function ibkPick(p, mode, iv) {
  if (mode === "line") { if (!p._ibkA) { p._ibkA = iv; info(I18N.t("ibk.first", { range: ibkRange(iv, 2) })); return; } ibkSet(p, "line", p._ibkA, iv); }
  else ibkSet(p, mode, iv);
}
function ibkMenuItems(p) {
  const it = [], iv = p.sel && p.sel[1] > p.sel[0] ? p.sel : null;
  if (iv && !ibOf(p)) it.push({ label: I18N.t("ibk.menu.sub"), fn: () => { ibkSet(p, "mean", [...iv]); p.sel = null; ctl(p); draw(p); } });
  if (ibOf(p)) it.push({ label: I18N.t("ibk.menu.off"), fn: () => setFondo(p, "") });
  return it;
}
// the background-of-a-stretch entries of the menu of a trace (registry azioni/1): the rows of ibkMenuItems
if (typeof AZIONI !== "undefined") AZIONI.registra({ id: "chrom.ibk", gruppo: "tools", ordine: 2, viste: ["chrom", "xic", "mrm"],
  voci: c => ibkMenuItems(c.p).map(/** @param {any} i */ i => ({ nome: i.label, esegui: i.fn })) });
function legend(p, all, note = "") {
  const sw = c => `<i style="background:${c}"></i>`, off = h => (h ? " off" : "");
  if (p.type === "xic") {
    // one trace: the trace is in the title of the panel and the legend lists only the files; several traces: two levels, the traces in bold, then the files
    const files = scanFiles(shownFor(p)), one = p.traces.length === 1;
    const dsw = (c, d) => d && d.length ? `<i style="background:repeating-linear-gradient(90deg,${c} 0 4px,transparent 4px 7px)"></i>` : sw(c);
    const trs = one ? "" : `<span class="lvl">` + p.traces.map(t => `<span class="tr${off(p.hid["t" + t.mz])}"><span class="tg" data-h="t${t.mz}" title="${I18N.t("leg.toggle.title")}">${sw(PAL[p.traces.indexOf(t) % PAL.length])}</span><b data-t="${t.id}" title="${I18N.t("leg.rename.title")}">${fmtFormula(t.label)}</b> ${t.w == null ? `<span>(${t.mz} ±${p.tol})</span>` : ""}<button data-r="${t.id}" title="${I18N.t("xic.remove.title")}">&times;</button></span>`).join("") + `</span>`;
    const fl = files.map((f, fi) => one ? `<span class="tg${off(p.hid["x|" + f.k + "|" + p.traces[0].mz])}" data-h="x|${f.k}|${p.traces[0].mz}" title="${I18N.t("leg.toggle.title")}">${sw(f.color)}${EH(f.label)}${legPol(f)}</span>`
      : `<span class="tg${off(p.hid["x|" + f.k + "|" + p.traces[0].mz] && p.traces.every(t => p.hid["x|" + f.k + "|" + t.mz]))}" data-hf="${f.k}" title="${I18N.t("leg.fileAll.title")}">${dsw("var(--muted)", fi > 0 ? DASHES[fi % 4] : [])}${EH(f.label)}</span>`).join("");
    p.leg.innerHTML = trs + (one ? fl : `<span class="lvl sm">${I18N.t("leg.files")}</span>${fl}`);
    p.leg.querySelectorAll("[data-hf]").forEach(b => b.onclick = () => { const k = b.dataset.hf, on = p.traces.every(t => p.hid["x|" + k + "|" + t.mz]); p.traces.forEach(t => { if (on) delete p.hid["x|" + k + "|" + t.mz]; else p.hid["x|" + k + "|" + t.mz] = true; }); draw(p); });
  } else p.leg.innerHTML = all.map(s => h`<span class="tg${raw(off(isHid(p, s)))}" data-h="${s.key}" title="${I18N.t("leg.toggle.title")}">${raw(sw(s.color))}${s.name}${raw(legPol(E.files[s.k]))}</span>`).join("");
  if ((p.corrbar || !note) && (p.type === "xic" ? p.traces.length <= 1 && scanFiles(shownFor(p)).length <= 1 : all.length <= 1)) p.leg.innerHTML = "";      // one file / one trace: the name is already in the header and in the mouse box
  if (p.corrbar) p.corrbar.innerHTML = "";                   // corrections (note, background of a stretch, reference): under the x axis, not over the graph
  if (note) (p.corrbar || p.leg).insertAdjacentHTML("beforeend", `<span class="sm">${EH(note)}</span>`);
  p.leg.querySelectorAll("[data-t]").forEach(b => b.ondblclick = async () => { const t = p.traces.find(x => x.id == b.dataset.t), v = await ask(I18N.t("leg.ion.prompt"), t.label); if (v) { t.label = v; draw(p); } });
  p.leg.querySelectorAll("[data-r]").forEach(b => b.onclick = () => { p.traces = p.traces.filter(x => x.id != b.dataset.r); ctl(p); draw(p); });
  p.leg.querySelectorAll("[data-h]").forEach(b => b.onclick = () => { const k = b.dataset.h; if (p.hid[k]) delete p.hid[k]; else p.hid[k] = true; draw(p); });
  legCollapse(p);
  ibkChip(p);
  if (window.SUBREF) SUBREF.chip(p);
}
// with many traces the legend would cover the graph: a small chip («7 tracce») and the list when the pointer rests on it
function legCollapse(p) {
  const L = p.leg, n = L.querySelectorAll(".tg").length; if (n <= 4) return;
  const lst = document.createElement("div"); lst.className = "lgl"; [...L.children].forEach(el => lst.appendChild(el));
  const chip = document.createElement("span"); chip.className = "lgc"; chip.textContent = I18N.t("leg.traces", { n });
  L.appendChild(chip); L.appendChild(lst);
}

// the highest and the lowest value of a trace in view, leaving out the initial zone (solvent front); if nothing else is in view the scale takes it
function traceMax(s, x0, x1) {
  let m = 1e-9, neg = 0;
  for (const za of [s.ze ?? -Infinity, -Infinity]) {
    for (let i = 0; i < s.x.length; i++) if (s.x[i] >= x0 && s.x[i] <= x1 && s.x[i] >= za) { if (s.ys[i] > m) m = s.ys[i]; if (s.ys[i] < neg) neg = s.ys[i]; }
    if (m > 1e-9 || neg < 0) break;
  }
  return { m, neg };
}
// the zero line (dashed; with a background it is the background itself, labelled) and the grey band of the initial zone
function zeroMarks(g, sr, X, Y, W, H, band) {
  const fo = sr.some(s => s.cf), y0 = Math.round(Y(0));
  g.save(); g.strokeStyle = css("--muted"); g.lineWidth = 1; g.setLineDash([5, 3]); g.beginPath(); g.moveTo(M.l, y0 + 0.5); g.lineTo(W - M.r, y0 + 0.5); g.stroke(); g.restore();
  if (fo) { g.save(); g.font = fpx(11); g.textAlign = "right"; g.fillStyle = css("--muted"); g.fillText(I18N.t("axis.zero"), W - M.r - 4, y0 - 4); g.restore(); }
  const s0 = band && sr.find(s => s.cf), xb = s0 ? Math.min(X(s0.ze), W - M.r) : 0;
  if (xb > M.l) { g.fillStyle = "rgba(120,120,120,.22)"; g.fillRect(M.l, M.t, xb - M.l, H - M.t - M.b); }
}
async function drawLines(p) {
  const all = await seriesOf(p);
  const { g, W, H } = setup(p.cv);
  const sr = all.filter(s => !isHid(p, s));
  if (!sr.length) {
    g.clearRect(0, 0, W, H); g.fillStyle = css("--muted"); g.fillText(all.length ? I18N.t("leg.allHidden") : MSG[p.type], M.l, 30);
    p._a = null; if (all.length) legend(p, all); else { p.leg.innerHTML = ""; if (p.corrbar) p.corrbar.innerHTML = ""; } return;
  }
  let lo = Infinity, hi = -Infinity;
  for (const s of sr) { lo = Math.min(lo, s.x[0]); hi = Math.max(hi, s.x[s.x.length - 1]); }
  p._own = [lo, hi];
  if (p.tl && !p._exp) [lo, hi] = unionTime(p, lo, hi);                    // linked chromatograms of a view share the time range: the union of their own
  p._u = [lo, hi];
  const x0 = p.zoom ? p.zoom[0] : lo, x1 = p.zoom ? p.zoom[1] : hi;
  const so = window.SUBREF ? SUBREF.eff(p) : null, clipDraw = !!p.clipDraw || !!(so && so.clip);
  sr.forEach(s => {
    s.ys = smooth(s.y, p.smooth ? 3 : 0);
    if (clipDraw) s.ys = s.ys.map(v => Math.max(0, v));          // «negativi a 0»: only the drawing; areas, heights and S/N use s.y
    const { m, neg } = traceMax(s, x0, x1);
    s.mx = Math.max(m, -neg * 0.1);                       // a trace that is only below zero keeps some room above the zero line
  });
  const stk = p.mode === "stk" || (!!p.ranges && p.ranges.length > 1), cas = p.mode === "cas" && sr.length > 1 && !p.log, logy = !!p.log && !stk && !cas && p.kind !== "pda", G = Math.max(...sr.map(s => s.mx));
  // rows of the stacked view: one per trace, or (stacked graphs of the bench) one per graph with the files overlaid in it
  const rowIx = new Map(); sr.forEach((s, i) => { const g = s.grp != null ? s.grp : "i" + i; if (!rowIx.has(g)) rowIx.set(g, rowIx.size); s.row = rowIx.get(g); });
  const nrow = rowIx.size, rowMax = new Array(nrow).fill(1e-9); sr.forEach(s => { rowMax[s.row] = Math.max(rowMax[s.row], s.mx); });
  // chart units: overlaid = intensity; stacked = row i + fraction of the height (common scale, or each row normalised 0-100 with p.norm100)
  sr.forEach((s, i) => { s.off = stk ? s.row : 0; s.sc = stk ? (p.norm100 ? rowMax[s.row] : G) / 0.92 : p.norm100 && !p.log ? s.mx / 100 : 1; });
  const U = (s, v) => s.off + v / s.sc;
  const yzf = !!p.zoomY && !stk && !logy;                       // y zoom (box, or drag on the numbers at the left of the axis): tall peaks are cut at the top
  const nrm = !stk && !!p.norm100 && !p.log;
  let ymax = stk ? nrow : (nrm ? 100 : G) * (p.ints.length ? 1.2 : 1.08) / (1 - (p._legPad || 0));   // room above the peaks for the area labels
  const ymaxAuto = ymax;
  let ymin = 0;                                          // PDA and baseline-corrected traces can be negative: extend the axis instead of drawing outside it
  if (!stk && !logy) for (const s of sr) for (let i = 0; i < s.x.length; i++) if (s.x[i] >= x0 && s.x[i] <= x1 && s.x[i] >= (s.ze ?? -Infinity) && s.ys[i] < ymin) ymin = s.ys[i];
  if (ymin < 0) ymin *= 1.08;
  const yfull = [ymin, ymaxAuto];
  if (yzf) { ymin = p.zoomY[0]; ymax = p.zoomY[1]; }
  const lo10 = Math.pow(10, Math.max(0, Math.floor(Math.log10(ymax)) - 4));
  const yt = nrm ? I18N.t("axis.relative") : stk ? (p.norm100 ? I18N.t("axis.rowsRel") : I18N.t("axis.rows")) : logy ? I18N.t("axis.logSuffix", { base: yTitleI(sr) }) : p.kind === "pda" && p.type === "chrom" ? I18N.t("export.col.pdaSignal") : isAnalog(p.kind) && p.type === "chrom" ? analogTitle(p, sr) : yTitleI(sr);
  // waterfall: the traces are ordered by time (the first one in front), each moved up and a little to the right; the room is taken from the axes
  const pw0 = W - M.l - M.r, ph0 = H - M.t - M.b;
  const cs = cas ? sr.slice().sort((a, b) => ((E.files[a.k] || {}).time ?? 1e9) - ((E.files[b.k] || {}).time ?? 1e9) || a.k - b.k) : null;
  const dxs = cas ? Math.min(16, pw0 * 0.22 / cs.length) : 0, dys = cas ? Math.min(28, ph0 * 0.5 / cs.length) : 0;
  const x1v = cas ? x0 + (x1 - x0) * pw0 / (pw0 - dxs * (cs.length - 1)) : x1;
  if (cas) ymax = ymax * ph0 / (ph0 - dys * (cs.length - 1));
  p.cv._needR = window.GRADIENTE ? GRADIENTE.needR(p) : 0;               // room for the right axis of the gradient: the panels of a view share it (syncView)
  const { X, Y } = axes(g, W, H, x0, x1v, ymax, fmt, { log: logy, lo: lo10, stack: stk, xt: XT_RT, yt, ymin });
  const ph = H - M.t - M.b;
  p._a = { x0, x1: x1v, cas, X, Y, W, H, sr, ymax, ymin, yfull, U, stk, logy, nl: rowMax, nrow, full: [lo, hi], yinv: py => ymin + (H - M.b - py) / ph * (ymax - ymin) };
  const withFondo = sr.some(s => s.cf);
  if ((ymin < 0 || withFondo) && !stk && !logy) zeroMarks(g, sr, X, Y, W, H, withFondo && !p._exp);
  if (ibOf(p) && !p._exp) {                                    // grey band(s) of the internal blank, always visible
    const ib = ibOf(p), bands = [ib.a].concat(ib.mode === "line" && ib.b ? [ib.b] : []), s0 = sr[0], l0 = s0 && s0.ibl;
    g.fillStyle = "rgba(120,120,120,.22)"; bands.forEach(b => { const xa = Math.max(X(b[0]), M.l), xb = Math.min(X(b[1]), W - M.r); if (xb > xa) g.fillRect(xa, M.t, xb - xa, H - M.t - M.b); });
    g.font = fpx(12); g.textAlign = "left"; g.fillStyle = css("--muted");
    g.fillText(I18N.t("ibk.band", { ranges: ibkRanges(ib), levels: l0 ? ibkLevels(l0) : "", extra: sr.length > 1 ? I18N.t("ibk.firstTrace") : "" }), Math.max(M.l + 6, X(ib.a[0]) + 4), H - M.b - 6);
  }
  if (p.sel && !p._exp) { g.fillStyle = "rgba(43,92,138,.10)"; g.fillRect(X(p.sel[0]), M.t, X(p.sel[1]) - X(p.sel[0]), H - M.t - M.b); }
  if (stk) {
    g.font = fpx(12); g.textAlign = "left";
    const hrw = !!window.BANCO && BANCO.on();
    sr.forEach(s => { if (s.grp != null && sr.find(q => q.row === s.row) !== s) return; const y = Y(s.off); g.strokeStyle = css("--line"); g.beginPath(); g.moveTo(M.l, y); g.lineTo(W - M.r, y); g.stroke(); const lab = s.grp != null && s.rlabel ? s.rlabel : s.name, t = lab.length > 34 ? lab.slice(0, 33) + "…" : lab;
      if (hrw) { const nl = "NL: " + rowMax[s.row].toExponential(2).replace("e+", "E"); g.save(); g.textAlign = "right"; g.lineWidth = 3; g.lineJoin = "round"; g.strokeStyle = css("--panel"); g.strokeText(nl, W - M.r - 4, y - 4); g.fillStyle = css("--muted"); g.fillText(nl, W - M.r - 4, y - 4); g.restore(); }
      g.lineWidth = 3; g.lineJoin = "round"; g.strokeStyle = css("--panel"); g.strokeText(t, M.l + 4, y - 4); g.fillStyle = s.color; g.fillText(t, M.l + 4, y - 4); g.lineWidth = 1; });
  }
  g.save(); g.beginPath(); g.rect(M.l + 1.5, M.t - 1, W - M.l - M.r - 1.5, H - M.t - M.b + 2); g.clip();   // the line (1.8 px wide) must not overdraw the y axis
  const tags = [];
  if (cas) {                                                     // back to front: white under every trace, so the ones behind are covered
    for (let k = cs.length - 1; k >= 0; k--) {
      const s = cs[k], sx = k * dxs, sy = -k * dys, f = E.files[s.k] || {}; let first = null, last = null;
      g.beginPath();
      s.x.forEach((r, i) => { if (r < x0 || r > x1) return; const px = X(r) + sx, py = Y(U(s, s.ys[i])) + sy; if (!first) { first = px; g.moveTo(px, Y(0) + sy); } g.lineTo(px, py); last = px; });
      if (first == null) continue;
      g.lineTo(last, Y(0) + sy); g.closePath(); g.fillStyle = css("--panel"); g.fill();
      g.strokeStyle = s.color; g.lineWidth = 1.6 + LWX(); g.setLineDash(s.dash); g.stroke(); g.setLineDash([]);
      tags.push({ s, f, px: first, py: Y(0) + sy });
    }
    g.restore(); g.save();                                       // time labels last (nothing draws over them), with a halo from --panel so they read on both themes
    g.font = fpx(12, "bold "); g.textAlign = "left"; g.lineWidth = 3; g.lineJoin = "round";
    for (const t of tags) { const txt = t.f.time != null ? `t = ${t.f.time} min` : t.s.name; g.strokeStyle = css("--panel"); g.strokeText(txt, t.px + 4, t.py - 4); g.fillStyle = t.s.color; g.fillText(txt, t.px + 4, t.py - 4); }
    g.font = fpx(12);
  } else for (const s of sr) {
    g.strokeStyle = s.color; g.lineWidth = (s.dash.length ? 1.5 : 1.8) + LWX(); g.setLineDash(s.dash); g.beginPath(); let st = false;
    let curX = -999, minY, maxY, firstY, lastY, minFirst = true;
    const flush = (cx) => { if (curX === -999) return; if (!st) { g.moveTo(cx, firstY); st = true; } else g.lineTo(cx, firstY); if (minFirst) { g.lineTo(cx, minY); g.lineTo(cx, maxY); } else { g.lineTo(cx, maxY); g.lineTo(cx, minY); } g.lineTo(cx, lastY); };
    s.x.forEach((r, i) => { if (r < x0 || r > x1) return; const px = Math.round(X(r)), py = Y(U(s, s.ys[i]));
      if (px !== curX) { flush(curX); curX = px; minY = maxY = firstY = lastY = py; minFirst = true; }
      else { if (py < minY) { minY = py; if (maxY === firstY) minFirst = true; } if (py > maxY) { maxY = py; if (minY === firstY) minFirst = false; } lastY = py; }
    });
    flush(curX);
    g.stroke();
  }
  g.restore(); g.setLineDash([]);
  p._a.tri = [];
  if (p.ms2tri && p.tab === "full") for (const s of sr) {              // MS2 events of the same physical file: a small triangle at the top at every MS2 scan
    const base = (E.files[s.k] || {}).file; if (!base) continue;
    const ev = window.DDA ? DDA.events(p, s.k) : null;                 // DDA file: its MS2 scans (only the followed ion's, if the student follows one)
    const sib = ev ? null : E.files.find(x => x.ms2_events && x.file.split("#")[0] === base.split("#")[0]); if (!ev && !sib) continue;
    g.fillStyle = s.color; let last = -9;
    for (const [rt, prec, sid] of (ev || sib.ms2_events)) {
      if (rt < x0 || rt > x1) continue; const px = X(rt); if (px - last < 3) continue; last = px;
      g.beginPath(); g.moveTo(px - 3, M.t + 1); g.lineTo(px + 3, M.t + 1); g.lineTo(px, M.t + 8); g.closePath(); g.fill();
      p._a.tri.push({ px, rt, prec, k2: ev ? ev.k2 : sib.k, sid });
    }
    if (ev && window.DDA) DDA.dots(p, g, X, Y, sr, x0, x1, U);
  }
  if (yzf) for (const s of sr) {                                  // a small triangle at the top says: this peak goes on above the graph
    /** @type {{ v: number, x: number } | null} */ let seg = null; const tri = x => { g.fillStyle = s.color; g.beginPath(); g.moveTo(X(x) - 4, M.t + 7); g.lineTo(X(x) + 4, M.t + 7); g.lineTo(X(x), M.t); g.closePath(); g.fill(); };
    s.x.forEach((r, i) => { if (r < x0 || r > x1) return; const v = U(s, s.ys[i]); if (v > ymax) { if (!seg || v > seg.v) seg = { v, x: r }; } else if (seg) { tri(/** @type {any} */ (seg).x); seg = null; } });
    if (seg) tri(/** @type {any} */ (seg).x);
  }
  for (const it of cas ? [] : p.ints) {                          // integrazioni: area colorata + barre trascinabili ai bordi (not in the waterfall view)
    const s = sr.find(q => q.key === it.key); if (!s) continue;
    if (it.noise && !p._exp) { g.fillStyle = "rgba(120,120,120,.18)"; const xa = Math.max(X(it.noise[0]), M.l), xb = Math.min(X(it.noise[1]), W - M.r); if (xb > xa) g.fillRect(xa, M.t, xb - xa, H - M.t - M.b); }       // noise stretch (for S/N)
    const r = integ(s, it.a, it.b); Object.assign(it, { area: r.area, height: r.height, rt: r.rt, corr: corrText(p) });
    const i0 = nearIdx(s.x, Math.min(it.a, it.b)), i1 = nearIdx(s.x, Math.max(it.a, it.b));
    if (i1 > i0) {
      const chosen = it.id === p.isel && !p._exp;                      // the selected peak: stronger fill and outline
      g.globalAlpha = chosen ? 0.5 : 0.28; g.fillStyle = s.color; g.beginPath(); g.moveTo(X(s.x[i0]), Y(U(s, s.ys[i0])));
      for (let j = i0; j <= i1; j++) g.lineTo(X(s.x[j]), Y(U(s, s.ys[j])));
      g.lineTo(X(s.x[i1]), Y(U(s, s.ys[i1]))); g.closePath(); g.fill(); g.globalAlpha = 1;
      if (chosen) { g.strokeStyle = s.color; g.lineWidth = 2.5; g.stroke(); }
      g.strokeStyle = s.color; g.lineWidth = 1; g.setLineDash([3, 2]); g.beginPath(); g.moveTo(X(s.x[i0]), Y(U(s, s.ys[i0]))); g.lineTo(X(s.x[i1]), Y(U(s, s.ys[i1]))); g.stroke(); g.setLineDash([]);
    }
    g.strokeStyle = s.color; g.lineWidth = 2.2;
    for (const e of ["a", "b"]) { const px = X(it[e]); g.beginPath(); g.moveTo(px, M.t + 10); g.lineTo(px, H - M.b); g.stroke(); g.fillStyle = s.color; g.fillRect(px - 4, M.t, 8, 12); }
    const ay = Math.max(M.t + 28, Y(U(s, s.ys[nearIdx(s.x, r.rt)])) - 20), lab = "A = " + fmtA(r.area) + (sr.length > 1 ? " · " + s.name : "");   // clearly above the apex, never on the peak
    g.font = fpx(13, "bold "); g.textAlign = "center"; g.lineWidth = 4; g.lineJoin = "round"; g.strokeStyle = css("--panel"); g.strokeText(lab, X(r.rt), ay); g.fillStyle = css("--ink"); g.fillText(lab, X(r.rt), ay); g.lineWidth = 1;
    const lw = g.measureText(lab).width; (p._a.lbls = p._a.lbls || []).push({ x: X(r.rt) - lw / 2 - 3, y: ay - 2 - 13 * fz(), w: lw + 6, h: 5 + 13 * fz(), tip: I18N.t("tip.area", { area: fmtFull(r.area) }) }); g.font = fpx(12);
  }
  // the cursor line is not part of the canvas: it is a thin overlay (cursorLine), so the arrows move it without redrawing the chromatogram (and it never enters a PNG)
  for (const a of p.anns) {
    let top = 0; sr.forEach(s => { const i = nearIdx(s.x, a.x); top = Math.max(top, U(s, s.ys[i])); });
    drawAnn(p, g, a, X(a.x), Math.max(M.t + 1, Math.min(H - M.b, Y(top))), W, 26);
  }
  if (window.GRADIENTE) { GRADIENTE.ensure(p); if (p.overlay) GRADIENTE.draw(p, g, p._a); }      // the gradient over the traces (PNG included)
  // what to show when the mouse is over the chart: RT and intensity of each visible trace at that point
  p._a.hov = (px, py) => {
    const x = xOf(p, px), rows = sr.map(s => { const i = nearIdx(s.x, x); return { s, v: s.ys[i], raw: s.yraw ? s.yraw[i] : null }; }).sort((a, b) => b.v - a.v);
    return { px, rt: x, html: `<b>RT ${x.toFixed(2)} min</b>` + rows.slice(0, 8).map(r => `<div><i style="background:${r.s.color}"></i>${EH(r.s.name.length > 30 ? r.s.name.slice(0, 29) + "…" : r.s.name)} <b>${fmt(r.v)}</b>${r.raw != null ? ` <span class="sm">${I18N.t("tip.raw", { v: fmt(r.raw) })}</span>` : ""}</div>`).join("") + (window.GRADIENTE && p.overlay ? GRADIENTE.hover(p, x, py) : "") + (rows.length > 8 ? `<div class="sm">${I18N.t("tip.more", { n: rows.length - 8 })}</div>` : "") };
  };
  legend(p, all, (p.type === "chrom" && p.kind === "press" && sr.some(s => s.unitNote) ? I18N.t("analog.unit.tenfold", sr.find(s => s.unitNote).unitNote.params) : "") + (p.type === "chrom" && !isAnalog(p.kind) && p.kind !== "pda" && (p.mz0 != null || p.mz1 != null) ? I18N.t("leg.note.mz", { lo: p.mz0 ?? "min", hi: p.mz1 ?? "max" }) : "") + (p.type === "chrom" && p.kind !== "pda" && p.prec && shown().some(f => f.kind === "ms2") ? I18N.t("leg.note.prec", { prec: p.prec }) : "") + (all.some(x => x.corr) ? I18N.t("leg.note.corr", { corr: all.find(x => x.corr).corr }) : "") + (stk ? (p.norm ? I18N.t("leg.note.rowNorm") : I18N.t("leg.note.rowScale", { h: fmt(G / 0.92) })) : ""));
}

// ---- mouse over the chart: vertical line + box with the values; the line also shows in the other panels with an RT axis
const xOf = (p, px) => { const a = p._a; return a.x0 + (px - M.l) / (a.W - M.l - M.r) * (a.x1 - a.x0); };
function hideHover(p) {
  if (p.tip) p.tip.hidden = true; if (p.vl) p.vl.hidden = true;
  E.panels.forEach(q => { if (q.vl && q !== p) q.vl.hidden = true; });
}
function showHover(p, px, py) {
  const a = p._a; if (!a || !a.hov || a.cas || px < M.l || px > a.W - M.r || py < M.t || py > a.H - M.b) return hideHover(p);
  const h = a.hov(px, py); if (!h) return hideHover(p);
  const cv = p.cv, el = p.el;
  p.vl.hidden = !!h.novl; p.vl.style.left = cv.offsetLeft + h.px + "px"; p.vl.style.top = cv.offsetTop + M.t + "px"; p.vl.style.height = a.H - M.t - M.b + "px";
  p.tip.hidden = false; p.tip.innerHTML = h.html;
  const tw = p.tip.offsetWidth, th = p.tip.offsetHeight;
  let l = cv.offsetLeft + px + 14; if (l + tw > el.clientWidth - 4) l = cv.offsetLeft + px - tw - 14;
  let t = cv.offsetTop + py + 12; if (t + th > el.clientHeight - 4) t = Math.max(4, el.clientHeight - th - 4);
  p.tip.style.left = Math.max(2, l) + "px"; p.tip.style.top = t + "px";
  if (h.rt != null) E.panels.forEach(q => {
    if (q === p || q.type === "spec" || !q._a || !q.vl) return;
    const b = q._a;
    if (h.rt < b.x0 || h.rt > b.x1) { q.vl.hidden = true; return; }
    q.vl.hidden = false; q.vl.style.left = q.cv.offsetLeft + b.X(h.rt) + "px"; q.vl.style.top = q.cv.offsetTop + M.t + "px"; q.vl.style.height = b.H - M.t - M.b + "px";
  });
}
// ---- zoom: Ctrl/Cmd + wheel (or trackpad pinch) around the cursor; Shift + drag pans
function clampView(z0, z1, f0, f1) {
  let span = z1 - z0; const full = f1 - f0;
  if (span >= full * 0.999) return null;
  span = Math.max(span, full * 0.002);
  if (z0 < f0) { z1 += f0 - z0; z0 = f0; } if (z1 > f1) { z0 -= z1 - f1; z1 = f1; }
  return [Math.max(f0, z0), Math.min(f1, z1)];
}
// y zoom in data units (bottom, top); back to the whole axis (null) when it covers everything. quiet = no redraw (a box zoom redraws once, with the x zoom)
function zoomYTo(p, lo, hi, quiet) {
  const a = p._a, f = a.yfull;
  lo = Math.max(lo, f[0]); hi = Math.min(hi, f[1]);
  p.zoomY = hi - lo < (f[1] - f[0]) * 1e-4 ? p.zoomY : lo <= f[0] + 1e-9 && hi >= f[1] - 1e-9 ? null : [lo, hi];
  if (p.type === "spec") p.lock = null;
  if (!quiet) draw(p);
}
function zoomTo(p, z0, z1) { const [f0, f1] = p._a.full; p.zoom = clampView(z0, z1, f0, f1); draw(p); }

// ---- RT-m/z map (like Xcalibur's "Ion Map" and MZmine's 2D viewer): each pixel is the mean intensity of one ion at one instant
const DIV = [[0, [213, 94, 0]], [0.5, [250, 250, 246]], [1, [0, 114, 178]]];       // Okabe-Ito vermilion #D55E00 <-> white <-> blue #0072B2: readable by colour-blind people
function lut(stops) {
  const out = new Uint8ClampedArray(256 * 4);
  for (let i = 0; i < 256; i++) {
    const t = i / 255; let k = 0; while (k < stops.length - 2 && t > stops[k + 1][0]) k++;
    const [t0, c0] = stops[k], [t1, c1] = stops[k + 1], f = Math.min(1, Math.max(0, (t - t0) / (t1 - t0)));
    for (let c = 0; c < 3; c++) out[i * 4 + c] = c0[c] + (c1[c] - c0[c]) * f; out[i * 4 + 3] = 255;
  }
  /** @type {any} */ (out)._u32 = new Uint32Array(out.buffer); return out;
}
const LUT_DIV = lut(DIV);
// sequential palettes of the map (2D and 3D): viridis (default) and cividis, both readable by colour-blind people
const MAP_PAL = {
  viridis: [[68, 1, 84], [70, 50, 126], [54, 92, 141], [39, 127, 142], [31, 161, 135], [74, 194, 109], [159, 218, 58], [253, 231, 37]],
  cividis: [[0, 32, 76], [31, 55, 108], [70, 77, 110], [105, 100, 112], [139, 125, 119], [177, 153, 117], [217, 184, 100], [254, 232, 56]]
};
const LUTS_SEQ = {};
const lutSeq = name => LUTS_SEQ[name] || (LUTS_SEQ[name] = lut(MAP_PAL[name].map((c, i, a) => [i / (a.length - 1), c])));
const mapPal = () => window.MAPPA && MAPPA.prefs.pal === "cividis" ? "cividis" : "viridis";
const mapBg = p => window.MAPPA ? MAPPA.corrQ(p) : "";                    // the request suffix of the background correction of the panel ("", "&bg=1" or a stretch)
const palCss = (name, diff) => diff ? "linear-gradient(90deg,#D55E00,#fafaf6,#0072B2)" : `linear-gradient(90deg,${MAP_PAL[name].map(c => `rgb(${c})`).join(",")})`;
const getMap = (k, lv, bg) => memo(`g${k}|${lv}|${bg}`, () => J(lv === "pda" ? `api/map?k=${k}&kind=pda${bg}` : `api/map?k=${k}&level=${lv}${bg}`).then(j => {
  const bin = atob(j.data), u = new Uint8Array(bin.length); for (let i = 0; i < bin.length; i++) u[i] = bin.charCodeAt(i);
  return { ...j, m: new Float32Array(u.buffer) };
}));
// Whether a file has a PDA matrix (RT x wavelength) for the map: a PDA trace alone is not one and the browser engine reads none, so the answer is probed once
// from the route (a plain fetch: the error is the answer, not a message) and kept on the file (`_pdaMap`); true re-renders the controls of the map panels, which then offer the PDA choice.
function pdaMapOk(f) {
  if (f._pdaMap === undefined) {
    f._pdaMap = null;
    fetch(`api/map?k=${f.k}&kind=pda`).then(r => r.json()).then(j => !j.error, () => false).then(ok => {
      f._pdaMap = ok;
      for (const q of E.panels) if (q.type === "map") { if (!ok && q.mapKind === "pda" && q.k === f.k) { q.mapKind = null; draw(q); } ctl(q); }
    });
  }
  return f._pdaMap === true;
}
// Optional normalisation before the comparison of two experiments: "max" divides each map by its own most intense point, "tic" by the sum of all its intensities
// (shown in parts per thousand). Without it two files acquired with different methods or injected amounts are subtracted in raw counts.
function mapNorm(m, norm) {
  if (norm !== "max" && norm !== "tic") return m;
  let t = 0; for (let i = 0; i < m.length; i++) t = norm === "max" ? Math.max(t, m[i]) : t + m[i];
  const f = t > 0 ? (norm === "max" ? 1 : 1000) / t : 1, o = new Float32Array(m.length); for (let i = 0; i < m.length; i++) o[i] = m[i] * f; return o;
}
function mapImage(p, A, B, scale, norm = "abs") {
  const dop = window.MAPPA ? MAPPA.diffOpt(!!B) : { show: "all", thr: B ? 0.02 : 0.004, rttol: 0 };     // mappa.js: what the difference shows, noise threshold, tolerance in RT
  const pal = mapPal(), key = `${A.k}|${B ? B.k : ""}|${scale}${scale === "gamma" ? p.gamma || 0.6 : ""}|${norm}|${pal}|${mapBg(p)}|${JSON.stringify(dop)}`;
  if (p._img && p._img.key === key && p._img.a === A.m && p._img.b === (B ? B.d || B.m : null)) return p._img;
  const n = A.m.length, v = new Float32Array(n);
  if (B && B.d) v.set(B.d); else { const mA = mapNorm(A.m, norm), mB = B ? mapNorm(B.m, norm) : null; for (let i = 0; i < n; i++) v[i] = mB ? mA[i] - mB[i] : mA[i]; }
  const nz = []; for (let i = 0; i < n; i++) { const a = Math.abs(v[i]); if (a > 0) nz.push(a); }
  let samp = nz; if (nz.length > 10000) {
    samp = []; const s = nz.length / 10000; for (let i = 0; i < 10000; i++) samp.push(nz[Math.floor(i * s)]);
  }
  samp.sort((a, b) => a - b);
  const ref = samp.length ? samp[Math.min(samp.length - 1, Math.floor(samp.length * 0.995))] : 1, floor = dop.thr;        // 99.5th percentile: one huge peak does not flatten the rest
  const T = x => { if (B && (dop.show === "up" && x < 0 || dop.show === "down" && x > 0)) return 0; const r = Math.min(1, Math.abs(x) / ref); if (r < floor) return 0; return scale === "lin" ? r : scale === "log" ? Math.log10(1 + 1000 * r) / 3 : scale === "gamma" ? Math.pow(r, p.gamma || 0.6) : Math.sqrt(r); };
  const { nrt, nmz } = A, off = document.createElement("canvas"); off.width = nrt; off.height = nmz;
  const ctx = /** @type {CanvasRenderingContext2D} */ (off.getContext("2d")), im = ctx.createImageData(nrt, nmz), tab = B ? LUT_DIV : lutSeq(pal);
  const tab32 = /** @type {Uint32Array} */ ((/** @type {any} */ (tab))._u32 || ((/** @type {any} */ (tab))._u32 = new Uint32Array(tab.buffer))), buf32 = new Uint32Array(im.data.buffer);
  for (let i = 0; i < nrt; i++) for (let j = 0; j < nmz; j++) {
    const x = v[i * nmz + j], t = T(x), q = B ? Math.round(127.5 + Math.sign(-x) * t * 127.5) : Math.round(t * 255);
    buf32[(nmz - 1 - j) * nrt + i] = tab32[q];
  }
  ctx.putImageData(im, 0, 0);
  return (p._img = { key, off, v, ref, T, a: A.m, b: B ? B.d || B.m : null });
}
// The whole view of the map: the union of the stretches with data of the files shown (first to last scan with peaks; lowest to highest m/z) with a small
// margin, inside the grid. A file without the extent (an older answer) or without data keeps the whole grid. Nothing is extrapolated.
function mapView(A, B, rt0, rt1, mz0, mz1) {
  const ex = [A, B].filter(Boolean).map(q => q.extent).filter(e => e && e.rt && e.mz);
  if (!ex.length) return { rt: [rt0, rt1], mz: [mz0, mz1] };
  const lo = (i, k) => Math.min(...ex.map(e => e[k][i])), hi = (i, k) => Math.max(...ex.map(e => e[k][i]));
  const a = lo(0, "rt"), b = hi(1, "rt"), c = lo(0, "mz"), d = hi(1, "mz"), pr = Math.max((b - a) * 0.01, 1e-3), pm = Math.max((d - c) * 0.01, 0.5);
  return { rt: [Math.max(rt0, a - pr), Math.min(rt1, b + pr)], mz: [Math.max(mz0, c - pm), Math.min(mz1, d + pm)] };
}
async function drawMap(p) {
  if (window.MAPPA) { MAPPA.corrInit(p); MAPPA.corrPick(p); }                // an old panel gets its «Correzione»; a stretch chosen with Alt + drag becomes the background
  const { g, W, H } = setup(p.cv);
  const say = t => { g.clearRect(0, 0, W, H); g.fillStyle = css("--muted"); g.fillText(t, M.l, 30); p.leg.innerHTML = ""; p._a = null; };
  const isPda = p.mapKind === "pda" || p.kind === "pda";
  const sf = isPda ? tabFiles(p.tab) : scanFiles(tabFiles(p.tab)); if (!sf.length) return say(I18N.t("map.noScans"));
  let f = E.browse && E.files[E.cur] && E.files[E.cur].kind === p.tab ? E.files[E.cur] : E.files[p.k];
  if (!f || (!isPda && f.kind === "mrm")) f = sf[0];
  const lv = isPda ? "pda" : f.lv;
  const se = window.SUBREF ? SUBREF.eff(p) : null;
  const rf = !isPda && se && E.files[se.k].kind !== "mrm" && E.files[se.k].lv === lv && se.k !== f.k ? E.files[se.k] : null;      // the reference: the one of the panel or the one of the session
  const A = { ...(await getMap(f.k, lv, mapBg(p))), k: f.k }, B = rf ? { ...(await getMap(rf.k, lv, mapBg(p))), k: rf.k } : null;
  if (B && window.MAPPA) B.d = await MAPPA.diffMap(f.k, rf.k, lv, p.norm || "abs");       // mappa.js: A - B in numpy, with the tolerance in RT
  const im = mapImage(p, A, B, p.scale, p.norm || "abs");
  const rt0 = A.rt0, rt1 = A.rt1, mzA = A.mz0, mzB = A.mz0 + A.nmz * A.dmz;
  const ext = mapView(A, B, rt0, rt1, mzA, mzB);                     // the whole view is where the files have data (no empty bands at the sides)
  const x0 = p.zoom ? p.zoom[0] : ext.rt[0], x1 = p.zoom ? p.zoom[1] : ext.rt[1], y0 = p.zoomY ? p.zoomY[0] : ext.mz[0], y1 = p.zoomY ? p.zoomY[1] : ext.mz[1];
  const pw = W - M.l - M.r, ph = H - M.t - M.b;
  const nTxt = { abs: "", max: I18N.t("map.norm.max"), tic: I18N.t("map.norm.tic") }[p.norm || "abs"], scTxt = p.scale === "gamma" ? I18N.t("map.sc.gamma", { g: p.gamma || 0.6 }) : I18N.t(`map.sc.${p.scale}`);
  g.clearRect(0, 0, W, H); g.imageSmoothingEnabled = !!(window.MAPPA && (p.zoom || p.zoomY) && MAPPA.smooth());      // zoomed in: «Liscia» blends the stretched bins
  if (!(window.MAPPA && MAPPA.zoomImg(p, g, { A, B, im, x0, x1, y0, y1, pw, ph, norm: p.norm || "abs" })))        // mappa.js: the zoomed region on new bins
    g.drawImage(im.off, (x0 - rt0) / (rt1 - rt0) * A.nrt, (mzB - y1) / A.dmz, (x1 - x0) / (rt1 - rt0) * A.nrt, (y1 - y0) / A.dmz, M.l, M.t, pw, ph);
  const X = v => M.l + (v - x0) / (x1 - x0) * pw, Yv = v => H - M.b - (v - y0) / (y1 - y0) * ph;
  frame(g, W, H, nice(x0, x1, 8).map(t => [X(t), +t.toFixed(2)]), nice(y0, y1, 6).map(t => [Yv(t), Math.round(t)]), XT_RT, isPda ? "nm" : "m/z");
  if (p.sel && !p._exp) { g.fillStyle = "rgba(43,92,138,.16)"; g.fillRect(X(p.sel[0]), M.t, X(p.sel[1]) - X(p.sel[0]), ph); }
  if (p.cur != null && !p._exp) { g.strokeStyle = css("--muted"); g.setLineDash([3, 3]); g.beginPath(); g.moveTo(X(p.cur), M.t); g.lineTo(X(p.cur), H - M.b); g.stroke(); g.setLineDash([]); }
  const mzAt = py => y1 - (py - M.t) / ph * (y1 - y0);
  p._a = { x0, x1, y0, y1, X, Y: Yv, W, H, full: ext.rt, fullY: ext.mz, grid: { rt: [rt0, rt1], mz: [mzA, mzB] }, f, mzAt, map: true, pda: isPda, dmz: A.dmz, ref: rf,
    mat: B ? null : { v: im.v, nrt: A.nrt, nmz: A.nmz, rt0, rt1, mz0: A.mz0, dmz: A.dmz },          // the matrix of the sight's preview (mappa.js)
    mapv: (rt, mz) => { const i = Math.floor((rt - rt0) / (rt1 - rt0) * A.nrt), j = Math.floor((mz - A.mz0) / A.dmz); return i >= 0 && i < A.nrt && j >= 0 && j < A.nmz ? im.v[i * A.nmz + j] : null; } };
  p._a.hov = (px, py) => {
    const rt = xOf(p, px), mz = mzAt(py), i = Math.min(A.nrt - 1, Math.max(0, Math.floor((rt - rt0) / (rt1 - rt0) * A.nrt))), j = Math.min(A.nmz - 1, Math.max(0, Math.floor((mz - A.mz0) / A.dmz)));
    const v = im.v[i * A.nmz + j];
    return { px, rt, html: isPda ? `<div>${rt.toFixed(2)} min, ${mz.toFixed(1)} nm</div><div>${(B && v > 0 ? "+" : "") + fmt(v)} mAU</div>` : I18N.t("map.hov", { rt: rt.toFixed(2), mz: mz.toFixed(mzd(p)), what: I18N.t(B ? "map.hov.diff" : "map.hov.mean"), v: (B && v > 0 ? "+" : "") + fmt(v) }) + `<div class="sm">${I18N.t("map.hov.bin", { lo: (A.mz0 + j * A.dmz).toFixed(0), hi: (A.mz0 + (j + 1) * A.dmz).toFixed(0) })}</div>` };
  };
  const rng = B ? [`<span title="${EH(I18N.t("map.leg2d.up.tip"))}">${I18N.t("map.leg2d.up")}</span>`, `<span title="${EH(I18N.t("map.leg2d.down.tip"))}">${I18N.t("map.leg2d.down")}</span>`] : ["0", fmt(im.ref)];       // the colour bar ends: the 99.5th percentile (above it the colour saturates)
  p.leg.innerHTML = `<span><i style="background:${f.color}"></i>${EH(f.label)}${B ? I18N.t("map.leg.minus", { label: EH(rf.label) }) : ""}</span><span class="sm">${rng[0]}</span><span class="cbar" style="background:${palCss(mapPal(), !!B)}"></span><span class="sm">${rng[1]}</span><span class="sm">${B ? "" : I18N.t("map.leg2d.mean", { scale: scTxt })}${nTxt}</span>`;
  if (p.corrbar) p.corrbar.innerHTML = "";
  if (window.SUBREF && B) SUBREF.chip(p);
  if (window.MAPPA) MAPPA.after(p);
}

function nearIdx(xs, v) { let lo = 0, hi = xs.length - 1; if (hi < 1) return 0; while (hi - lo > 1) { const m = (lo + hi) >> 1; xs[m] < v ? lo = m : hi = m; } return Math.abs(xs[lo] - v) < Math.abs(xs[hi] - v) ? lo : hi; }

