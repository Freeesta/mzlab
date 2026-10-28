// @ts-check
// @ts-strict
"use strict";
// Layout of the ruler of the spectrum (spettro.js drawMeas), the part that does not touch the page (classic script; node loads it for the layout test).
// The labels of the peaks have precedence: the «rif.» text, every row of the measured differences and the dashed verticals look for a place that crosses none of them.
//   layout(o)  rows (a free horizontal line per measure, 17 px steps), the place of «rif.» and the pieces of every vertical that stay outside the labels
const RIGHELLO = (() => {
  /** @typedef {{ x: number, y: number, w: number, h: number, g?: number }} Rect  (g: the measure it belongs to, -1 for «rif.») */
  const STEP = 17, TICK = 4, SLACK = 2;
  /** @param {Rect} a @param {Rect} b */
  const hit = (a, b) => a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
  /** the parts of the vertical [y0, y1] at x that no rectangle covers @param {number} x @param {number} y0 @param {number} y1 @param {Rect[]} obs @returns {[number, number][]} */
  function pieces(x, y0, y1, obs) {
    const cut = obs.filter(r => x >= r.x - 1 && x <= r.x + r.w + 1 && r.y < y1 && r.y + r.h > y0).sort((p, q) => p.y - q.y);
    /** @type {[number, number][]} */ const out = []; let at = y0;
    for (const r of cut) { if (r.y > at) out.push([at, r.y]); at = Math.max(at, r.y + r.h); }
    if (at < y1) out.push([at, y1]);
    return out;
  }
  /**
   * @param {{ obs: Rect[], x0: number, x1: number, top: number, base: number, lh: number,
   *   ref: { x: number, w: number } | null, items: { xa: number, xb: number, w: number }[] }} o  obs = rectangles of the labels of the peaks; lh = height of a text line
   * @returns {{ ref: { x: number, y: number } | null, items: { y: number, bx: number, va: [number, number][], vb: [number, number][] }[], rects: Rect[] }}
   */
  function layout(o) {
    const taken = o.obs.slice(), rects = /** @type {Rect[]} */ ([]);
    const free = (/** @type {Rect[]} */ rs) => !rs.some(r => taken.some(t => hit(r, t)));
    const claim = (/** @type {Rect[]} */ rs) => { for (const r of rs) { taken.push(r); rects.push(r); } };
    /** @type {{ x: number, y: number } | null} */ let ref = null;
    if (o.ref) {
      const rows = [o.top + 10, o.top + 10 + STEP, o.top + 10 + 2 * STEP];
      for (const y of rows) for (const left of [false, true]) {
        const x = left ? o.ref.x - 3 - o.ref.w : o.ref.x + 3; if (x < o.x0 + 1 || x + o.ref.w > o.x1 - 1) continue;
        const r = { x: x - 1, y: y - o.lh + 2, w: o.ref.w + 2, h: o.lh, g: -1 };
        if (!ref && free([r])) { ref = { x, y }; claim([r]); }
      }
      if (!ref) { ref = { x: o.ref.x + 3, y: rows[0] }; }
    }
    const first = o.top + 26, last = o.base - 8;
    const items = o.items.map((q, i) => {
      const lo = Math.min(q.xa, q.xb), hi = Math.max(q.xa, q.xb), mid = (q.xa + q.xb) / 2, bx = Math.max(o.x0 + 2, Math.min(o.x1 - q.w - 2, mid - q.w / 2));
      /** @type {(y: number) => Rect[]} */ const rowsAt = y => [{ x: lo - SLACK, y: y - TICK - 1, w: hi - lo + 2 * SLACK, h: 2 * TICK + 2, g: i }, { x: bx - 3, y: y - 5 - o.lh, w: q.w + 6, h: o.lh + 5, g: i }];
      let y = first + (i % 6) * STEP, found = false;
      for (let k = 0; first + k * STEP <= last; k++) { const c = first + k * STEP; if (free(rowsAt(c))) { y = c; found = true; break; } }
      const rs = rowsAt(y); if (found) claim(rs); else for (const r of rs) rects.push(r);
      const body = taken.filter(t => !rs.includes(t));
      return { y, bx, va: pieces(q.xa, y, o.base, body), vb: pieces(q.xb, y, o.base, body) };
    });
    return { ref, items, rects };
  }
  return { layout, pieces, hit };
})();
