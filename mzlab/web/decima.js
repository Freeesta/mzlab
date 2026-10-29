// @ts-check
// @ts-strict
// Pure helpers of the drawing loop of the chromatograms (grafici.js drawLines): the window of a zoomed trace by binary search, the four vertices per pixel column
// (first, lowest, highest, last: no apex is lost) read without calling X/Y per point, and max/min of a range without a spread (which throws RangeError on 10^6 values).

/** @typedef {ArrayLike<number>} Vec */

/** @type {WeakMap<object, boolean>} */
const SORTED = new WeakMap();

/** Whether x never decreases (cached per array): the window search needs it, any other trace falls back to the full scan. @param {Vec} x */
export function isSorted(x) {
  const o = /** @type {object} */ (x);
  let r = SORTED.get(o);
  if (r === undefined) { r = true; for (let i = 1; i < x.length; i++) if (x[i] < x[i - 1]) { r = false; break; } SORTED.set(o, r); }
  return r;
}

/** Indices [i0, i1) of the points with lo <= x <= hi, by binary search (x sorted). @param {Vec} x @param {number} lo @param {number} hi @returns {[number, number]} */
export function windowIdx(x, lo, hi) {
  let a = 0, b = x.length;
  while (a < b) { const m = (a + b) >> 1; if (x[m] < lo) a = m + 1; else b = m; }
  const i0 = a; b = x.length;
  while (a < b) { const m = (a + b) >> 1; if (x[m] <= hi) a = m + 1; else b = m; }
  return [i0, a];
}

/** Highest value of a[i0..i1) above `init` (NaN is ignored, as with `>`). @param {Vec} a @param {number} [i0] @param {number} [i1] @param {number} [init] */
export function amax(a, i0 = 0, i1 = a.length, init = -Infinity) {
  let m = init;
  for (let i = i0; i < i1; i++) { const v = a[i]; if (v > m) m = v; }
  return m;
}

/** Lowest value of a[i0..i1) below `init` (NaN is ignored, as with `<`). @param {Vec} a @param {number} [i0] @param {number} [i1] @param {number} [init] */
export function amin(a, i0 = 0, i1 = a.length, init = Infinity) {
  let m = init;
  for (let i = i0; i < i1; i++) { const v = a[i]; if (v < m) m = v; }
  return m;
}

/**
 * The polyline of a trace, one pixel column at a time: for each column the vertices first, lowest, highest, last (the lowest and the highest in the order in which the
 * data reach them first), as flat [px, py, px, py, ...]. Points are grouped by Math.round(X(x)) exactly as the point-by-point loop did; the end of a column is found by
 * galloping, the extremes are compared as values (V is monotonic: the pixel of a higher value is higher) and V runs 4 times per column, not once per point.
 * @param {Vec} x sorted
 * @param {Vec} y
 * @param {number} i0 first point of the window
 * @param {number} i1 end of the window
 * @param {(v: number) => number} X time -> pixel
 * @param {(v: number) => number} V value -> pixel (up is smaller)
 * @returns {number[]}
 */
export function m4(x, y, i0, i1, X, V) {
  /** @type {number[]} */
  const out = [];
  let i = i0;
  while (i < i1) {
    const cx = Math.round(X(x[i]));
    let lo = i, step = 1, hi = i + 1;
    while (hi < i1 && Math.round(X(x[hi])) === cx) { lo = hi; step *= 2; hi = Math.min(i1, hi + step); }
    let a = lo + 1, b = hi;
    while (a < b) { const m = (a + b) >> 1; if (Math.round(X(x[m])) === cx) a = m + 1; else b = m; }
    const end = a, vFirst = y[i], first = V(vFirst);
    let vTop = vFirst, vBot = vFirst, minY = first, maxY = first, minFirst = true;
    for (let k = i + 1; k < end; k++) {
      const v = y[k];
      if (v > vTop) { const py = V(v); if (py < minY) { vTop = v; minY = py; if (maxY === first) minFirst = true; } }
      if (v < vBot) { const py = V(v); if (py > maxY) { vBot = v; maxY = py; if (minY === first) minFirst = false; } }
    }
    const last = V(y[end - 1]);
    out.push(cx, first, cx, minFirst ? minY : maxY, cx, minFirst ? maxY : minY, cx, last);
    i = end;
  }
  return out;
}

/**
 * Highest and lowest value of y over the points with lo <= x <= hi, starting from the given bounds (any x order; the window search is used when x is sorted).
 * @param {Vec} x @param {Vec} y @param {number} lo @param {number} hi @param {number} [mx] @param {number} [mn] @returns {[number, number]}
 */
export function extent(x, y, lo, hi, mx = -Infinity, mn = Infinity) {
  if (isSorted(x)) { const [i0, i1] = windowIdx(x, lo, hi); return [amax(y, i0, i1, mx), amin(y, i0, i1, mn)]; }
  return extentScan(x, y, lo, hi, mx, mn);
}

/** extent() point by point, for any x order. @param {Vec} x @param {Vec} y @param {number} lo @param {number} hi @param {number} mx @param {number} mn @returns {[number, number]} */
export function extentScan(x, y, lo, hi, mx, mn) {
  for (let i = 0; i < x.length; i++) if (x[i] >= lo && x[i] <= hi) { if (y[i] > mx) mx = y[i]; if (y[i] < mn) mn = y[i]; }
  return [mx, mn];
}

/**
 * The polyline of a trace in the window [x0, x1] as flat [px, py, ...]: m4 when x is sorted, else the point-by-point loop (columns of consecutive points only).
 * @param {Vec} x @param {Vec} y @param {number} x0 @param {number} x1 @param {(v: number) => number} X @param {(v: number) => number} V @returns {number[]}
 */
export function polyline(x, y, x0, x1, X, V) {
  if (isSorted(x)) { const [i0, i1] = windowIdx(x, x0, x1); return m4(x, y, i0, i1, X, V); }
  return polylineScan(x, y, x0, x1, X, V);
}

/** polyline() point by point, for any x order. @param {Vec} x @param {Vec} y @param {number} x0 @param {number} x1 @param {(v: number) => number} X @param {(v: number) => number} V @returns {number[]} */
export function polylineScan(x, y, x0, x1, X, V) {
  /** @type {number[]} */
  const out = [];
  let curX = -999, minY = 0, maxY = 0, firstY = 0, lastY = 0, minFirst = true;
  const flush = (/** @type {number} */ cx) => { if (curX === -999) return; out.push(cx, firstY); if (minFirst) out.push(cx, minY, cx, maxY); else out.push(cx, maxY, cx, minY); out.push(cx, lastY); };
  for (let i = 0; i < x.length; i++) {
    const r = x[i]; if (r < x0 || r > x1) continue;
    const px = Math.round(X(r)), py = V(y[i]);
    if (px !== curX) { flush(curX); curX = px; minY = maxY = firstY = lastY = py; minFirst = true; }
    else { if (py < minY) { minY = py; if (maxY === firstY) minFirst = true; } if (py > maxY) { maxY = py; if (minY === firstY) minFirst = false; } lastY = py; }
  }
  flush(curX);
  return out;
}
