// @ts-strict
// @ts-noany
// @ts-check
// Memo cache of the Dati requests (CACHE, memo) with a byte budget: the answers used least recently leave first and are asked again if needed. Classic script, loaded before explore.js; tested in Node (tests/test_perf_web6.py).
const CACHE = new Map();
const CACHE_BUD = { max: ((/** @type {any} */ (navigator)).deviceMemory || 4) <= 2 ? 96e6 : 256e6, size: new WeakMap() };
/** Estimated bytes of an answer: typed arrays by size, arrays of numbers 8 per value, objects by their fields. @param {any} v @returns {number} */
const cacheBytes = (v, d = 0) => {
  if (!v || typeof v !== "object") return 16;
  if (ArrayBuffer.isView(v)) return v.byteLength;
  if (Array.isArray(v)) { if (!v.length || typeof v[0] !== "object" || d > 2) return 8 * v.length; let n = 0; for (const e of v) n += cacheBytes(e, d + 1); return n; }
  let n = 64; for (const k in v) n += cacheBytes(v[k], d + 1);
  return n;
};
const cacheAccount = (/** @type {string} */ key, /** @type {Promise<unknown>} */ pr, /** @type {unknown} */ v) => {
  if (CACHE.get(key) !== pr) return;
  CACHE_BUD.size.set(pr, cacheBytes(v));
  let total = 0; for (const e of CACHE.values()) total += CACHE_BUD.size.get(e) || 0;
  for (const [k, e] of CACHE) { if (total <= CACHE_BUD.max || k === key) break; if (CACHE_BUD.size.has(e)) { total -= CACHE_BUD.size.get(e) || 0; CACHE.delete(k); } }
};
const memo = (/** @type {string} */ key, /** @type {() => Promise<unknown>} */ fn) => {
  if (CACHE.has(key)) { const hit = CACHE.get(key); CACHE.delete(key); CACHE.set(key, hit); return hit; }        // a hit becomes the newest
  const pr = fn().catch((/** @type {unknown} */ e) => { if (CACHE.get(key) === pr) CACHE.delete(key); throw e; });
  CACHE.set(key, pr); pr.then(v => cacheAccount(key, pr, v), () => {});
  return pr;
};
// a plain Array from doubles: the consumers push, map and stringify it; the loop is several times faster than Array.from on a typed array
const f64Arr = (/** @type {Float64Array} */ a) => { const o = new Array(a.length); for (let i = 0; i < o.length; i++) o[i] = a[i]; return o; };
// the scan times of a file are the same for every ion: an rt equal (bit for bit) to the last one of the same file returns that array, one copy instead of one per ion
// (20 ions x 6 files of 50 000 scans: half the copies and the garbage; isSorted in decima.js is cached per array). No caller edits a trace array in place.
/** @type {Map<number, { arr: number[], bits: Int32Array }>} */
const RT_LAST = new Map();
const rtArr = (/** @type {number} */ k, /** @type {Float64Array} */ a) => {
  const o = RT_LAST.get(k), u = new Int32Array(a.buffer, a.byteOffset, 2 * a.length);        // the words of the doubles: an integer compare is several times faster
  if (o && o.bits.length === u.length) { const b = o.bits, n = u.length; let i = 0; for (; i < n; i++) if (b[i] !== u[i]) break; if (i === n) return o.arr; }
  const arr = f64Arr(a); RT_LAST.set(k, { arr, bits: u.slice() }); return arr;
};
