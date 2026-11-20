// @ts-strict
// @ts-noany
// «How an XIC and a spectrum are extracted» on the RT x m/z map (module, loaded the first time the view is turned on from mappa.js).
// The student places a box on the map (m/z window x RT window; drag it, drag its edges); beside the map the XIC of the m/z window (sum of the
// intensities with m/z inside the window, against RT) and the mean spectrum of the RT window are drawn from the SAME box, the integrated zone is
// coloured, dashed guides link the box to the axes of the two plots, and the numbers are written under the map. Nothing is labelled and nothing is
// found for the student: the position is always theirs (principle 1). The data are the ones of /api/xic and /api/spectrum.
/**
 * @typedef {{ rt0: number, rt1: number, mz0: number, mz1: number }} MapBox  the box of the student: an RT window (min) and an m/z window
 * @typedef {{ rt: number[], y: number[] }} MapXic  @typedef {{ mz: number[], y: number[] }} MapSpec
 * @typedef {{ area: number | null, height: number | null, rtMax: number | null, mzMax: number | null, nPts: number }} MapMeasure
 * @typedef {{ x: number, y: number, box: MapBox }} MapStart  where a drag began
 * @typedef {{ l?: boolean, r?: boolean, t?: boolean, b?: boolean, move?: boolean }} MapGrab  the edges (or the inside) of the box under the pointer
 * @typedef {{ box: MapBox, gen: number, t: number, xic: (MapXic & { c: number, tol: number }) | null, spec: (MapSpec & { rt0: number, rt1: number, n: number }) | null, num: MapMeasure | null,
 *   win: MapBox | null, k?: number }} MapEx  the state of the extraction (st(p).ex)
 */
/** @param {string} k @param {Record<string, any>} [o] */
const T = (k, o) => I18N.t(k, o);
const MIN_RT = 0.004;                                   // smallest RT window (min)
// smallest m/z window (Da): a few ppm in high resolution
const minMz = (/** @type {PannelloMappa} */ p) => window.HR && HR.isHr(p._a.f, p._a.f.lv || 1) ? 1e-4 : 0.05;
/** @param {PannelloMappa} p @returns {MapEx} */
const ex = p => MAPPA.st(p).ex;
const dp = (/** @type {PannelloMappa} */ p) => window.HR && HR.isHr(p._a.f, p._a.f.lv || 1) ? 4 : 2;
const num = (/** @type {number | null | undefined} */ v, /** @type {number} */ d) => v == null || !Number.isFinite(v) ? "–" : v.toFixed(d);

