// @ts-check
// Page layout of the panels: a stack of rows, each row with 1-3 panels side by side (shared height, widths from `quota`).
// `geometria` is pure (tested in Node); `colloca` reads the panels of one stack and gives back the rectangles. No view is named here:
// the callers pass the minimum width of each panel and the gap that follows each row.
const LAYOUT = (() => {
  const MAX_ROW = 3;
  /** Reading order of the panels: top to bottom, then left to right, then creation. The only comparator of the page. */
  const ordine = (/** @type {{x?: number, y: number, id: number}} */ a, /** @type {{x?: number, y: number, id: number}} */ b) => a.y - b.y || (a.x || 0) - (b.x || 0) || a.id - b.id;

  /**
   * @typedef {{ id: number, quota?: number | null, h: number, min?: number }} Voce
   * @typedef {{ voci: Voce[], gapDopo?: number }} Riga
   * @typedef {{ id: number, x: number, y: number, w: number, h: number }} Rett
   */
  /** Widths (px) of the n items of a row: shares from `quota` (equal when missing), none below its minimum, summing to `avail`. */
  function larghezze(/** @type {Voce[]} */ voci, avail) {
    const n = voci.length, q = voci.map(v => (v.quota != null && v.quota > 0 ? v.quota : 1 / n)), tot = q.reduce((a, b) => a + b, 0);
    let w = q.map(x => avail * x / tot);
    const min = voci.map(v => v.min || 0), fixed = new Set();
    for (let it = 0; it < n; it++) {                                           // the ones under their minimum take it; the others share what is left
      let changed = false;
      w.forEach((x, i) => { if (!fixed.has(i) && x < min[i]) { fixed.add(i); changed = true; } });
      if (!changed) break;
      const left = avail - [...fixed].reduce((a, i) => a + min[i], 0), free = q.filter((_, i) => !fixed.has(i)), ft = free.reduce((a, b) => a + b, 0);
      w = w.map((x, i) => (fixed.has(i) ? min[i] : ft > 0 ? left * q[i] / ft : x));
    }
    return w;
  }
  /**
   * Rectangles of a stack of rows in a width W. A row whose items do not fit side by side (each at least its minimum) is shown stacked:
   * the saved arrangement is untouched, only the rectangles change. A row of one item is never touched horizontally (the caller keeps its width).
   * @returns {{ rett: Rett[], H: number, impilata: boolean[] }}
   */
  function geometria(/** @type {Riga[]} */ righe, W, gap, minW = 0) {
    /** @type {Rett[]} */ const rett = [], impilata = [];
    let y = 0;
    righe.forEach((r, ri) => {
      const n = r.voci.length, avail = W - gap * (n - 1);
      const fit = n === 1 || (n <= MAX_ROW && r.voci.every(v => (v.min || minW) * n + gap * (n - 1) <= W) && avail > 0);
      impilata.push(!fit);
      const after = r.gapDopo != null ? r.gapDopo : gap;
      if (!fit) {                                                                // narrow window: one under the other, each with its own height
        r.voci.forEach((v, i) => { rett.push({ id: v.id, x: 0, y, w: W, h: v.h }); y += v.h + (i < n - 1 ? gap : 0); });
      } else {
        const h = Math.max(...r.voci.map(v => v.h)), w = larghezze(r.voci.map(v => ({ ...v, min: v.min || minW })), avail);
        let x = 0;
        r.voci.forEach((v, i) => { const wi = i === n - 1 ? W - x : Math.floor(w[i]); rett.push({ id: v.id, x, y, w: wi, h }); x += wi + gap; });
        y += h;
      }
      y += ri < righe.length - 1 ? after : 0;
    });
    return { rett, H: y, impilata };
  }

  /** Rows of an ordered list of panels: consecutive panels with the same `riga` (at most MAX_ROW) share one row. */
  function righeDi(/** @type {any[]} */ order) {
    /** @type {any[][]} */ const rows = [];
    order.forEach(q => { const l = rows[rows.length - 1]; if (l && q.riga != null && l[0].riga === q.riga && l.length < MAX_ROW) l.push(q); else rows.push([q]); });
    return rows;
  }
  const inRiga = (/** @type {any} */ p, /** @type {any[]} */ list) => p.riga != null && list.some(q => q !== p && q.riga === p.riga && q.tab === p.tab);

  /**
   * Places an ordered stack. Returns Map panel → rectangle. Single-item rows get only `y` (their width stays what the page gives them);
   * `riga` is renumbered in order, so a notebook without rows is one row per panel, as before.
   */
  function colloca(/** @type {any[]} */ order, W, gap, minOf, gapDopo) {
    const rows = righeDi(order);
    const righe = rows.map((r, i) => ({ voci: r.map(q => ({ id: q.id, quota: q.quota, h: q.h, min: minOf ? minOf(q) : 0 })), gapDopo: gapDopo ? gapDopo(r[r.length - 1], rows[i + 1] && rows[i + 1][0]) : gap }));
    const g = geometria(righe, W, gap, 0), byId = new Map(g.rett.map(r => [r.id, r]));
    const out = new Map();
    rows.forEach((r, i) => r.forEach(q => {
      q.riga = i;
      const t = byId.get(q.id); if (!t) return;
      if (r.length === 1 && q.quota == null) out.set(q, { id: q.id, x: q.x, y: t.y, w: q.w, h: q.h });
      else { out.set(q, t); if (r.length === 1) q.quota = null; }       // left alone by its neighbours: back to the full width
    }));
    return { pos: out, H: g.H, impilata: g.impilata };
  }
  /** A panel leaves its row (own row from now on, full width). */
  function esci(/** @type {any} */ p, /** @type {any[]} */ list, W) {
    const used = list.reduce((m, q) => Math.max(m, q.riga ?? -1), -1);
    p.riga = used + 1; p.quota = null; p.x = 0; p.w = W;
  }
  /** The row has one height: the panels beside `p` take its height. */
  function altezza(/** @type {any} */ p, /** @type {any[]} */ list) { if (p.riga != null) list.forEach(q => { if (q !== p && q.riga === p.riga && q.tab === p.tab && q.full) q.h = p.h; }); }
  return { ordine, geometria, colloca, righeDi, inRiga, esci, altezza, MAX_ROW };
})();
