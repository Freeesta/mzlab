// @ts-check
// Numeric helpers shared by the drawing and list code: extremes of big (typed) arrays without spread, lower bound, and the index span of an x window.
// Loaded before grafici.js; used by picchi.js, liste.js, banco.js, conversione.js.
// Largest / smallest value of an array or typed array, seeded with `init` (the fallback of an empty one). A NaN (or a value that is not a number) gives NaN, as Math.max(...a) did;
// no spread, so a spectrum of 200 000 points cannot overflow the call stack (RangeError).
function maxOf(a, init = -Infinity) { let m = init; for (let i = 0; i < a.length; i++) { const v = +a[i]; if (v > m) m = v; else if (v !== v) return NaN; } return m; }
// Same, for the largest absolute value.
function maxAbs(a, init = 0) { let m = init; for (let i = 0; i < a.length; i++) { const v = Math.abs(a[i]); if (v > m) m = v; else if (v !== v) return NaN; } return m; }
function minOf(a, init = Infinity) { let m = init; for (let i = 0; i < a.length; i++) { const v = +a[i]; if (v < m) m = v; else if (v !== v) return NaN; } return m; }
// First index with a[i] >= v in an ascending array.
function lowerIdx(a, v) { let lo = 0, hi = a.length; while (lo < hi) { const m = (lo + hi) >> 1; if (a[m] < v) lo = m + 1; else hi = m; } return lo; }
const _ASC = new WeakMap();
// [i0, i1) that holds every index with lo <= a[i] <= hi: a binary search when `a` is ascending (checked once per array), else the whole array (the caller still tests the bounds).
function spanOf(a, lo, hi) {
  let asc = _ASC.get(a);
  if (asc === undefined) { asc = true; for (let i = 1; i < a.length; i++) if (a[i] < a[i - 1]) { asc = false; break; } _ASC.set(a, asc); }
  if (!asc) return [0, a.length];
  let i1 = lowerIdx(a, hi); while (i1 < a.length && a[i1] <= hi) i1++;
  return [lowerIdx(a, lo), i1];
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