// ---------------------------------------------------------------- numbers (pure: tests/test_mappa_estrai.py runs the same rule in numpy through node)
export function measure(/** @type {MapXic | null} */ xic, /** @type {MapSpec | null} */ spec, /** @type {MapBox} */ w) {
  /** @type {MapMeasure} */ const out = { area: null, height: null, rtMax: null, mzMax: null, nPts: 0 };
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
function defaultBox(/** @type {PannelloMappa} */ p) {
  const a = p._a, f = a.f, q = MAPPA.point(p) || { rt: (a.x0 + a.x1) / 2, mz: (a.y0 + a.y1) / 2 };
  const hr = window.HR && HR.isHr(f, f.lv || 1), half = hr ? HR.tolDa(f, q.mz, f.lv || 1) : 0, win = hr ? [q.mz - half, q.mz + half] : xicWin(q.mz, true);
  const w = Math.max(4 * scanStep(), (a.x1 - a.x0) / 20);
  return { rt0: q.rt - w / 2, rt1: q.rt + w / 2, mz0: win[0], mz1: win[1] };
}
function clampBox(/** @type {PannelloMappa} */ p, /** @type {MapBox} */ b) {
  const a = p._a, MIN_MZ = minMz(p), lim = (/** @type {number} */ lo, /** @type {number} */ hi, /** @type {number} */ min, /** @type {number} */ f0, /** @type {number} */ f1) => { let w = Math.max(hi - lo, min); if (lo < f0) { lo = f0; hi = lo + w; } if (hi > f1) { hi = f1; lo = Math.max(f0, hi - w); } return [lo, hi]; };
  [b.rt0, b.rt1] = lim(b.rt0, b.rt1, MIN_RT, a.full[0], a.full[1]); [b.mz0, b.mz1] = lim(b.mz0, b.mz1, MIN_MZ, a.fullY[0], a.fullY[1]);
}
// pixel rectangle (y0 = top)
const px = (/** @type {PannelloMappa} */ p, /** @type {MapBox} */ b) => { const a = p._a; return { x0: a.X(b.rt0), x1: a.X(b.rt1), y0: a.Y(b.mz1), y1: a.Y(b.mz0) }; };
// what is at (x, y): an edge, a corner, the inside, or nothing
function grab(/** @type {PannelloMappa} */ p, /** @type {number} */ x, /** @type {number} */ y, /** @type {number} */ tol) {
  const b = ex(p).box, r = px(p, b);
  // a thin box is easy to hit: its hit area is at least 18 px
  const hh = Math.max(r.y1 - r.y0, 18), hw = Math.max(r.x1 - r.x0, 18), cy = (r.y0 + r.y1) / 2, cx = (r.x0 + r.x1) / 2;
  const y0 = cy - hh / 2, y1 = cy + hh / 2, x0 = cx - hw / 2, x1 = cx + hw / 2, ty = Math.min(tol, hh / 3), tx = Math.min(tol, hw / 3);
  if (x < x0 - tx || x > x1 + tx || y < y0 - ty || y > y1 + ty) return null;
  const L = Math.abs(x - x0) <= tx, R = Math.abs(x - x1) <= tx, Tp = Math.abs(y - y0) <= ty, B = Math.abs(y - y1) <= ty;
  if (L || R || Tp || B) return { l: L, r: R, t: Tp, b: B };
  return { move: true };
}
const cursorOf = (/** @type {MapGrab | null} */ h) => !h ? "" : h.move ? "move" : (h.l && h.t) || (h.r && h.b) ? "nwse-resize" : (h.r && h.t) || (h.l && h.b) ? "nesw-resize" : h.l || h.r ? "ew-resize" : "ns-resize";
function drag(/** @type {PannelloMappa} */ p, /** @type {MapGrab} */ h, /** @type {MapStart} */ start, /** @type {number} */ x, /** @type {number} */ y) {
  const a = p._a, s = ex(p), b = s.box, o = start.box, MIN_MZ = minMz(p), dRt = xOf(p, x) - xOf(p, start.x), dMz = a.mzAt(y) - a.mzAt(start.y);
  if (h.move) { b.rt0 = o.rt0 + dRt; b.rt1 = o.rt1 + dRt; b.mz0 = o.mz0 + dMz; b.mz1 = o.mz1 + dMz; }
  else { if (h.l) b.rt0 = Math.min(o.rt0 + dRt, o.rt1 - MIN_RT); if (h.r) b.rt1 = Math.max(o.rt1 + dRt, o.rt0 + MIN_RT); if (h.t) b.mz1 = Math.max(o.mz1 + dMz, o.mz0 + MIN_MZ); if (h.b) b.mz0 = Math.min(o.mz0 + dMz, o.mz1 - MIN_MZ); }
  clampBox(p, b); changed(p);
}
export function down(/** @type {PannelloMappa} */ p, /** @type {PointerEvent} */ e, /** @type {number} */ x, /** @type {number} */ y) {
  const s = ex(p); if (!s) return false;
  const h = grab(p, x, y, e.pointerType === "touch" ? 14 : 6); if (!h) return false;
  e.preventDefault(); const start = { x, y, box: { ...s.box } }, r = p.cv.getBoundingClientRect();
  const mv = (/** @type {MouseEvent} */ ev) => drag(p, h, start, ev.clientX - r.left, ev.clientY - r.top);
  const up = () => { removeEventListener("mousemove", mv); removeEventListener("mouseup", up); };
  addEventListener("mousemove", mv); addEventListener("mouseup", up); return true;
}
// a finger on the box moves it; returns the function that follows the finger, or null
export function touch(/** @type {PannelloMappa} */ p, /** @type {number} */ x, /** @type {number} */ y) {
  const s = ex(p); if (!s) return null; const h = grab(p, x, y, 16); if (!h) return null;
  const start = { x, y, box: { ...s.box } }; return (/** @type {number} */ nx, /** @type {number} */ ny) => drag(p, h, start, nx, ny);
}
export function hover(/** @type {PannelloMappa} */ p, /** @type {number} */ x, /** @type {number} */ y) { const s = ex(p); if (!s) return false; p.cv.style.cursor = cursorOf(grab(p, x, y, 6)); return true; }
// arrows move the box by one scan / one bin, Alt + arrows resize it
export function key(/** @type {PannelloMappa} */ p, /** @type {KeyboardEvent} */ e) {
  const s = ex(p), a = p._a; if (!s || !a || !["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(e.key)) return false;
  const MIN_MZ = minMz(p), b = s.box, dir = e.key === "ArrowRight" || e.key === "ArrowUp" ? 1 : -1, dt = scanStep(), dm = Math.max(MIN_MZ, (b.mz1 - b.mz0) / 4), horiz = e.key === "ArrowLeft" || e.key === "ArrowRight";
  if (e.altKey) { if (horiz) { b.rt0 -= dir * dt / 2; b.rt1 += dir * dt / 2; } else { b.mz0 -= dir * dm / 2; b.mz1 += dir * dm / 2; } }
  else if (horiz) { b.rt0 += dir * dt; b.rt1 += dir * dt; } else { b.mz0 += dir * dm; b.mz1 += dir * dm; }
  if (b.rt1 - b.rt0 < MIN_RT) { b.rt0 += (b.rt1 - b.rt0 - MIN_RT) / 2; b.rt1 = b.rt0 + MIN_RT; }
  if (b.mz1 - b.mz0 < MIN_MZ) { b.mz0 += (b.mz1 - b.mz0 - MIN_MZ) / 2; b.mz1 = b.mz0 + MIN_MZ; }
  clampBox(p, b); changed(p); return true;
}

// ---------------------------------------------------------------- data: the XIC of the m/z window and the mean spectrum of the RT window, from the API
function changed(/** @type {PannelloMappa} */ p) { const s = ex(p); s.gen++; MAPPA.redraw(p); clearTimeout(s.t); s.t = setTimeout(() => load(p), 90); }
async function load(/** @type {PannelloMappa} */ p) {
  const s = ex(p); if (!s || !p._a) return; const f = p._a.f, lv = f.lv || 1, b = { ...s.box }, gen = s.gen, c = (b.mz0 + b.mz1) / 2, tol = (b.mz1 - b.mz0) / 2;
  s.k = f.k;
  try {
    const [xj, sj] = await Promise.all([J(`api/xic?k=${f.k}&mz=${c}&tol=${tol}&level=${lv}`), J(`api/spectrum?k=${f.k}&rt0=${b.rt0}&rt1=${b.rt1}&level=${lv}&precursor=&bin=0.1`)]);
    // the box has moved again: the newest answer is on its way
    if (ex(p) !== s || gen !== s.gen) return;
    const tr = (xj.traces || [])[0];
    s.xic = tr ? { rt: tr.rt, y: tr.y, c, tol } : null;
    const m = MAPPA.st(p);
    s.spec = { mz: sj.mz, y: sj.y, rt0: b.rt0, rt1: b.rt1, n: sj.n }; s.win = b; s.num = measure(s.xic, s.spec, b);
    MAPPA.redraw(p); table(p);
  } catch (e) { /* nothing to show: the plots keep their title */ }
}
// read by the tests
export function numbers(/** @type {PannelloMappa} */ p) { const s = ex(p); return s && s.num ? { win: s.win, ...s.num } : null; }

// ---------------------------------------------------------------- drawing
export function overlay(/** @type {PannelloMappa} */ p, /** @type {CanvasRenderingContext2D} */ g) {
  const s = ex(p), a = p._a; if (!s || !a) return;
  const r = px(p, s.box), L = a.W - M.r, Bt = a.H - M.b, ac = css("--accent"), ink = css("--ink");
  g.save();
  // the guides: the edges of the box continue to the axes of the two plots
  g.setLineDash([5, 4]); g.strokeStyle = ac; g.lineWidth = 1; g.globalAlpha = .8;
  g.beginPath(); for (const x of [r.x0, r.x1]) { g.moveTo(Math.round(x) + .5, r.y1); g.lineTo(Math.round(x) + .5, Bt); }
  for (const y of [r.y0, r.y1]) { g.moveTo(r.x1, Math.round(y) + .5); g.lineTo(L, Math.round(y) + .5); } g.stroke(); g.setLineDash([]);
  g.globalAlpha = .14; g.fillStyle = ac; g.fillRect(r.x0, r.y0, r.x1 - r.x0, r.y1 - r.y0);
  g.globalAlpha = 1; g.lineWidth = 2; g.strokeStyle = ac; g.strokeRect(r.x0, r.y0, r.x1 - r.x0, r.y1 - r.y0);
  g.fillStyle = css("--panel"); g.lineWidth = 1.5; for (const [x, y] of [[r.x0, r.y0], [r.x1, r.y0], [r.x0, r.y1], [r.x1, r.y1]]) { g.fillRect(x - 4, y - 4, 8, 8); g.strokeRect(x - 4, y - 4, 8, 8); }
  const n = s.num;
  // where the maximum of the window is: the two coordinates go to the axes
  if (n && n.rtMax != null && n.mzMax != null) {
    const x = a.X(n.rtMax), y = a.Y(n.mzMax); g.strokeStyle = ink; g.lineWidth = 1; g.setLineDash([2, 3]);
    g.beginPath(); g.moveTo(x + .5, y); g.lineTo(x + .5, Bt); g.moveTo(x, y + .5); g.lineTo(L, y + .5); g.stroke(); g.setLineDash([]);
    g.fillStyle = ink; g.beginPath(); g.arc(x, y, 3, 0, 2 * Math.PI); g.fill();
  }
  g.restore();
}
function plotArea(/** @type {PannelloMappa} */ p, /** @type {HTMLCanvasElement} */ c) {
  const { g, W, H } = MAPMARGINI.sized(c, devicePixelRatio);
  return { g: /** @type {CanvasRenderingContext2D} */ (g), W, H, a: p._a };
}
export function margins(/** @type {PannelloMappa} */ p) {
  const s = ex(p); if (!s || !p._a) return; const m = MAPPA.st(p);
  // another file: the plots are asked again for the same box
  if (s.k !== p._a.f.k) { s.k = p._a.f.k; load(p); }
  if (!m.xc.hidden) drawXic(p, m.xc, s);
  if (!m.sp.hidden) drawSpec(p, m.sp, s);
}
// the same RT axis as the map; the integrated zone is filled
function drawXic(/** @type {PannelloMappa} */ p, /** @type {HTMLCanvasElement} */ c, /** @type {MapEx} */ s) {
  const { g, W, H, a } = plotArea(p, c), mt = 10, mb = 22, ink = css("--muted"), ac = css("--accent");
  const o = MAPMARGINI.rel(p.cv, c);
  const ml = o.x + M.l, mr = o.x + p.cv.clientWidth - M.r, pw = mr - ml;
  const x = (/** @type {number} */ v) => ml + (v - a.x0) / (a.x1 - a.x0) * pw;
  g.font = fpx(11); g.fillStyle = ink; g.strokeStyle = ink; g.beginPath(); g.moveTo(ml, H - mb + .5); g.lineTo(mr, H - mb + .5); g.stroke();
  g.textAlign = "center"; for (const t of nice(a.x0, a.x1, 8)) { const xx = x(t); if (xx >= ml - 1 && xx <= mr + 1) g.fillText(String(+t.toFixed(2)), xx, H - mb + 13); }
  g.textAlign = "left"; const b = s.box, xa = x(b.rt0), xb = x(b.rt1);
  // the RT window of the box, for the eye
  g.globalAlpha = .14; g.fillStyle = ac; g.fillRect(xa, mt, xb - xa, H - mt - mb); g.globalAlpha = 1;
  const t = s.xic; if (!t || !t.rt.length) { g.fillStyle = css("--ink"); g.fillText(T("mapex.xic.title"), ml + 4, mt + 8); guides(g, xa, xb, mt, H - mb, ac); return; }
  let ymax = 0; for (let i = 0; i < t.rt.length; i++) if (t.rt[i] >= a.x0 && t.rt[i] <= a.x1 && t.y[i] > ymax) ymax = t.y[i];
  const y = (/** @type {number} */ v) => H - mb - v / (ymax || 1) * (H - mt - mb);
  // the filled area: between the two edges of the box
  g.beginPath(); let first = true, started = false;
  const pts = []; for (let i = 0; i < t.rt.length; i++) if (t.rt[i] >= a.x0 && t.rt[i] <= a.x1) pts.push(i);
  g.fillStyle = ac; g.globalAlpha = .45;
  const inside = pts.filter(i => t.rt[i] >= b.rt0 && t.rt[i] <= b.rt1);
  if (inside.length) { g.beginPath(); g.moveTo(x(t.rt[inside[0]]), y(0)); for (const i of inside) g.lineTo(x(t.rt[i]), y(t.y[i])); g.lineTo(x(t.rt[inside[inside.length - 1]]), y(0)); g.closePath(); g.fill(); }
  g.globalAlpha = 1; g.strokeStyle = a.f.color || css("--ink"); g.lineWidth = 1.3; g.beginPath(); for (const i of pts) { const xx = x(t.rt[i]), yy = y(t.y[i]); if (first) { g.moveTo(xx, yy); first = false; } else g.lineTo(xx, yy); } g.stroke(); g.lineWidth = 1;
  guides(g, xa, xb, mt, H - mb, ac);
  g.fillStyle = css("--ink"); g.fillText(`XIC ${num(s.win ? s.win.mz0 : b.mz0, 2)}–${num(s.win ? s.win.mz1 : b.mz1, 2)} · max ${fmt(ymax)}`, ml + 4, mt + 8);
  if (s.num && s.num.rtMax != null && s.num.rtMax >= a.x0 && s.num.rtMax <= a.x1) { g.strokeStyle = css("--ink"); g.setLineDash([2, 3]); g.beginPath(); g.moveTo(x(s.num.rtMax) + .5, mt); g.lineTo(x(s.num.rtMax) + .5, H - mb); g.stroke(); g.setLineDash([]); }
}
const guides = (/** @type {CanvasRenderingContext2D} */ g, /** @type {number} */ xa, /** @type {number} */ xb, /** @type {number} */ y0, /** @type {number} */ y1, /** @type {string} */ col) => { g.save(); g.strokeStyle = col; g.setLineDash([5, 4]); g.beginPath(); for (const xx of [xa, xb]) { g.moveTo(Math.round(xx) + .5, y0); g.lineTo(Math.round(xx) + .5, y1); } g.stroke(); g.restore(); };
// the same m/z axis as the map, turned on its side; the m/z window is a band
function drawSpec(/** @type {PannelloMappa} */ p, /** @type {HTMLCanvasElement} */ c, /** @type {MapEx} */ s) {
  const { g, W, H, a } = plotArea(p, c), ink = css("--muted"), ac = css("--accent"), x0 = 6, x1 = W - 8, b = s.box;
  const o = MAPMARGINI.rel(p.cv, c);
  const mapTop = o.y + M.t, mapBot = o.y + p.cv.clientHeight - M.b, mapH = mapBot - mapTop;
  const Y = (/** @type {number} */ v) => mapBot - (v - a.y0) / (a.y1 - a.y0) * mapH;
  g.font = fpx(11); g.fillStyle = ink; g.strokeStyle = ink; g.beginPath(); g.moveTo(x0 + .5, mapTop); g.lineTo(x0 + .5, mapBot); g.stroke();
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
function table(/** @type {PannelloMappa} */ p) {
  const s = ex(p), m = MAPPA.st(p); if (!s || !m.grid) return; const hr = window.HR && HR.isHr(p._a.f, p._a.f.lv || 1), d = hr ? 4 : 2;
  let box = m.exBox; if (!box) { box = m.exBox = document.createElement("div"); box.className = "mzex"; box.setAttribute("role", "region"); box.setAttribute("aria-label", T("mapex.region")); (m.tab || m.grid).after(box); }
  const n = s.num, w = s.win || s.box, row = (/** @type {string} */ k, /** @type {string} */ lab, /** @type {string} */ v, /** @type {string} */ u) => `<div class="mzex-r"><dt>${T(lab)}</dt><dd><output data-k="${k}" aria-label="${T(lab)}">${v}</output>${u ? ` <span class="muted">${u}</span>` : ""}</dd></div>`;
  box.innerHTML = `<p class="mzex-d">${T("mapex.intro")}</p><dl class="mzex-n">` +
    row("mz", "mapex.n.mz", `${num(w.mz0, d)} – ${num(w.mz1, d)}`, `(± ${num((w.mz1 - w.mz0) / 2, d)})`) + row("rt", "mapex.n.rt", `${num(w.rt0, 3)} – ${num(w.rt1, 3)}`, "min") +
    row("area", "mapex.n.area", n && n.area != null ? I18N.sciE(n.area) : "–", T("mapex.u.area")) + row("height", "mapex.n.height", n && n.height != null ? I18N.sciE(n.height) : "–", T("mapex.u.height")) +
    row("rtmax", "mapex.n.rtmax", n ? num(n.rtMax, 3) : "–", "min") + row("mzmax", "mapex.n.mzmax", n ? num(n.mzMax, d) : "–", "") + `</dl>` +
    `<p class="mzex-f"><b>XIC(t)</b> = Σ ${T("mapex.f.xic")}<br><b>S(m/z)</b> = ${T("mapex.f.spec")}<br>${T("mapex.f.area")}</p>` +
    `<p class="mzex-b"><button type="button" data-ex="off">${T("mapex.close")}</button></p>`;
  box.querySelector('[data-ex="off"]').onclick = () => MAPPA.explain(p, false);
  MAPPA.grow(p, "ex", Math.ceil(box.offsetHeight) + 4);
}

// ---------------------------------------------------------------- on and off
export function start(/** @type {PannelloMappa} */ p) {
  const m = MAPPA.st(p); if (m.ex || !p._a || !p._a.map) return;
  m.ex = { box: defaultBox(p), gen: 0, t: 0, xic: null, spec: null, num: null, win: null }; clampBox(p, m.ex.box);
  MAPPA.layoutNow(p); MAPPA.redraw(p); table(p); load(p);
}
export function stop(/** @type {PannelloMappa} */ p) {
  const m = MAPPA.st(p), s = m.ex; if (!s) return; clearTimeout(s.t); m.ex = null;
  if (m.exBox) { m.exBox.remove(); m.exBox = null; MAPPA.grow(p, "ex", 0); }
  p.cv.style.cursor = ""; MAPPA.layoutNow(p); MAPPA.redraw(p);
}
