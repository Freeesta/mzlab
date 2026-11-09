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

// Highest and lowest value of every block of BLK points, built once per array (and again if its length changes): a range query reads the blocks it covers whole and scans
// only its two ends, so 120 traces of 50 000 scans cost ~1/60 of a full scan per drawing. The result is the same value as amax/amin over the range.
const BLK = 128;
/** @type {WeakMap<object, { n: number, mx: Float64Array, mn: Float64Array }>} */
const BLOCKS = new WeakMap();
/** @param {Vec} a */
function blocks(a) {
  const o = /** @type {object} */ (a);
  let b = BLOCKS.get(o);
  if (!b || b.n !== a.length) {
    const nb = Math.ceil(a.length / BLK), mx = new Float64Array(nb), mn = new Float64Array(nb);
    for (let k = 0; k < nb; k++) { const i0 = k * BLK, i1 = Math.min(a.length, i0 + BLK); mx[k] = amax(a, i0, i1); mn[k] = amin(a, i0, i1); }
    b = { n: a.length, mx, mn }; BLOCKS.set(o, b);
  }
  return b;
}
/** amax/amin of a[i0..i1) through the blocks (short ranges are scanned). @param {Vec} a @param {number} i0 @param {number} i1 @param {number} mx @param {number} mn @returns {[number, number]} */
export function rangeExtent(a, i0, i1, mx = -Infinity, mn = Infinity) {
  if (i1 - i0 < 4 * BLK) return [amax(a, i0, i1, mx), amin(a, i0, i1, mn)];
  const b = blocks(a), k0 = Math.ceil(i0 / BLK), k1 = Math.floor(i1 / BLK), e0 = k0 * BLK, e1 = k1 * BLK;
  mx = amax(a, e1, i1, amax(a, i0, e0, mx)); mn = amin(a, e1, i1, amin(a, i0, e0, mn));
  return [amax(b.mx, k0, k1, mx), amin(b.mn, k0, k1, mn)];
}

/**
 * The runs of consecutive points of y[i0..i1) whose f(y) is above lim (a point that is not, NaN included, ends a run): for each run the index of its highest point, the
 * first at that height. A block whose highest value is not above lim ends the run at its first point and is skipped whole (f is increasing, as the chart units are).
 * @param {Vec} y @param {number} i0 @param {number} i1 @param {(v: number) => number} f @param {number} lim @returns {number[]}
 */
export function runsAbove(y, i0, i1, f, lim) {
  /** @type {number[]} */
  const out = [], b = i1 - i0 >= 4 * BLK ? blocks(y) : null;
  let best = -1, bv = 0;
  for (let i = i0; i < i1;) {
    if (b && i % BLK === 0 && i + BLK <= i1 && !(f(b.mx[i / BLK]) > lim)) { if (best >= 0) { out.push(best); best = -1; } i += BLK; continue; }
    const v = f(y[i]);
    if (v > lim) { if (best < 0 || v > bv) { best = i; bv = v; } } else if (best >= 0) { out.push(best); best = -1; }
    i++;
  }
  if (best >= 0) out.push(best);
  return out;
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
  if (isSorted(x)) { const [i0, i1] = windowIdx(x, lo, hi); return rangeExtent(y, i0, i1, mx, mn); }
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

/** Points per pixel column above which a line is drawn 1 px wide: a wider stroke of a dense, jagged line (noise) costs tens of times more in a software canvas. */
export const HAIR = 0.5;

/** Whether the line of the points with x0 <= x <= x1 is drawn 1 px wide (more than HAIR points per pixel column; an unsorted x keeps the normal width). @param {Vec} x @param {number} x0 @param {number} x1 @param {(v: number) => number} X */
export function hairline(x, x0, x1, X) {
  if (!isSorted(x)) return false;
  const [i0, i1] = windowIdx(x, x0, x1);
  return i1 - i0 > Math.abs(X(x1) - X(x0)) * HAIR;
}

/**
 * Raw (unsmoothed) traces with more than `ppx` points per pixel: the highest value of each pixel column, joined by a thin line (peak envelope: no apex is lost, and the
 * line stays a line where first/lowest/highest/last per column fills the whole plot with a solid band). Fewer points per pixel (a zoom) or an unsorted x: polyline().
 * @param {Vec} x @param {Vec} y @param {number} x0 @param {number} x1 @param {(v: number) => number} X @param {(v: number) => number} V @param {number} [ppx]
 * @returns {{ v: number[], thin: boolean }}
 */
export function rawLine(x, y, x0, x1, X, V, ppx = 1.5) {
  if (!isSorted(x)) return { v: polylineScan(x, y, x0, x1, X, V), thin: false };
  const [i0, i1] = windowIdx(x, x0, x1), cols = Math.abs(X(x1) - X(x0));
  if (i1 - i0 <= cols * ppx) return { v: m4(x, y, i0, i1, X, V), thin: false };
  /** @type {number[]} */
  const out = [];
  let cx = Math.round(X(x[i0])), top = y[i0];
  for (let i = i0 + 1; i < i1; i++) {
    const c = Math.round(X(x[i])), v = y[i];
    if (c !== cx) { out.push(cx, V(top)); cx = c; top = v; } else if (v > top) top = v;
  }
  out.push(cx, V(top));
  return { v: out, thin: true };
}

/**
 * The polyline v (x, y pairs) with every stretch beyond lo..hi moved onto that bound, cut exactly where it crosses it: the part inside is unchanged. A y zoom puts
 * the peaks thousands of plot heights above the canvas, and a software canvas pays for the whole length of those strokes (0.3 s per draw for 20 traces).
 * @param {number[]} v @param {number} lo @param {number} hi @returns {number[]}
 */
export function clipY(v, lo, hi) {
  let i = 1;
  while (i < v.length && v[i] >= lo && v[i] <= hi) i += 2;
  if (i >= v.length) return v;
  const cl = (/** @type {number} */ y) => y < lo ? lo : y > hi ? hi : y, out = [v[0], cl(v[1])];
  for (let k = 2; k < v.length; k += 2) {                 // the crossings of the two bounds in the order the segment meets them; no array per segment (120 traces)
    const xa = v[k - 2], ya = v[k - 1], xb = v[k], yb = v[k + 1], cLo = (ya - lo) * (yb - lo) < 0, cHi = (ya - hi) * (yb - hi) < 0;
    if (cLo || cHi) {
      const tl = (lo - ya) / (yb - ya), th = (hi - ya) / (yb - ya);
      if (cLo && (!cHi || tl <= th)) out.push(xa + (xb - xa) * tl, ya + (yb - ya) * tl);
      if (cHi) out.push(xa + (xb - xa) * th, ya + (yb - ya) * th);
      if (cLo && cHi && tl > th) out.push(xa + (xb - xa) * tl, ya + (yb - ya) * tl);
    }
    out.push(xb, cl(yb));
  }
  return out;
}
