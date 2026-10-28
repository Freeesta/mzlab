// @ts-check
// @ts-strict
"use strict";
// Drawing of the small XIC of the lens (xic-lente.js), the parts that do not touch the page (classic script; node loads it for the axis test).
//   ticks(x0, x1)      round, equidistant, equally-decimal tick values for an RT axis (at least 4 when the range allows it)
//   draw(g, w, h, t, o) the axis, the filled trace (every pixel column keeps its highest and lowest point) and the apex number; returns what it drew
const XICDRAW = (() => {
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
    const tk = ticks(x0, x1), tx = [];
    g.fillStyle = o.muted; g.strokeStyle = o.muted; g.font = "9px sans-serif";
    for (const v of tk.vals) {
      const x = Math.round(X(v)) + .5, s = v.toFixed(tk.dec), tw = g.measureText(s).width;
      g.beginPath(); g.moveTo(x, base); g.lineTo(x, base + 3); g.stroke();
      g.textAlign = "left"; g.fillText(s, Math.max(1, Math.min(x - tw / 2, w - tw - 1)), h - 2); tx.push(x);
    }
    g.font = "10px sans-serif";
    if (o.scanRt != null && o.scanRt >= x0 && o.scanRt <= x1) {
      g.save(); g.strokeStyle = o.muted; g.setLineDash([3, 3]); g.beginPath(); g.moveTo(X(o.scanRt), Tp); g.lineTo(X(o.scanRt), base); g.stroke(); g.restore();
    }
    const idx = columns(t, Math.max(2, Math.round(pw)));
    g.beginPath(); g.moveTo(X(t.rt[idx[0]]), base);
    for (const i of idx) g.lineTo(X(t.rt[i]), Y(t.y[i]));
    g.lineTo(X(t.rt[idx[idx.length - 1]]), base); g.closePath();
    const al = g.globalAlpha; g.globalAlpha = al * .3; g.fillStyle = o.accent; g.fill(); g.globalAlpha = al;
    g.strokeStyle = o.accent; g.lineWidth = 1.5; g.lineJoin = "round"; g.beginPath();
    idx.forEach((i, k) => { if (k) g.lineTo(X(t.rt[i]), Y(t.y[i])); else g.moveTo(X(t.rt[i]), Y(t.y[i])); });
    g.stroke(); g.lineWidth = 1;
    const b = apex(t), ax = X(t.rt[b]), ay = Y(t.y[b]);
    if (o.label) {
      const s = t.rt[b].toFixed(1), tw = g.measureText(s).width;
      g.fillStyle = o.ink; g.textAlign = "left"; g.fillText(s, Math.max(L, Math.min(ax - tw / 2, L + pw - tw)), Math.max(9, ay - 3));
    }
    return { x0: L, y0: Tp, x1: L + pw, y1: base, tx, ax, ay };
  }
  return { ticks, columns, apex, draw };
})();
