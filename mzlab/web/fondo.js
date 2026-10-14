// @ts-check
// Background correction of a chromatogram trace (TIC / XIC / MRM): pure functions, tested in Node (tests/fondo_node.js). Classic script, loaded before grafici.js.
// The state of a panel is p.fondo = { mode: "none" | "auto" | "tract", w, ib }: "auto" = SNIP baseline of half-width w min, centred on the mean of the noise;
// "tract" = the level of a stretch chosen by the student, ib = { mode: "mean" | "median" | "line", a: [t0, t1], b: [t0, t1] | null }.
// The order is fixed (grafici.js, corrected): raw traces -> difference with the blank (in the engine, never clipped) -> ONE background on the difference.
// The zero of the axis is the mean level of the background, for every method. Nothing here clips: «negativi a 0» is only a way of drawing.
const FONDO = (() => {
  /** @param {number[]} a */
  const sorted = a => a.slice().sort((p, q) => p - q);
  /** @param {number[]} s sorted */
  const medOf = s => (s.length % 2 ? s[s.length >> 1] : (s[s.length / 2 - 1] + s[s.length / 2]) / 2);
  /** SNIP (Morhac 1997; Ryan 1988) with the LLS transform: the baseline is clipped iteratively by the mean of its two neighbours. @param {number[]} y @param {number} w */
  function snipBaseline(y, w) {
    const n = y.length; if (n < 2 * w + 3) return new Array(n).fill(0);
    let v = y.map(a => Math.log(Math.log(Math.sqrt(Math.max(a, 0) + 1) + 1) + 1));
    for (let m = 1; m <= w; m++) { const t = v.slice(); for (let i = m; i < n - m; i++) { const a = (v[i - m] + v[i + m]) / 2; if (a < t[i]) t[i] = a; } v = t; }
    return v.map(a => { const e = Math.exp(Math.exp(a) - 1) - 1; return e * e - 1; });
  }
  /** Lift of the SNIP baseline: the mean of the residuals y - bl of the background points (those not above median + 3 robust sigma), so that the noise is centred on 0. @param {number[]} y @param {number[]} bl */
  function lift(y, bl) {
    const r = y.map((v, i) => v - bl[i]), s = sorted(r), m = medOf(s), mad = medOf(sorted(r.map(v => Math.abs(v - m)))) * 1.4826, cut = m + 3 * mad;
    let sum = 0, n = 0; for (const v of r) if (v <= cut) { sum += v; n++; }
    return n ? sum / n : m;
  }
  /** Median step of the time axis (min). @param {number[]} x */
  function dtOf(x) { const d = sorted(x.slice(1).map((v, i) => v - x[i])); return d[d.length >> 1] || 0.01; }
  /** Default length (min) of the initial zone, counted from the first scan: the solvent front, where the baseline means nothing. */
  const ZD = 0.5;
  /** End time (min) of the initial zone of a trace; -Infinity when no background is on. @param {number[]} x @param {{ mode: string, zd?: number } | null | undefined} fo */
  const zoneEnd = (x, fo) => (!fo || fo.mode === "none" || !x.length ? -Infinity : x[0] + (fo.zd != null && fo.zd >= 0 ? fo.zd : ZD));
  /** Automatic baseline of a trace: SNIP of window w min, centred, computed outside the initial zone (inside it the baseline is the value a window in).
   *  Returns the baseline (same length as y) or null when the trace is too short. @param {number[]} x @param {number[]} y @param {number} w @param {number} [zd] */
  function auto(x, y, w, zd) {
    const z = zoneEnd(x, { mode: "auto", zd }); let k = 0; while (k < x.length && x[k] < z) k++;
    if (x.length - k <= 5) return null;
    const xs = x.slice(k), ys = y.slice(k), h = Math.max(2, Math.round((w || 1) / dtOf(xs) / 2)), bl = snipBaseline(ys, h), up = lift(ys, bl), b = bl.map(v => v + up);
    return new Array(k).fill(b[Math.min(h, b.length - 1)]).concat(b);
  }
  /** Optional detection of the end of the solvent front (min from the first scan): the first point after the highest one of the first quarter of the run that is back to the noise of the second half (median + 3 robust sigma).
   *  Returns null when there is no front. @param {number[]} x @param {number[]} y */
  function front(x, y) {
    const n = x.length; if (n < 20) return null;
    const late = sorted(y.slice(n >> 1)), m = medOf(late), mad = medOf(sorted(late.map(v => Math.abs(v - m)))) * 1.4826, cut = m + 3 * mad;
    let im = 0; for (let i = 1; i < n >> 2; i++) if (y[i] > y[im]) im = i;
    if (y[im] <= m + 8 * mad) return null;                       // a front is a clear peak, not the top of the noise
    let i = im; while (i < n && y[i] > cut) i++;
    return i < n ? Math.round((x[i] - x[0]) * 100) / 100 : null;
  }
  /** Level of a stretch: mean or median of the points with t in r. @param {number[]} x @param {number[]} y @param {number[]} r @param {boolean} [med] */
  function stat(x, y, r, med) {
    const v = []; x.forEach((t, i) => { if (t >= r[0] && t <= r[1]) v.push(y[i]); });
    if (!v.length) return 0;
    return med ? medOf(sorted(v)) : v.reduce((p, q) => p + q, 0) / v.length;
  }
  /** Background of a stretch chosen by the student: a constant (mean / median) or a line between two stretches. @param {number[]} x @param {number[]} y @param {{ mode: string, a: number[], b?: number[] | null }} ib
   *  @returns {{ f: (t: number) => number, lvl: number, lvl2?: number }} */
  function tract(x, y, ib) {
    if (ib.mode === "line" && ib.b) {
      const xa = (ib.a[0] + ib.a[1]) / 2, xb = (ib.b[0] + ib.b[1]) / 2, ya = stat(x, y, ib.a), yb = stat(x, y, ib.b), m = (yb - ya) / ((xb - xa) || 1);
      return { f: t => ya + m * (t - xa), lvl: ya, lvl2: yb };
    }
    const L = stat(x, y, ib.a, ib.mode === "median"); return { f: () => L, lvl: L };
  }
  /** The background of a trace: { y: corrected values (never clipped), ibl: levels of a stretch (for the chip) | null, bl: { min, max } of the automatic baseline | undefined }, or null when no background applies.
   *  @param {number[]} x @param {number[]} y @param {{ mode: string, w?: number, ib?: any, zd?: number } | null | undefined} fo */
  function apply(x, y, fo) {
    if (!fo || fo.mode === "none") return null;
    if (fo.mode === "tract" && fo.ib) { const t = tract(x, y, fo.ib); return { y: y.map((v, i) => v - t.f(x[i])), ibl: t }; }
    if (fo.mode === "auto") { const bl = auto(x, y, fo.w || 1, fo.zd); return bl ? { y: y.map((v, i) => v - bl[i]), ibl: null, bl: { min: Math.min(...bl), max: Math.max(...bl) } } : null; }
    return null;
  }
  /** The state of a panel from a saved one or an old notebook (snip / snipw / ibk). @param {any} o */
  function migra(o) {
    o = o || {};
    if (o.fondo && typeof o.fondo === "object") {
      const f = o.fondo, mode = f.mode === "auto" || f.mode === "tract" ? f.mode : "none";
      return { mode: mode === "tract" && !f.ib ? "none" : mode, w: +f.w > 0 ? +f.w : 1, ib: f.ib || null, ...(f.zd != null && +f.zd >= 0 ? { zd: +f.zd } : {}) };
    }
    const ib = o.ibk && o.ibk.a ? { mode: o.ibk.mode || "mean", a: o.ibk.a, b: o.ibk.b || null } : null;
    return { mode: ib ? "tract" : o.snip ? "auto" : "none", w: +o.snipw > 0 ? +o.snipw : 1, ib };
  }
  /** «negativi a 0» of an old notebook (it was inside ibk and only for the tract). @param {any} o */
  const clipOld = o => !!(o && (o.clipDraw || (!o.fondo && o.ibk && o.ibk.clip)));
  return { snipBaseline, lift, auto, stat, tract, apply, migra, clipOld, dtOf, zoneEnd, front, ZD };
})();
