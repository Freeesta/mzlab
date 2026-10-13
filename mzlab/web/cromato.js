// @ts-check
// @ts-strict
"use strict";
// Chromatographic figures of a peak (block H): S/N, width at half height, plates, tailing. Pure functions (also run by node in tests/test_cromato.py):
// the program computes, the student reads. Same baseline as the integration (straight line between the two edges, ends = mean of 3 points).
const cpNear = (xs, v) => { let lo = 0, hi = xs.length - 1; if (hi < 1) return 0; while (hi - lo > 1) { const m = (lo + hi) >> 1; xs[m] < v ? lo = m : hi = m; } return Math.abs(xs[lo] - v) < Math.abs(xs[hi] - v) ? lo : hi; };

// x (time) where the baseline-corrected signal crosses `level` going away from the apex; dir = -1 (front) or +1 (back); null if it never does inside [i0, i1]
function cpCross(xs, c, ap, i0, i1, level, dir) {
  for (let j = ap; dir < 0 ? j > i0 : j < i1; j += dir) {
    const k = j + dir;
    if (c[k] <= level) { const f = (c[j] - level) / ((c[j] - c[k]) || 1); return xs[j] + (xs[k] - xs[j]) * f; }
  }
  return null;
}
// peak between the edges a and b (min): height above the baseline, apex time, width at half height (min), plates N = 5.54 (tR / w1/2)^2, USP tailing at 5% of the height
function chromPeak(xs, ys, a, b) {
  const i0 = cpNear(xs, Math.min(a, b)), i1 = cpNear(xs, Math.max(a, b));
  if (i1 - i0 < 2) return null;
  const e3 = i => { let t = 0, n = 0; for (let j = i - 1; j <= i + 1; j++) if (j >= 0 && j < ys.length) { t += ys[j]; n++; } return t / n; };
  const ya = e3(i0), yb = e3(i1), base = x => ya + (yb - ya) * (x - xs[i0]) / ((xs[i1] - xs[i0]) || 1);
  const c = ys.map((v, j) => v - base(xs[j]));
  let ap = i0; for (let j = i0; j <= i1; j++) if (c[j] > c[ap]) ap = j;
  const h = c[ap]; if (!(h > 0)) return null;
  const at = f => [cpCross(xs, c, ap, i0, i1, h * f, -1), cpCross(xs, c, ap, i0, i1, h * f, 1)];
  const [l50, r50] = at(0.5), [l05, r05] = at(0.05), tR = xs[ap];
  const w50 = l50 != null && r50 != null ? r50 - l50 : null;
  return { height: h, rt: tR, w50, N: w50 ? 5.54 * (tR / w50) ** 2 : null, tailing: l05 != null && r05 != null && tR > l05 ? (r05 - l05) / (2 * (tR - l05)) : null };
}
// noise = standard deviation (n - 1) of the signal around a straight line fitted in the stretch [a, b] chosen by the student (a drift is not noise)
function noiseSD(xs, ys, a, b) {
  const lo = Math.min(a, b), hi = Math.max(a, b), px = [], py = [];
  xs.forEach((x, j) => { if (x >= lo && x <= hi) { px.push(x); py.push(ys[j]); } });
  const n = px.length; if (n < 5) return null;
  const mx = px.reduce((s, v) => s + v, 0) / n, my = py.reduce((s, v) => s + v, 0) / n;
  let sxx = 0, sxy = 0; px.forEach((x, j) => { sxx += (x - mx) ** 2; sxy += (x - mx) * (py[j] - my); });
  const m = sxx ? sxy / sxx : 0;
  return Math.sqrt(py.reduce((s, y, j) => s + (y - (my + m * (px[j] - mx))) ** 2, 0) / (n - 1));
}
if (typeof module !== "undefined") module.exports = { chromPeak, noiseSD };
