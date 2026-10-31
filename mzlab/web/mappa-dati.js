// @ts-strict
// Pure helpers of the RT x m/z map data: the matrix of api/map (base64 of float32) and the noise of a spectrum or trace. Classic script, loaded before mappa.js; tested in Node (tests/test_perf_web6.py).
const MAPDATI = (() => {
  /** The matrix of api/map as a Float32Array (the native decoder where the browser has one). @param {string} b64 */
  const b64f32 = b64 => {
    const native = /** @type {any} */ (Uint8Array).fromBase64;
    let u; if (native) u = native.call(Uint8Array, b64); else { const bin = atob(b64); u = new Uint8Array(bin.length); for (let i = 0; i < bin.length; i++) u[i] = bin.charCodeAt(i); }
    return new Float32Array(u.buffer, 0, u.length >> 2);
  };
  const med = (/** @type {Float64Array} */ a) => { const h = a.length >> 1; return a.length & 1 ? a[h] : (a[h - 1] + a[h]) / 2; };
  /** Noise level: 1.4826 x the median absolute deviation of the positive values (0 when there are none). @param {ArrayLike<number>} y */
  const madNoise = y => {
    const v = new Float64Array(y.length); let n = 0; for (let i = 0; i < y.length; i++) if (y[i] > 0) v[n++] = y[i];
    if (!n) return 0;
    const a = v.subarray(0, n).sort(), m = med(a); for (let i = 0; i < n; i++) a[i] = Math.abs(a[i] - m);
    return 1.4826 * med(a.sort());
  };
  return { b64f32, madNoise };
})();
