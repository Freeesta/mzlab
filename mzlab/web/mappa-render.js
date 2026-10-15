// @ts-check
"use strict";
// RT x m/z map, the parts that do not touch the page (classic script, loaded before mappa.js; node loads it with require).
//   splat(v, nrt, nmz, sx, sy)   the «Liscia» look of a zoomed region: every cell spreads its value as a gaussian tent (max, not sum: heights stay those of the
//                                matrix) as wide as the m/z resolution (sy, in bins) and the scan spacing (sx, in bins), so a peak stays one continuous blob
//   snrOf(mz, y, lo, hi)         S/N of each peak: its intensity over the noise of its neighbourhood (median + 1.4826 MAD of the other peaks within a band in m/z)
//   pick(mz, y, o)               which peaks get a m/z label, best first: the active peak, then the best S/N of each band of the visible window, then the best S/N left
(() => {
  const GROUP_DA = 2.5, MAXR = 48;
  /** @param {number} sigma */
  function kernel(sigma) {
    const r = sigma < 0.35 ? 0 : Math.min(MAXR, Math.ceil(3 * sigma)), w = new Float32Array(r + 1);
    for (let d = 0; d <= r; d++) w[d] = r ? Math.exp(-d * d / (2 * sigma * sigma)) : 1;
    return { r, w };
  }
  /** One non-negative matrix, v[i * nmz + j] (i = scan bin, j = m/z bin): scatter from the non-zero cells only (a region is mostly empty). */
  function spread(u, nrt, nmz, kx, ky) {
    let cur = u;
    if (ky.r) {
      const o = new Float32Array(cur.length);
      for (let i = 0; i < nrt; i++) { const b = i * nmz;
        for (let j = 0; j < nmz; j++) { const x = cur[b + j]; if (x <= 0) continue; const a = Math.max(0, j - ky.r), z = Math.min(nmz - 1, j + ky.r);
          for (let k = a; k <= z; k++) { const y = x * ky.w[Math.abs(k - j)]; if (y > o[b + k]) o[b + k] = y; } } }
      cur = o;
    }
    if (kx.r) {
      const o = new Float32Array(cur.length);
      for (let i = 0; i < nrt; i++) { const b = i * nmz, a = Math.max(0, i - kx.r), z = Math.min(nrt - 1, i + kx.r);
        for (let j = 0; j < nmz; j++) { const x = cur[b + j]; if (x <= 0) continue;
          for (let k = a; k <= z; k++) { const y = x * kx.w[Math.abs(k - i)], q = k * nmz + j; if (y > o[q]) o[q] = y; } } }
      cur = o;
    }
    return cur;
  }
  /** @param {Float32Array} v @param {number} nrt @param {number} nmz @param {number} sx sigma in RT bins @param {number} sy sigma in m/z bins */
  function splat(v, nrt, nmz, sx, sy) {
    const kx = kernel(sx), ky = kernel(sy); if (!kx.r && !ky.r) return v;
    let neg = false; for (let i = 0; i < v.length && !neg; i++) if (v[i] < 0) neg = true;
    if (!neg) return spread(v, nrt, nmz, kx, ky);
    const p = new Float32Array(v.length), n = new Float32Array(v.length);                // a difference A − B: the two signs spread apart, the larger wins in each cell
    for (let i = 0; i < v.length; i++) { if (v[i] > 0) p[i] = v[i]; else n[i] = -v[i]; }
    const P = spread(p, nrt, nmz, kx, ky), N = spread(n, nrt, nmz, kx, ky), o = new Float32Array(v.length);
    for (let i = 0; i < v.length; i++) o[i] = P[i] >= N[i] ? P[i] : -N[i];
    return o;
  }
  const med = (a) => { const s = Float64Array.from(a).sort(), n = s.length; return n % 2 ? s[(n - 1) >> 1] : (s[n / 2 - 1] + s[n / 2]) / 2; };
  /** Level of the noise of a list of intensities: median + 1.4826 MAD (the typical height of a noise peak, robust to a few real ones). */
  function level(a) {
    const m = med(a), d = new Float64Array(a.length); for (let i = 0; i < a.length; i++) d[i] = Math.abs(a[i] - m);
    return m + 1.4826 * med(d);
  }
  /**
   * @param {ArrayLike<number>} mz @param {ArrayLike<number>} y  peaks (any order; intensities by absolute value)
   * @param {number} lo @param {number} hi  the visible m/z window (only these peaks are scored)
   * @param {{band?: number}} [o] band: half width in Da of the neighbourhood (default: a quarter of the window, at least 15 Da)
   * @returns {{ i: number, snr: number, noise: number }[]} one per visible peak, in the order of the input
   */
  function snrOf(mz, y, lo, hi, o = {}) {
    const n = mz.length, idx = Array.from({ length: n }, (_, i) => i).sort((a, b) => mz[a] - mz[b]), m = idx.map(i => mz[i]), v = idx.map(i => Math.abs(y[i]));
    const H = o.band || Math.max(15, (hi - lo) / 4), MIN = 6; let gmax = 0; for (const x of v) if (x > gmax) gmax = x;
    const floor = Math.max(gmax * 1e-4, 1e-12), wide = level(v.length >= MIN ? v : [gmax * 0.02]);
    const lb = (x) => { let a = 0, b = n; while (a < b) { const h = (a + b) >> 1; if (m[h] < x) a = h + 1; else b = h; } return a; };
    const out = [];
    for (let k = 0; k < n; k++) {
      if (m[k] < lo || m[k] > hi) continue;
      const a = lb(m[k] - H), z = lb(m[k] + H + 1e-9), band = [];
      for (let j = a; j < z; j++) if (Math.abs(m[j] - m[k]) > GROUP_DA) band.push(v[j]);      // the peak and its isotopes are not noise
      const noise = Math.max(band.length >= MIN ? level(band) : wide, floor);
      out.push({ i: idx[k], snr: v[k] / noise, noise });
    }
    return out;
  }
  /**
   * @param {ArrayLike<number>} mz @param {ArrayLike<number>} y
   * @param {{ lo: number, hi: number, bands?: number, quota?: number, minSnr?: number, act?: [number, number] | null, strategy?: "bands" | "snr" | "abs", band?: number }} o
   *   strategy bands (default): the active peak, the best `quota` of each of `bands` equal slices of [lo, hi], then the best S/N left; snr: best S/N only; abs: the old rule
   *   (intensity over 5 % of the highest). Peaks under minSnr are never picked. A peak with a more intense one within GROUP_DA is an isotope: not picked.
   * @returns {{ i: number, snr: number, noise: number, act?: boolean }[] & { all: { i: number, snr: number, noise: number }[] }} candidates, best first (`all`: the S/N of every visible peak): the caller draws them in order until they fit
   */
  function pick(mz, y, o) {
    const { lo, hi } = o, strategy = o.strategy || "bands", bands = Math.max(1, o.bands || 4), quota = o.quota || 1, minSnr = o.minSnr ?? 5, ab = (i) => Math.abs(y[i]);
    const all = snrOf(mz, y, lo, hi, o), by = all.slice().sort((a, b) => ab(b.i) - ab(a.i)), seen = [], keep = [];
    for (const c of by) if (!seen.some(k => Math.abs(mz[k] - mz[c.i]) <= GROUP_DA)) { seen.push(c.i); keep.push(c); }       // the highest of each group of peaks
    let top = 0; for (const c of by) { top = Math.max(top, ab(c.i)); break; }
    /** @type {typeof keep} */ let ord;
    if (strategy === "abs") ord = keep.filter(c => ab(c.i) >= top * 0.05);
    else {
      const ok = keep.filter(c => c.snr >= minSnr).sort((a, b) => b.snr - a.snr);
      if (strategy === "snr") ord = ok;
      else {
        const w = (hi - lo) / bands || 1, first = [], per = new Array(bands).fill(0);
        for (const c of ok) { const b = Math.min(bands - 1, Math.max(0, Math.floor((mz[c.i] - lo) / w))); if (per[b] < quota) { per[b]++; first.push(c); } }
        ord = first.concat(ok.filter(c => !first.includes(c)));
      }
    }
    if (o.act) {                                                    // the peak of the XIC / sight is always labelled, first
      let best = -1; for (const c of all) if (mz[c.i] >= o.act[0] && mz[c.i] <= o.act[1] && (best < 0 || ab(c.i) > ab(all[best].i))) best = all.indexOf(c);
      if (best >= 0) ord = /** @type {any[]} */ ([{ ...all[best], act: true }]).concat(ord.filter(c => c.i !== all[best].i));
    }
    return Object.assign(ord, { all });
  }
  const MAPREND = { splat, snrOf, pick, GROUP_DA };
  if (typeof window !== "undefined") /** @type {any} */ (window).MAPREND = MAPREND;
  if (typeof module !== "undefined") module.exports = MAPREND;
})();
