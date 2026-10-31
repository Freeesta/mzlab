// @ts-check
// @ts-strict
"use strict";
// Drawing of the small XIC of the lens (xic-lente.js), the parts that do not touch the page (classic script; node loads it for the axis test).
//   ticks(x0, x1)      round, equidistant, equally-decimal tick values for an RT axis (at least 4 when the range allows it)
// XICPIECES: the pieces both drawings share.
//   axis(g, X, x0, x1, o) the RT axis ticks and numbers; trace(g, t, X, Y, base, pw, o) the filled trace (every pixel column keeps its highest and lowest point); apexLabel(...) the number on the apex
//   draw(g, w, h, t, o) all of them for the lens; the small XIC under the 2D map (mappa.js) draws with the same axis, trace and apexLabel
const XICPIECES = (() => {
  const STEPS = [100, 50, 20, 10, 5, 2, 1, 0.5, 0.2, 0.1, 0.05, 0.02, 0.01, 0.005, 0.002, 0.001];
  /** The multiples of the largest round step that leaves at least 4 ticks inside [x0, x1] (the finest step if none does).
   * @param {number} x0 @param {number} x1 @returns {{ vals: number[], dec: number }} */
  function ticks(x0, x1) {
    let best = STEPS[STEPS.length - 1], vals = /** @type {number[]} */ ([]);
    const at = (/** @type {number} */ st) => { const o = []; for (let k = Math.ceil(x0 / st - 1e-9); k * st <= x1 + 1e-9; k++) o.push(+(k * st).toFixed(6)); return o; };
    for (const st of STEPS) { const v = at(st); if (v.length >= 4) { best = st; vals = v; break; } }
    if (!vals.length) vals = at(best);
    const dec = best >= 1 ? 0 : Math.min(3, Math.ceil(-Math.log10(best) - 1e-9));
    return { vals, dec };
  }
  /** The points to draw for `pw` pixel columns: first, lowest, highest and last of each column, in data order (no apex is lost).
   * @param {{ rt: number[], y: number[] }} t @param {number} pw @returns {number[]} */
  function columns(t, pw) {
    const n = t.rt.length, x0 = t.rt[0], span = t.rt[n - 1] - x0 || 1, out = [];
    let c = -1, f = 0, lo = 0, hi = 0;
    const flush = (/** @type {number} */ l) => { for (const i of [...new Set([f, lo, hi, l])].sort((u, v) => u - v)) out.push(i); };
    for (let i = 0; i < n; i++) {
      const k = Math.min(pw - 1, Math.floor((t.rt[i] - x0) / span * (pw - 1)));
      if (k !== c) { if (c >= 0) flush(i - 1); c = k; f = lo = hi = i; }
      else { if (t.y[i] < t.y[lo]) lo = i; if (t.y[i] > t.y[hi]) hi = i; }
    }
    if (c >= 0) flush(n - 1);
    return out;
  }
  /** @param {{ y: number[] }} t first maximum */
  const apex = t => { let b = 0; for (let i = 1; i < t.y.length; i++) if (t.y[i] > t.y[b]) b = i; return b; };
  /** the apex is worth a time label: at least 3 times the median and wider than one point (more than one point at half height) @param {{ rt: number[], y: number[] }} t @param {number} b */
  function clearPeak(t, b) {
    const top = t.y[b]; if (!(top > 0) || t.rt.length < 3) return false;
    const med = [...t.y].sort((u, v) => u - v)[t.y.length >> 1];
    return top >= 3 * med && t.y.filter(v => v >= top / 2).length > 1;
  }
  /** Round equidistant RT ticks on the line y = o.base, numbers at o.labelY kept between o.lo and o.hi; returns the tick x positions.
   * @param {CanvasRenderingContext2D} g @param {(v: number) => number} X @param {number} x0 @param {number} x1 @param {{ base: number, labelY: number, lo: number, hi: number, color: string, font: string }} o */
  function axis(g, X, x0, x1, o) {
    const tk = ticks(x0, x1), tx = [];
    g.fillStyle = o.color; g.strokeStyle = o.color; g.font = o.font; g.textAlign = "left";
    for (const v of tk.vals) {
      const x = Math.round(X(v)) + .5, s = v.toFixed(tk.dec), tw = g.measureText(s).width;
      g.beginPath(); g.moveTo(x, o.base); g.lineTo(x, o.base + 3); g.stroke();
      g.fillText(s, Math.max(o.lo, Math.min(x - tw / 2, o.hi - tw)), o.labelY); tx.push(x);
    }
    return tx;
  }
  /** The filled trace over `pw` pixel columns (fill at 30 % of the colour, line 1.5 px), under the current globalAlpha; returns the points drawn.
   * @param {CanvasRenderingContext2D} g @param {{ rt: number[], y: number[] }} t @param {(v: number) => number} X @param {(v: number) => number} Y @param {number} base @param {number} pw @param {string} color @param {number[]} [dash] */
  function trace(g, t, X, Y, base, pw, color, dash) {
    const idx = columns(t, Math.max(2, Math.round(pw)));
    g.beginPath(); g.moveTo(X(t.rt[idx[0]]), base);
    for (const i of idx) g.lineTo(X(t.rt[i]), Y(t.y[i]));
    g.lineTo(X(t.rt[idx[idx.length - 1]]), base); g.closePath();
    const al = g.globalAlpha; g.globalAlpha = al * .3; g.fillStyle = color; g.fill(); g.globalAlpha = al;
    g.strokeStyle = color; g.lineWidth = 1.5; g.lineJoin = "round"; if (dash) g.setLineDash(dash); g.beginPath();
    idx.forEach((i, k) => { if (k) g.lineTo(X(t.rt[i]), Y(t.y[i])); else g.moveTo(X(t.rt[i]), Y(t.y[i])); });
    g.stroke(); g.lineWidth = 1; if (dash) g.setLineDash([]);
    return idx;
  }
  /** The RT of the apex (one decimal) above it, kept between lo and hi and below the top row. @param {CanvasRenderingContext2D} g @param {{ rt: number[], y: number[] }} t @param {number} b @param {number} ax @param {number} ay @param {number} lo @param {number} hi @param {string} ink */
  function apexLabel(g, t, b, ax, ay, lo, hi, ink) {
    const s = t.rt[b].toFixed(1), tw = g.measureText(s).width;
    g.fillStyle = ink; g.textAlign = "left"; g.fillText(s, Math.max(lo, Math.min(ax - tw / 2, hi - tw)), Math.max(9, ay - 3));
  }
  return { ticks, columns, apex, clearPeak, axis, trace, apexLabel };
})();
const XICDRAW = (() => {
  const { ticks, columns, apex, clearPeak, axis, trace, apexLabel } = XICPIECES;
  /** The part of a trace with rt inside [x0, x1]. @param {{ rt: number[], y: number[] }} t @param {number} x0 @param {number} x1 */
  function windowOf(t, x0, x1) {
    const v = { rt: /** @type {number[]} */ ([]), y: /** @type {number[]} */ ([]) };
    for (let i = 0; i < t.rt.length; i++) if (t.rt[i] >= x0 && t.rt[i] <= x1) { v.rt.push(t.rt[i]); v.y.push(t.y[i]); }
    return v;
  }
  /** @typedef {{ line: string, ink: string, muted: string, accent: string, scanRt?: number | null, label?: boolean }} Look */
  /**
   * @param {CanvasRenderingContext2D} g (already scaled to css px) @param {number} w @param {number} h css size
   * @param {{ rt: number[], y: number[] }} t @param {Look} o
   * @returns {{ x0: number, y0: number, x1: number, y1: number, tx: number[], ax: number, ay: number } | null} the plot box (x0..x1 × y0..y1), the tick x positions and the apex
   */
  function draw(g, w, h, t, o) {
    const L = 6, R = 6, Tp = 12, Bt = 14, pw = w - L - R, ph = h - Tp - Bt, base = Tp + ph, n = t.rt.length;
    g.clearRect(0, 0, w, h); g.font = "10px sans-serif"; g.textBaseline = "alphabetic";
    g.strokeStyle = o.line; g.lineWidth = 1; g.beginPath(); g.moveTo(L, base + .5); g.lineTo(L + pw, base + .5); g.stroke();
    if (!n) return null;
    const x0 = t.rt[0], x1 = t.rt[n - 1], span = x1 - x0 || 1;
    let ym = 0; for (let i = 0; i < n; i++) if (t.y[i] > ym) ym = t.y[i];
    const X = (/** @type {number} */ v) => L + (v - x0) / span * pw, Y = (/** @type {number} */ v) => base - (ym > 0 ? v / ym : 0) * ph;
    // axis: round equidistant values, the first and the last kept inside the box
    const tx = axis(g, X, x0, x1, { base, labelY: h - 2, lo: 1, hi: w - 1, color: o.muted, font: "9px sans-serif" });
    g.font = "10px sans-serif";
    if (o.scanRt != null && o.scanRt >= x0 && o.scanRt <= x1) {
      g.save(); g.strokeStyle = o.muted; g.setLineDash([3, 3]); g.beginPath(); g.moveTo(X(o.scanRt), Tp); g.lineTo(X(o.scanRt), base); g.stroke(); g.restore();
    }
    trace(g, t, X, Y, base, pw, o.accent);
    const b = apex(t), ax = X(t.rt[b]), ay = Y(t.y[b]);
    if (o.label) apexLabel(g, t, b, ax, ay, L, L + pw, o.ink);
    return { x0: L, y0: Tp, x1: L + pw, y1: base, tx, ax, ay };
  }
  return { ticks, columns, apex, clearPeak, axis, trace, apexLabel, draw, slice: windowOf };
})();
