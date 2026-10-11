// «How an XIC and a spectrum are extracted» on the RT x m/z map (module, loaded the first time the view is turned on from mappa.js).
// The student places a box on the map (m/z window x RT window; drag it, drag its edges); beside the map the XIC of the m/z window (sum of the
// intensities with m/z inside the window, against RT) and the mean spectrum of the RT window are drawn from the SAME box, the integrated zone is
// coloured, dashed guides link the box to the axes of the two plots, and the numbers are written under the map. Nothing is labelled and nothing is
// found for the student: the position is always theirs (principle 1). The data are the ones of /api/xic and /api/spectrum.
const T = (k, o) => I18N.t(k, o);
const MIN_RT = 0.004;                                   // smallest RT window (min)
const minMz = p => window.HR && HR.isHr(p._a.f, p._a.f.lv || 1) ? 1e-4 : 0.05;       // smallest m/z window (Da): a few ppm in high resolution
const ex = p => MAPPA.st(p).ex;
const dp = p => window.HR && HR.isHr(p._a.f, p._a.f.lv || 1) ? 4 : 2;
const num = (v, d) => v == null || !Number.isFinite(v) ? "–" : v.toFixed(d);

// ---------------------------------------------------------------- numbers (pure: tests/test_mappa_estrai.py runs the same rule in numpy through node)
export function measure(xic, spec, w) {
  const out = { area: null, height: null, rtMax: null, mzMax: null, nPts: 0 };
  if (xic && xic.rt && xic.rt.length) {
    let i0 = -1, i1 = -1;
    for (let i = 0; i < xic.rt.length; i++) if (xic.rt[i] >= w.rt0 && xic.rt[i] <= w.rt1) { if (i0 < 0) i0 = i; i1 = i; }
    if (i0 >= 0) {
      let ar = 0, mx = -Infinity, at = i0;
      for (let i = i0; i <= i1; i++) { if (xic.y[i] > mx) { mx = xic.y[i]; at = i; } if (i > i0) ar += (xic.y[i] + xic.y[i - 1]) / 2 * (xic.rt[i] - xic.rt[i - 1]) * 60; }
      Object.assign(out, { area: ar, height: mx, rtMax: xic.rt[at], nPts: i1 - i0 + 1 });
    }
  }
  if (spec && spec.mz && spec.mz.length) {
    let mx = -Infinity, at = -1;
    for (let i = 0; i < spec.mz.length; i++) if (spec.mz[i] >= w.mz0 && spec.mz[i] <= w.mz1 && spec.y[i] > mx) { mx = spec.y[i]; at = i; }
    if (at >= 0) out.mzMax = spec.mz[at];
  }
  return out;
}

