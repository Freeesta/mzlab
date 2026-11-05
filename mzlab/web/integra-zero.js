// @ts-nocheck
// Manual integration window: the area under the trace between the two bars, cut vertically at each bar down to zero (no baseline, no neighbour
// correction), and its shaded region. Classic script, loaded before picchi.js; integ() in picchi.js calls it for records with zero:true.
const INTZERO = (() => {
// value of the trace at time t (linear between the two samples around it, the end samples outside the trace)
  function yAtT(/** @type {number[]} */ xs, /** @type {number[]} */ ys, /** @type {number} */ t) {
  const n = xs.length; if (t <= xs[0]) return ys[0]; if (t >= xs[n - 1]) return ys[n - 1];
  let lo = 0, hi = n - 1; while (hi - lo > 1) { const m = (lo + hi) >> 1; if (xs[m] <= t) lo = m; else hi = m; }
  return ys[lo] + (ys[hi] - ys[lo]) * (t - xs[lo]) / ((xs[hi] - xs[lo]) || 1);
}
// manual window: the area under the trace between the two bars, cut vertically at each bar down to zero (trapezoids, linear interpolation at the exact bar
// times); no baseline and no correction for a neighbour. The height is the highest point between the bars, the retention time the one of that point.
function area(/** @type {Bag} */ s, /** @type {number} */ a, /** @type {number} */ b) {
  const xs = s.x, ys = s.y, n = xs.length;
  const lo = Math.max(Math.min(a, b), xs[0]), hi = Math.min(Math.max(a, b), xs[n - 1]);
  if (!(hi > lo)) return { area: 0, height: 0, rt: lo };
  let ar = 0, px = lo, py = yAtT(xs, ys, lo), mx = py, at = lo;
  for (let j = 0; j < n; j++) {
    if (xs[j] <= lo) continue; if (xs[j] >= hi) break;
    ar += (py + ys[j]) / 2 * (xs[j] - px) * 60; px = xs[j]; py = ys[j];
    if (py > mx) { mx = py; at = px; }
  }
  const ye = yAtT(xs, ys, hi); ar += (py + ye) / 2 * (hi - px) * 60;
  if (ye > mx) { mx = ye; at = hi; }
  return { area: Math.max(ar, 0), height: Math.max(mx, 0), rt: at };
}

  // fills the region between the bars and zero (g, X/Y pixel maps, U unit map of the trace s)
  function draw(g, X, Y, U, s, it, chosen) {
    const lo = Math.max(Math.min(it.a, it.b), s.x[0]), hi = Math.min(Math.max(it.a, it.b), s.x[s.x.length - 1]); if (!(hi > lo)) return;
    const z = Y(U(s, 0)), at = t => Y(U(s, yAtT(s.x, s.ys, t)));
    g.globalAlpha = chosen ? 0.5 : 0.28; g.fillStyle = s.color; g.beginPath(); g.moveTo(X(lo), z); g.lineTo(X(lo), at(lo));
    for (let j = 0; j < s.x.length; j++) if (s.x[j] > lo && s.x[j] < hi) g.lineTo(X(s.x[j]), Y(U(s, s.ys[j])));
    g.lineTo(X(hi), at(hi)); g.lineTo(X(hi), z); g.closePath(); g.fill(); g.globalAlpha = 1;
    if (chosen) { g.strokeStyle = s.color; g.lineWidth = 2.5; g.stroke(); }
  }
  const clamp = (s, t) => Math.max(s.x[0], Math.min(s.x[s.x.length - 1], t));          // a bar time kept inside the trace
  return { area, draw, clamp, base: [0, 0, 1, 0] };                      // base = the zero line, for chromPeak
})();
