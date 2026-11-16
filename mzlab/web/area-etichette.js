// @ts-nocheck
// The «A = …» label of an integration on a chromatogram, drawn above its apex (click to copy, tip on hover). A window split at its valleys gets one label
// per part, from the engine's table rows (MANINT.parts), stacked when close parts would overlap. Classic script, loaded by grafici.js with the first integration.
const AREALBL = (() => {
  /** @param {any} p @param {CanvasRenderingContext2D} g @param {(t: number) => number} X @param {(v: number) => number} Y @param {(s: any, v: number) => number} U @param {any} s @param {any} it @param {any} r the window as measured by the page @param {any[]} sr */
  function draw(p, g, X, Y, U, s, it, r, sr) {
    // a window split at its valleys labels each part (the engine's rows, MANINT.parts); until they arrive, or without splits, the window has the one label
    const parts = it.splits && !it.fol && !it.none && window.MANINT ? MANINT.parts(it) : null, many = sr.length > 1, A = p._a, lbls = A.lbls = A.lbls || [];
    for (const q of parts || [{ rt: r.rt, area: r.area, height: r.height, wait: r.wait }]) {
      const top = Y(U(s, s.ys[nearIdx(s.x, q.rt)])), y0 = Math.max(M.t + 28, top - 20);                   // clearly above the apex, never on the peak
      const lab = "A = " + (it.none ? "—" : q.wait && !Number.isFinite(q.area) ? "…" : I18N.sciE(q.area));   // overlaid files: the colour of the text is the file's, its name is in the hover text
      g.font = fpx(13, "bold "); g.textAlign = "center"; g.lineJoin = "round";
      const lw = g.measureText(lab).width, bw = lw + 6, bh = 5 + 13 * fz(), step = bh + 1, ax = X(q.rt);
      const cx = Math.max(M.l + bw / 2, Math.min(A.W - M.r - bw / 2, ax)), bx = cx - bw / 2;                // inside the plot at any zoom
      const free = (/** @type {number} */ y) => y - 2 - 13 * fz() >= M.t && !lbls.some((/** @type {any} */ o) => o.int && bx < o.x + o.w && o.x < bx + bw && y - 2 - 13 * fz() < o.y + o.h && o.y < y - 2 - 13 * fz() + bh);
      let ay = y0; for (let k = 1; k <= 12 && !free(ay); k++) ay = y0 - Math.ceil(k / 2) * step * (k % 2 ? 1 : -1);   // a free row: above first, then below
      if (!free(ay)) ay = y0;
      if (Math.abs(ay - y0) > 1 || Math.abs(cx - ax) > 1) { g.strokeStyle = s.color; g.lineWidth = 1; g.beginPath(); g.moveTo(cx, ay + 3); g.lineTo(ax, top - 2); g.stroke(); }   // a thin leader to its apex
      g.lineWidth = 4; g.strokeStyle = ecss("--panel"); g.strokeText(lab, cx, ay); g.fillStyle = many ? s.color : ecss("--ink"); g.fillText(lab, cx, ay); g.lineWidth = 1;
      lbls.push({ x: bx, y: ay - 2 - 13 * fz(), w: bw, h: bh, int: it, text: lab, copy: it.none ? null : I18N.sciE(q.area, 5), tip: (it.nm ? `<div><b>${EH(it.nm)}</b></div>` : "") + (many ? `<div>${EH(s.name)}</div>` : "") + (it.none ? "" : I18N.t(p.kind === "pda" ? "tip.areaUv" : "tip.area", { area: I18N.sciE(q.area, 5), y: I18N.sciE(q.height) })) + (it.fol ? SEGUE.tip(p, it) : it.bm ? `<div class="sm">${I18N.t("int.base.tip", { b: I18N.t("int.base.tag." + it.bm) })}</div>` : "") + (it.none ? "" : `<div class="sm">${I18N.t("int.copy.tip")}</div>`) }); g.font = fpx(12);
    }
  }
  return { draw };
})();
