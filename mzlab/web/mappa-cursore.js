// @ts-check
// @ts-strict
"use strict";
// Cursor of the RT x m/z map, the parts that draw nothing on the canvases (classic script, loaded before mappa.js): the chip with RT and m/z that follows
// the pointer, the dot on the XIC at the cursor RT, and the label of the peak under the cursor in the side spectrum. All DOM / geometry; the data come from mappa.js.
const MAPCURSORE = (() => {
  /** The two decorative elements, in the grid of the map: absolute, pointer-events none (app.css), hidden until the cursor is over the map.
   * @param {HTMLElement} grid @returns {{ chip: HTMLElement, live: HTMLElement }} */
  const dom = grid => {
    const chip = document.createElement("div"); chip.className = "mzchip"; chip.hidden = true; chip.setAttribute("aria-hidden", "true"); grid.appendChild(chip);
    const live = document.createElement("div"); live.className = "mzlive"; live.hidden = true; live.setAttribute("aria-hidden", "true"); live.innerHTML = '<i class="mzlive-l"></i><i class="mzlive-d"></i>'; grid.appendChild(live);
    return { chip, live };
  };

  /** The label of the peak under the cursor: bold, beyond the bar tip when it fits, otherwise above the bar and the guide, always inside the panel; returns its box.
   * @param {CanvasRenderingContext2D} g @param {number} W panel width @param {Record<string, any>} q {t, tip, y, i} @param {number} x0 @param {{ y: number, end: number } | null} guide
   * @param {number} px font size @param {number} asc @param {number} hh @param {(n: number, st?: string) => string} fpx */
  const label = (g, W, q, x0, guide, px, asc, hh, fpx) => {
    g.font = fpx(12.5, "bold "); const w = g.measureText(q.t).width, right = q.tip + 5 + w <= W - 2;
    const top = guide ? Math.min(q.y, guide.y) : q.y, by = right ? q.y + Math.round(px * .35) : top - 7, x = right ? q.tip + 5 : Math.min(W - 2 - w, Math.max(x0 + 2, q.tip - 3 - w));
    g.strokeText(q.t, x, by); g.fillText(q.t, x, by); g.font = fpx(12.5);
    return { t: q.t, x, y: by - asc, w, h: hh, ly: q.y, tip: q.tip, right, i: q.i, cur: true };
  };

  /** Shows or hides both elements for the cursor of panel `p`. The cursor is the pointer or the key-moved point; the locked point only while the map has the keyboard focus.
   * The chip flips to the other side near the edges; on the XIC the dot rides the trace (s.xg, set by drawBottom) at the cursor RT, with a thin line when the sight is held elsewhere.
   * @param {any} p @param {{ st: (p: any) => Record<string, any>, is2d: (p: any) => boolean, point: (p: any) => any, mzd: (p: any) => number, T: (k: string, v?: any) => string, MIRINO: any }} c */
  const sync = (p, c) => {
    const s = c.st(p), a = p._a, chip = s.chip, live = s.live; if (!chip || !live) return;
    const q = a && c.is2d(p) && !s.frz ? s.hov || s.kp || (s.lock && document.activeElement === p.cv ? s.lock : null) : null;
    const inside = !!q && q.rt >= a.x0 && q.rt <= a.x1 && q.mz >= a.y0 && q.mz <= a.y1;
    chip.hidden = !inside; if (!inside) { live.hidden = true; return; }
    const cv = p.cv, ox = cv.offsetLeft + cv.clientLeft, oy = cv.offsetTop + cv.clientTop, x = ox + a.X(q.rt), y = oy + a.Y(q.mz), pk = c.point(p) === q && s.curPk ? s.curPk : null;
    chip.textContent = c.T("mappa.live.chip", { rt: q.rt.toFixed(2), mz: q.mz.toFixed(c.mzd(p)) }) + (pk ? " · " + c.T("mappa.live.peak", { mz: pk }) : "");
    const w = chip.offsetWidth, h = chip.offsetHeight, xr = ox + a.W - 2, yb = oy + a.H - 2;
    chip.style.left = Math.round(Math.max(ox + 2, x + 14 + w > xr ? x - 14 - w : x + 14)) + "px"; chip.style.top = Math.round(Math.max(oy + 2, y + 16 + h > yb ? y - 14 - h : y + 16)) + "px";
    const g = s.xg, xc = s.xc;      // the trace value at the cursor RT, linearly between its two nearest points
    if (!g || !g.tr || xc.hidden || q.rt < a.x0 || q.rt > a.x1 || !g.tr.rt.length) { live.hidden = true; return; }
    const r = g.tr.rt, i = c.MIRINO.nearest(r, q.rt), j = Math.min(r.length - 1, Math.max(0, q.rt > r[i] ? i + 1 : i - 1)), d = r[j] - r[i];
    const v = d ? g.tr.y[i] + (g.tr.y[j] - g.tr.y[i]) * Math.min(1, Math.max(0, (q.rt - r[i]) / d)) : g.tr.y[i];
    const pt = c.point(p), apart = !pt || Math.abs(g.X(pt.rt) - g.X(q.rt)) > 1.5;
    live.hidden = false; live.style.left = (xc.offsetLeft + xc.clientLeft + g.X(q.rt)).toFixed(1) + "px"; live.style.top = (xc.offsetTop + xc.clientTop + g.mt).toFixed(1) + "px";
    const l = /** @type {HTMLElement} */ (live.firstElementChild), dot = /** @type {HTMLElement} */ (live.lastElementChild);
    l.style.height = (g.H - g.mb - g.mt) + "px"; l.hidden = !apart; dot.style.top = (g.Y(v) - g.mt).toFixed(1) + "px";
  };
  return { dom, label, sync };
})();
