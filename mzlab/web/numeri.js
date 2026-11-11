// @ts-check
// Numeric helpers shared by the drawing and list code: extremes of big (typed) arrays without spread, lower bound, the index span of an x window and the tallest peak of each pixel column.
// Loaded before grafici.js; used by picchi.js, liste.js, banco.js, conversione.js, tdati-pannelli.js.
// Largest / smallest value of an array or typed array, seeded with `init` (the fallback of an empty one). A NaN (or a value that is not a number) gives NaN, as Math.max(...a) did;
// no spread, so a spectrum of 200 000 points cannot overflow the call stack (RangeError).
function maxOf(a, init = -Infinity) { let m = init; for (let i = 0; i < a.length; i++) { const v = +a[i]; if (v > m) m = v; else if (v !== v) return NaN; } return m; }
// Same, for the largest absolute value.
function maxAbs(a, init = 0) { let m = init; for (let i = 0; i < a.length; i++) { const v = Math.abs(a[i]); if (v > m) m = v; else if (v !== v) return NaN; } return m; }
// maxOf(a) and maxAbs(a, 1e-9) kept per array: the top of a spectrum is the same at every drawing (a summed HR run: 730 000 peaks, one pass each per drawing)
const _TOP = new WeakMap(), _TOPA = new WeakMap();
function topOf(a) { let v = _TOP.get(a); if (v === undefined) { v = maxOf(a); _TOP.set(a, v); } return v; }
function topAbs(a) { let v = _TOPA.get(a); if (v === undefined) { v = maxAbs(a, 1e-9); _TOPA.set(a, v); } return v; }
// The largest y whose x lies in [lo, hi] (-Infinity if none), kept per array for its last window: every redraw of the same view asks it again (730 000 peaks: one pass each)
const _WTOP = new WeakMap();
function winTop(xs, ys, lo, hi) {
  const c = _WTOP.get(ys); if (c && c.xs === xs && c.lo === lo && c.hi === hi) return c.v;
  let v = -Infinity; const [i0, i1] = spanOf(xs, lo, hi); for (let i = i0; i < i1 && i < ys.length; i++) if (xs[i] >= lo && xs[i] <= hi && ys[i] > v) v = ys[i];
  _WTOP.set(ys, { xs, lo, hi, v }); return v;
}
function minOf(a, init = Infinity) { let m = init; for (let i = 0; i < a.length; i++) { const v = +a[i]; if (v < m) m = v; else if (v !== v) return NaN; } return m; }
// First index with a[i] >= v in an ascending array.
function lowerIdx(a, v) { let lo = 0, hi = a.length; while (lo < hi) { const m = (lo + hi) >> 1; if (a[m] < v) lo = m + 1; else hi = m; } return lo; }
// Index of the value of an ascending array nearest to v (ties go to the upper one).
function nearIdx(xs, v) { let lo = 0, hi = xs.length - 1; if (hi < 1) return 0; while (hi - lo > 1) { const m = (lo + hi) >> 1; xs[m] < v ? lo = m : hi = m; } return Math.abs(xs[lo] - v) < Math.abs(xs[hi] - v) ? lo : hi; }
const _ASC = new WeakMap();
// [i0, i1) that holds every index with lo <= a[i] <= hi: a binary search when `a` is ascending (checked once per array), else the whole array (the caller still tests the bounds).
function spanOf(a, lo, hi) {
  let asc = _ASC.get(a);
  if (asc === undefined) { asc = true; for (let i = 1; i < a.length; i++) if (a[i] < a[i - 1]) { asc = false; break; } _ASC.set(a, asc); }
  if (!asc) return [0, a.length];
  let i1 = lowerIdx(a, hi); while (i1 < a.length && a[i1] <= hi) i1++;
  return [lowerIdx(a, lo), i1];
}
const TALL = new WeakMap();
// The tallest peak of each pixel column of the window (indices of d.mz), for a spectrum with many more peaks than pixels. Kept per spectrum and
// `key` (window, width, margins): a resize step or a redraw makes no pass over the peaks. The hidden contaminants (`hid`) go to `dim`; with them nothing is kept.
function tallCols(/** @type {any} */ d, /** @type {number} */ i0, /** @type {number} */ i1, /** @type {(m: number) => number} */ X, /** @type {number} */ x0, /** @type {number} */ x1, /** @type {string} */ key, /** @type {any} */ hid) {
  const c = !hid && TALL.get(d); if (c && c.key === key) return c;
  const /** @type {number[]} */ idx = [], /** @type {number[]} */ dim = []; let cc = NaN, cj = -1;
  for (let j = i0; j < i1; j++) { const m = d.mz[j]; if (m < x0 || m > x1) continue; if (hid && hid(j)) { dim.push(j); continue; } const col = Math.floor(X(m)); if (col !== cc) { if (cj >= 0) idx.push(cj); cc = col; cj = j; } else if (d.y[j] > d.y[cj]) cj = j; }
  if (cj >= 0) idx.push(cj);
  const r = { key, idx, dim }; if (!hid) TALL.set(d, r); return r;
}
// The float32 array packed in a base64 string (the map routes). A payload that is not base64, or whose size is not a whole number of float32 (or not `n` of them), throws a plain Error.
function b64F32(str, n) {
  let bin; try { bin = atob(str); } catch (e) { throw new Error("corrupt map payload: not base64"); }
  if (bin.length % 4) throw new Error("corrupt map payload: length " + bin.length + " is not a multiple of 4");
  const u = new Uint8Array(bin.length); for (let i = 0; i < bin.length; i++) u[i] = bin.charCodeAt(i);
  const m = new Float32Array(u.buffer);
  if (n != null && m.length !== n) throw new Error("corrupt map payload: " + m.length + " values, expected " + n);
  return m;
}
if (typeof module !== "undefined") module.exports = { nearIdx };