// ---------------------------------------------------------------- the box
function defaultBox(p) {
  const a = p._a, f = a.f, q = MAPPA.point(p) || { rt: (a.x0 + a.x1) / 2, mz: (a.y0 + a.y1) / 2 };
  const hr = window.HR && HR.isHr(f, f.lv || 1), half = hr ? HR.tolDa(f, q.mz, f.lv || 1) : 0, win = hr ? [q.mz - half, q.mz + half] : xicWin(q.mz, true);
  const w = Math.max(4 * scanStep(), (a.x1 - a.x0) / 20);
  return { rt0: q.rt - w / 2, rt1: q.rt + w / 2, mz0: win[0], mz1: win[1] };
}
function clampBox(p, b) {
  const a = p._a, MIN_MZ = minMz(p), lim = (lo, hi, min, f0, f1) => { let w = Math.max(hi - lo, min); if (lo < f0) { lo = f0; hi = lo + w; } if (hi > f1) { hi = f1; lo = Math.max(f0, hi - w); } return [lo, hi]; };
  [b.rt0, b.rt1] = lim(b.rt0, b.rt1, MIN_RT, a.full[0], a.full[1]); [b.mz0, b.mz1] = lim(b.mz0, b.mz1, MIN_MZ, a.fullY[0], a.fullY[1]);
}
const px = (p, b) => { const a = p._a; return { x0: a.X(b.rt0), x1: a.X(b.rt1), y0: a.Y(b.mz1), y1: a.Y(b.mz0) }; };       // pixel rectangle (y0 = top)
function grab(p, x, y, tol) {                                          // what is at (x, y): an edge, a corner, the inside, or nothing
  const b = ex(p).box, r = px(p, b);
  const hh = Math.max(r.y1 - r.y0, 18), hw = Math.max(r.x1 - r.x0, 18), cy = (r.y0 + r.y1) / 2, cx = (r.x0 + r.x1) / 2;      // a thin box is easy to hit: its hit area is at least 18 px
  const y0 = cy - hh / 2, y1 = cy + hh / 2, x0 = cx - hw / 2, x1 = cx + hw / 2, ty = Math.min(tol, hh / 3), tx = Math.min(tol, hw / 3);
  if (x < x0 - tx || x > x1 + tx || y < y0 - ty || y > y1 + ty) return null;
  const L = Math.abs(x - x0) <= tx, R = Math.abs(x - x1) <= tx, Tp = Math.abs(y - y0) <= ty, B = Math.abs(y - y1) <= ty;
  if (L || R || Tp || B) return { l: L, r: R, t: Tp, b: B };
  return { move: true };
}
const cursorOf = h => !h ? "" : h.move ? "move" : (h.l && h.t) || (h.r && h.b) ? "nwse-resize" : (h.r && h.t) || (h.l && h.b) ? "nesw-resize" : h.l || h.r ? "ew-resize" : "ns-resize";
function drag(p, h, start, x, y) {
  const a = p._a, s = ex(p), b = s.box, o = start.box, MIN_MZ = minMz(p), dRt = xOf(p, x) - xOf(p, start.x), dMz = a.mzAt(y) - a.mzAt(start.y);
  if (h.move) { b.rt0 = o.rt0 + dRt; b.rt1 = o.rt1 + dRt; b.mz0 = o.mz0 + dMz; b.mz1 = o.mz1 + dMz; }
  else { if (h.l) b.rt0 = Math.min(o.rt0 + dRt, o.rt1 - MIN_RT); if (h.r) b.rt1 = Math.max(o.rt1 + dRt, o.rt0 + MIN_RT); if (h.t) b.mz1 = Math.max(o.mz1 + dMz, o.mz0 + MIN_MZ); if (h.b) b.mz0 = Math.min(o.mz0 + dMz, o.mz1 - MIN_MZ); }
  clampBox(p, b); changed(p);
}
export function down(p, e, x, y) {
  const s = ex(p); if (!s) return false;
  const h = grab(p, x, y, e.pointerType === "touch" ? 14 : 6); if (!h) return false;
  e.preventDefault(); const start = { x, y, box: { ...s.box } }, r = p.cv.getBoundingClientRect();
  const mv = ev => drag(p, h, start, ev.clientX - r.left, ev.clientY - r.top);
  const up = () => { removeEventListener("mousemove", mv); removeEventListener("mouseup", up); };
  addEventListener("mousemove", mv); addEventListener("mouseup", up); return true;
}
export function touch(p, x, y) {                                       // a finger on the box moves it; returns the function that follows the finger, or null
  const s = ex(p); if (!s) return null; const h = grab(p, x, y, 16); if (!h) return null;
  const start = { x, y, box: { ...s.box } }; return (nx, ny) => drag(p, h, start, nx, ny);
}
export function hover(p, x, y) { const s = ex(p); if (!s) return false; p.cv.style.cursor = cursorOf(grab(p, x, y, 6)); return true; }
export function key(p, e) {                                            // arrows move the box by one scan / one bin, Alt + arrows resize it
  const s = ex(p), a = p._a; if (!s || !a || !["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(e.key)) return false;
  const MIN_MZ = minMz(p), b = s.box, dir = e.key === "ArrowRight" || e.key === "ArrowUp" ? 1 : -1, dt = scanStep(), dm = Math.max(MIN_MZ, (b.mz1 - b.mz0) / 4), horiz = e.key === "ArrowLeft" || e.key === "ArrowRight";
  if (e.altKey) { if (horiz) { b.rt0 -= dir * dt / 2; b.rt1 += dir * dt / 2; } else { b.mz0 -= dir * dm / 2; b.mz1 += dir * dm / 2; } }
  else if (horiz) { b.rt0 += dir * dt; b.rt1 += dir * dt; } else { b.mz0 += dir * dm; b.mz1 += dir * dm; }
  if (b.rt1 - b.rt0 < MIN_RT) { b.rt0 += (b.rt1 - b.rt0 - MIN_RT) / 2; b.rt1 = b.rt0 + MIN_RT; }
  if (b.mz1 - b.mz0 < MIN_MZ) { b.mz0 += (b.mz1 - b.mz0 - MIN_MZ) / 2; b.mz1 = b.mz0 + MIN_MZ; }
  clampBox(p, b); changed(p); return true;
}

// ---------------------------------------------------------------- data: the XIC of the m/z window and the mean spectrum of the RT window, from the API
function changed(p) { const s = ex(p); s.gen++; MAPPA.redraw(p); clearTimeout(s.t); s.t = setTimeout(() => load(p), 90); }
async function load(p) {
  const s = ex(p); if (!s || !p._a) return; const f = p._a.f, lv = f.lv || 1, b = { ...s.box }, gen = s.gen, c = (b.mz0 + b.mz1) / 2, tol = (b.mz1 - b.mz0) / 2;
  s.k = f.k;
  try {
    const [xj, sj] = await Promise.all([J(`api/xic?k=${f.k}&mz=${c}&tol=${tol}&level=${lv}`), J(`api/spectrum?k=${f.k}&rt0=${b.rt0}&rt1=${b.rt1}&level=${lv}&precursor=&bin=0.1`)]);
    if (ex(p) !== s || gen !== s.gen) return;                           // the box has moved again: the newest answer is on its way
    const tr = (xj.traces || [])[0];
    s.xic = tr ? { rt: tr.rt, y: tr.y, c, tol } : null;
    const m = MAPPA.st(p);
    if (s.fresh && m.fq && tr) {                                        // a feature was chosen: the RT window goes to the edges of its peak, then the plots are asked again
      s.fresh = false; const ed = autoEdges({ x: tr.rt, y: tr.y }, m.fq.rt); m.fedges = ed ? { rt0: ed[0], rt1: ed[1], auto: true } : null;
      if (ed) { s.box.rt0 = ed[0]; s.box.rt1 = Math.max(ed[1], ed[0] + MIN_RT); clampBox(p, s.box); changed(p); return; }
    }
    s.spec = { mz: sj.mz, y: sj.y, rt0: b.rt0, rt1: b.rt1, n: sj.n }; s.win = b; s.num = measure(s.xic, s.spec, b);
    MAPPA.redraw(p); table(p);
  } catch (e) { /* nothing to show: the plots keep their title */ }
}
export function numbers(p) { const s = ex(p); return s && s.num ? { win: s.win, ...s.num } : null; }       // read by the tests

// ---------------------------------------------------------------- drawing
export function overlay(p, g) {
  const s = ex(p), a = p._a; if (!s || !a) return;
  const r = px(p, s.box), L = a.W - M.r, Bt = a.H - M.b, ac = css("--accent"), ink = css("--ink");
  g.save();
  g.setLineDash([5, 4]); g.strokeStyle = ac; g.lineWidth = 1; g.globalAlpha = .8;                    // the guides: the edges of the box continue to the axes of the two plots
  g.beginPath(); for (const x of [r.x0, r.x1]) { g.moveTo(Math.round(x) + .5, r.y1); g.lineTo(Math.round(x) + .5, Bt); }
  for (const y of [r.y0, r.y1]) { g.moveTo(r.x1, Math.round(y) + .5); g.lineTo(L, Math.round(y) + .5); } g.stroke(); g.setLineDash([]);
  g.globalAlpha = .14; g.fillStyle = ac; g.fillRect(r.x0, r.y0, r.x1 - r.x0, r.y1 - r.y0);
  g.globalAlpha = 1; g.lineWidth = 2; g.strokeStyle = ac; g.strokeRect(r.x0, r.y0, r.x1 - r.x0, r.y1 - r.y0);
  g.fillStyle = css("--panel"); g.lineWidth = 1.5; for (const [x, y] of [[r.x0, r.y0], [r.x1, r.y0], [r.x0, r.y1], [r.x1, r.y1]]) { g.fillRect(x - 4, y - 4, 8, 8); g.strokeRect(x - 4, y - 4, 8, 8); }
  const n = s.num;
  if (n && n.rtMax != null && n.mzMax != null) {                                                       // where the maximum of the window is: the two coordinates go to the axes
    const x = a.X(n.rtMax), y = a.Y(n.mzMax); g.strokeStyle = ink; g.lineWidth = 1; g.setLineDash([2, 3]);
    g.beginPath(); g.moveTo(x + .5, y); g.lineTo(x + .5, Bt); g.moveTo(x, y + .5); g.lineTo(L, y + .5); g.stroke(); g.setLineDash([]);
    g.fillStyle = ink; g.beginPath(); g.arc(x, y, 3, 0, 2 * Math.PI); g.fill();
  }
  g.restore();
}
function plotArea(p, c) {
  const a = p._a, r = c.getBoundingClientRect(), d = Math.min(4, devicePixelRatio || 1), w = Math.round(r.width * d), h = Math.round(r.height * d);
  if (c.width !== w || c.height !== h) { c.width = w; c.height = h; }
  const g = c.getContext("2d"); g.setTransform(d, 0, 0, d, 0, 0); g.clearRect(0, 0, r.width, r.height); return { g, W: r.width, H: r.height, a };
}
export function margins(p) {
  const s = ex(p); if (!s || !p._a) return; const m = MAPPA.st(p);
  if (s.k !== p._a.f.k) { s.k = p._a.f.k; load(p); }                      // another file: the plots are asked again for the same box
  if (!m.xc.hidden) drawXic(p, m.xc, s);
  if (!m.sp.hidden) drawSpec(p, m.sp, s);
}
function drawXic(p, c, s) {                                           // the same RT axis as the map; the integrated zone is filled
  const { g, W, H, a } = plotArea(p, c), mt = 10, mb = 22, ink = css("--muted"), ac = css("--accent"), x = v => M.l + (v - a.x0) / (a.x1 - a.x0) * (W - M.l - M.r);
  g.font = fpx(11); g.fillStyle = ink; g.strokeStyle = ink; g.beginPath(); g.moveTo(M.l, H - mb + .5); g.lineTo(W - M.r, H - mb + .5); g.stroke();
  g.textAlign = "center"; for (const t of nice(a.x0, a.x1, 8)) { const xx = x(t); if (xx >= M.l - 1 && xx <= W - M.r + 1) g.fillText(String(+t.toFixed(2)), xx, H - mb + 13); }
  g.textAlign = "left"; const b = s.box, xa = x(b.rt0), xb = x(b.rt1);
  g.globalAlpha = .14; g.fillStyle = ac; g.fillRect(xa, mt, xb - xa, H - mt - mb); g.globalAlpha = 1;       // the RT window of the box, for the eye
  const t = s.xic; if (!t || !t.rt.length) { g.fillStyle = css("--ink"); g.fillText(T("mapex.xic.title"), M.l + 4, mt + 8); guides(g, xa, xb, mt, H - mb, ac); return; }
  let ymax = 0; for (let i = 0; i < t.rt.length; i++) if (t.rt[i] >= a.x0 && t.rt[i] <= a.x1 && t.y[i] > ymax) ymax = t.y[i];
  const y = v => H - mb - v / (ymax || 1) * (H - mt - mb);
  g.beginPath(); let first = true, started = false;                                                       // the filled area: between the two edges of the box
  const pts = []; for (let i = 0; i < t.rt.length; i++) if (t.rt[i] >= a.x0 && t.rt[i] <= a.x1) pts.push(i);
  g.fillStyle = ac; g.globalAlpha = .45;
  const inside = pts.filter(i => t.rt[i] >= b.rt0 && t.rt[i] <= b.rt1);
  if (inside.length) { g.beginPath(); g.moveTo(x(t.rt[inside[0]]), y(0)); for (const i of inside) g.lineTo(x(t.rt[i]), y(t.y[i])); g.lineTo(x(t.rt[inside[inside.length - 1]]), y(0)); g.closePath(); g.fill(); }
  g.globalAlpha = 1; g.strokeStyle = a.f.color || css("--ink"); g.lineWidth = 1.3; g.beginPath(); for (const i of pts) { const xx = x(t.rt[i]), yy = y(t.y[i]); if (first) { g.moveTo(xx, yy); first = false; } else g.lineTo(xx, yy); } g.stroke(); g.lineWidth = 1;
  guides(g, xa, xb, mt, H - mb, ac);
  g.fillStyle = css("--ink"); g.fillText(`XIC ${num(s.win ? s.win.mz0 : b.mz0, 2)}–${num(s.win ? s.win.mz1 : b.mz1, 2)} · max ${fmt(ymax)}`, M.l + 4, mt + 8);
  if (s.num && s.num.rtMax != null && s.num.rtMax >= a.x0 && s.num.rtMax <= a.x1) { g.strokeStyle = css("--ink"); g.setLineDash([2, 3]); g.beginPath(); g.moveTo(x(s.num.rtMax) + .5, mt); g.lineTo(x(s.num.rtMax) + .5, H - mb); g.stroke(); g.setLineDash([]); }
}
const guides = (g, xa, xb, y0, y1, col) => { g.save(); g.strokeStyle = col; g.setLineDash([5, 4]); g.beginPath(); for (const xx of [xa, xb]) { g.moveTo(Math.round(xx) + .5, y0); g.lineTo(Math.round(xx) + .5, y1); } g.stroke(); g.restore(); };
function drawSpec(p, c, s) {                                          // the same m/z axis as the map, turned on its side; the m/z window is a band
  const { g, W, H, a } = plotArea(p, c), ink = css("--muted"), ac = css("--accent"), x0 = 6, x1 = W - 8, b = s.box;
  const Y = v => H - M.b - (v - a.y0) / (a.y1 - a.y0) * (H - M.t - M.b);
  g.font = fpx(11); g.fillStyle = ink; g.strokeStyle = ink; g.beginPath(); g.moveTo(x0 + .5, M.t); g.lineTo(x0 + .5, H - M.b); g.stroke();
  const ya = Y(b.mz1), yb = Y(b.mz0); g.globalAlpha = .14; g.fillStyle = ac; g.fillRect(x0, ya, x1 - x0, yb - ya); g.globalAlpha = 1;
  g.save(); g.strokeStyle = ac; g.setLineDash([5, 4]); g.beginPath(); for (const yy of [ya, yb]) { g.moveTo(x0, Math.round(yy) + .5); g.lineTo(x1, Math.round(yy) + .5); } g.stroke(); g.restore();
  const sp = s.spec; g.fillStyle = css("--ink");
  if (!sp || !sp.mz.length) { g.fillText(T("mapex.spec.title"), x0 + 4, M.t + 10); return; }
  let ymax = 0; for (let i = 0; i < sp.mz.length; i++) if (sp.mz[i] >= a.y0 && sp.mz[i] <= a.y1 && sp.y[i] > ymax) ymax = sp.y[i];
  if (!ymax) { g.fillText(T("mappa.side.empty"), x0 + 4, M.t + 10); return; }
  g.lineWidth = 1.3; for (let i = 0; i < sp.mz.length; i++) {
    if (sp.mz[i] < a.y0 || sp.mz[i] > a.y1) continue; const yy = Math.round(Y(sp.mz[i])) + .5, inWin = sp.mz[i] >= b.mz0 && sp.mz[i] <= b.mz1;
    g.strokeStyle = inWin ? ac : (a.f.color || css("--ink")); g.globalAlpha = inWin ? 1 : .55; g.beginPath(); g.moveTo(x0, yy); g.lineTo(x0 + (x1 - x0) * sp.y[i] / ymax, yy); g.stroke();
  }
  g.globalAlpha = 1; g.lineWidth = 1; g.fillStyle = ink; g.textAlign = "left";
  g.fillText(T("mapex.spec.cap", { a: num(s.win ? s.win.rt0 : b.rt0, 2), b: num(s.win ? s.win.rt1 : b.rt1, 2) }), x0 + 4, H - M.b + 14);
  if (s.num && s.num.mzMax != null && s.num.mzMax >= a.y0 && s.num.mzMax <= a.y1) { g.strokeStyle = css("--ink"); g.setLineDash([2, 3]); g.beginPath(); g.moveTo(x0, Y(s.num.mzMax) + .5); g.lineTo(x1, Y(s.num.mzMax) + .5); g.stroke(); g.setLineDash([]); }
}

// ---------------------------------------------------------------- the numbers and the rule, under the map
function table(p) {
  const s = ex(p), m = MAPPA.st(p); if (!s || !m.grid) return; const hr = window.HR && HR.isHr(p._a.f, p._a.f.lv || 1), d = hr ? 4 : 2;
  let box = m.exBox; if (!box) { box = m.exBox = document.createElement("div"); box.className = "mzex"; box.setAttribute("role", "region"); box.setAttribute("aria-label", T("mapex.region")); (m.tab || m.grid).after(box); }
  const n = s.num, w = s.win || s.box, row = (k, lab, v, u) => `<div class="mzex-r"><dt>${T(lab)}</dt><dd><output data-k="${k}" aria-label="${T(lab)}">${v}</output>${u ? ` <span class="muted">${u}</span>` : ""}</dd></div>`;
  box.innerHTML = `<dl class="mzex-n">` +
    row("mz", "mapex.n.mz", `${num(w.mz0, d)} – ${num(w.mz1, d)}`, `(± ${num((w.mz1 - w.mz0) / 2, d)})`) + row("rt", "mapex.n.rt", `${num(w.rt0, 3)} – ${num(w.rt1, 3)}`, "min") +
    row("area", "mapex.n.area", n && n.area != null ? fmt(n.area) : "–", T("mapex.u.area")) + row("height", "mapex.n.height", n && n.height != null ? fmt(n.height) : "–", T("mapex.u.height")) +
    row("rtmax", "mapex.n.rtmax", n ? num(n.rtMax, 3) : "–", "min") + row("mzmax", "mapex.n.mzmax", n ? num(n.mzMax, d) : "–", "") + `</dl>` +
    `<p class="mzex-f"><b>XIC(t)</b> = Σ ${T("mapex.f.xic")}<br><b>S(m/z)</b> = ${T("mapex.f.spec")}<br>${T("mapex.f.area")}</p>` +
    `<p class="mzex-b"><button type="button" data-ex="off">${T("mapex.close")}</button></p>`;
  box.querySelector('[data-ex="off"]').onclick = () => MAPPA.explain(p, false);
  found(p, box); MAPPA.grow(p, "ex", Math.ceil(box.offsetHeight) + 4);
}

// ---------------------------------------------------------------- high resolution only: the features that mzFinder has already found (read-only, switched off at the start)
// window.MAPFEAT.features(file) -> [{ id, mz, rt, ppm, rt0?, rt1?, filters?: [{ key, label, thr, value?, from?: "height", cmp?: ">=" | "<=", unit?, note? }] }] or null;
// it is written by mzFinder from the result it already holds (TP_Mine/js/31_map_features.js): nothing is computed again in the engine.
const feats = p => { const f = p._a && p._a.f; try { return (window.MAPFEAT && f && window.HR && HR.isHr(f, f.lv || 1) ? MAPFEAT.features(f) : null) || []; } catch (e) { return []; } };
export function featOverlay(p, g) {
  const a = p._a, s = MAPPA.st(p); if (!a) return; g.save();
  for (const q of feats(p)) {
    if (q.rt < a.x0 || q.rt > a.x1 || q.mz < a.y0 || q.mz > a.y1) continue;
    const x = a.X(q.rt), y = a.Y(q.mz), sel = s.fsel === q.id;
    g.beginPath(); g.arc(x, y, sel ? 7 : 4.5, 0, 2 * Math.PI); g.globalAlpha = .9; g.fillStyle = css("--panel"); g.fill(); g.globalAlpha = 1;
    g.lineWidth = sel ? 2.4 : 1.4; g.strokeStyle = sel ? css("--accent") : css("--ink"); g.stroke();
  }
  g.restore();
}
function featAt(p, x, y, tol = 9) {
  const a = p._a; let best = null, bd = tol * tol;
  for (const q of feats(p)) { const d = (a.X(q.rt) - x) ** 2 + (a.Y(q.mz) - y) ** 2; if (d <= bd) { bd = d; best = q; } }
  return best;
}
export function featHover(p, x, y) {
  const q = featAt(p, x, y); if (!q) return false; const v = MAPPA.valueAt(p, q.rt, q.mz);
  p.rd.textContent = T("mapfeat.read", { mz: q.mz.toFixed(dp(p)), rt: q.rt.toFixed(2), v: v == null ? "–" : fmt(v) }); p.cv.style.cursor = "pointer"; return true;
}
export function featClick(p, x, y) { const q = featAt(p, x, y, 11); if (!q) return false; selectFeat(p, q); return true; }
export async function selectFeat(p, q) {
  const m = MAPPA.st(p); m.fsel = q.id; m.fq = q; m.fedges = q.rt0 != null && q.rt1 != null ? { rt0: q.rt0, rt1: q.rt1, auto: false } : null;
  if (!m.ex) await MAPPA.explain(p, true);
  const s = ex(p); if (!s) return; const d = q.mz * (q.ppm ?? 5) * 1e-6, w = Math.max(0.15, 4 * scanStep()) / 2;
  s.box = { mz0: q.mz - d, mz1: q.mz + d, rt0: m.fedges ? m.fedges.rt0 : q.rt - w, rt1: m.fedges ? m.fedges.rt1 : q.rt + w }; s.fresh = !m.fedges; clampBox(p, s.box); changed(p);
}
export function featClear(p) { const m = MAPPA.st(p); m.fsel = null; m.fq = null; m.fedges = null; if (m.exBox) table(p); MAPPA.redraw(p); }
function found(p, box) {                                               // «How it was found»: window, peak with its edges, the filters with their thresholds
  const m = MAPPA.st(p), s = ex(p), q = m.fq; let el = box.querySelector(".mzex-found"); if (el) el.remove(); if (!q || !s) return;
  const n = s.num || {}, d = dp(p), ppm = q.ppm ?? 5, dm = q.mz * ppm * 1e-6, ed = m.fedges;
  const rows = (q.filters || []).map(f => {
    const v = f.value != null ? f.value : f.from === "height" ? n.height : null, ok = v != null && (f.cmp === "<=" ? v <= f.thr : v >= f.thr);
    return `<tr><th scope="row">${EH(f.label)}</th><td>${v == null ? "–" : fmt(v)}${f.unit ? " " + EH(f.unit) : ""}</td><td>${f.cmp === "<=" ? "≤" : "≥"} ${fmt(f.thr)}</td><td>${v == null ? EH(f.note || "–") : ok ? T("mapfeat.pass") : T("mapfeat.below")}</td></tr>`;
  }).join("");
  el = document.createElement("div"); el.className = "mzex-found"; el.setAttribute("role", "region"); el.setAttribute("aria-label", T("mapfeat.region"));
  el.innerHTML = `<h4>${T("mapfeat.title", { id: EH(String(q.id)) })}</h4>` +
    `<p>${T("mapfeat.win", { mz: q.mz.toFixed(d), ppm: ppm, a: (q.mz - dm).toFixed(d), b: (q.mz + dm).toFixed(d) })}</p>` +
    `<canvas class="mzex-prof" aria-label="${T("mapfeat.prof.aria")}"></canvas>` +
    `<p class="muted">${ed ? T(ed.auto ? "mapfeat.edges.auto" : "mapfeat.edges.rec", { a: ed.rt0.toFixed(3), b: ed.rt1.toFixed(3) }) : T("mapfeat.edges.none")}</p>` +
    (rows ? `<table class="mzex-flt"><caption>${T("mapfeat.filters")}</caption><thead><tr><th scope="col">${T("mapfeat.f.name")}</th><th scope="col">${T("mapfeat.f.value")}</th><th scope="col">${T("mapfeat.f.thr")}</th><th scope="col"></th></tr></thead><tbody>${rows}</tbody></table>` : "") +
    `<p class="muted">${T("mapfeat.note")}</p>`;
  box.appendChild(el);
  const c = el.querySelector("canvas"), t = s.xic;
  requestAnimationFrame(() => {
    const r = c.getBoundingClientRect(), dpr = Math.min(4, devicePixelRatio || 1); c.width = Math.round(r.width * dpr); c.height = Math.round(r.height * dpr);
    const g = c.getContext("2d"); g.setTransform(dpr, 0, 0, dpr, 0, 0); const W = r.width, H = r.height, ml = 6, mb = 16, mt = 8;
    if (!t || !t.rt.length) return;
    const lo = Math.min(...[ed ? ed.rt0 : q.rt, q.rt]) - 0.3, hi = Math.max(...[ed ? ed.rt1 : q.rt, q.rt]) + 0.3, pts = []; for (let i = 0; i < t.rt.length; i++) if (t.rt[i] >= lo && t.rt[i] <= hi) pts.push(i);
    if (!pts.length) return; let ymax = Math.max(...pts.map(i => t.y[i])); const thr = (q.filters || []).filter(f => f.from === "height").map(f => f.thr); ymax = Math.max(ymax, ...thr) * 1.05;
    const X = v => ml + (v - lo) / (hi - lo) * (W - 2 * ml), Y = v => H - mb - v / ymax * (H - mb - mt), ink = css("--ink"), ac = css("--accent"), mu = css("--muted");
    if (ed) { g.globalAlpha = .35; g.fillStyle = ac; g.beginPath(); g.moveTo(X(ed.rt0), Y(0)); for (const i of pts) if (t.rt[i] >= ed.rt0 && t.rt[i] <= ed.rt1) g.lineTo(X(t.rt[i]), Y(t.y[i])); g.lineTo(X(ed.rt1), Y(0)); g.closePath(); g.fill(); g.globalAlpha = 1;
      g.strokeStyle = ac; g.setLineDash([4, 3]); g.beginPath(); for (const v of [ed.rt0, ed.rt1]) { g.moveTo(X(v) + .5, mt); g.lineTo(X(v) + .5, H - mb); } g.stroke(); g.setLineDash([]); }
    g.strokeStyle = ink; g.lineWidth = 1.3; g.beginPath(); pts.forEach((i, k) => k ? g.lineTo(X(t.rt[i]), Y(t.y[i])) : g.moveTo(X(t.rt[i]), Y(t.y[i]))); g.stroke(); g.lineWidth = 1;
    g.strokeStyle = mu; g.setLineDash([2, 3]); for (const v of thr) { g.beginPath(); g.moveTo(ml, Y(v) + .5); g.lineTo(W - ml, Y(v) + .5); g.stroke(); } g.setLineDash([]);
    g.strokeStyle = ink; g.beginPath(); g.moveTo(X(q.rt) + .5, mt); g.lineTo(X(q.rt) + .5, H - mb); g.stroke();
    g.fillStyle = mu; g.font = fpx(10); g.textAlign = "left"; g.fillText(lo.toFixed(2), ml, H - 3); g.textAlign = "right"; g.fillText(hi.toFixed(2), W - ml, H - 3);
  });
}

// ---------------------------------------------------------------- on and off
export function start(p) {
  const m = MAPPA.st(p); if (m.ex || !p._a || !p._a.map) return;
  m.ex = { box: defaultBox(p), gen: 0, t: 0, xic: null, spec: null, num: null, win: null }; clampBox(p, m.ex.box);
  MAPPA.layoutNow(p); MAPPA.redraw(p); table(p); load(p);
}
export function stop(p) {
  const m = MAPPA.st(p), s = m.ex; if (!s) return; clearTimeout(s.t); m.ex = null;
  if (m.exBox) { m.exBox.remove(); m.exBox = null; MAPPA.grow(p, "ex", 0); }
  p.cv.style.cursor = ""; MAPPA.layoutNow(p); MAPPA.redraw(p);
}
