// @ts-nocheck
// The «A = …» label of an integration on a chromatogram, drawn above its apex (click to copy, tip on hover). A window split at its valleys gets one label
// per part, from the engine's table rows (MANINT.parts), stacked when close parts would overlap. Classic script, loaded by grafici.js with the first integration.
const AREALBL = (() => {
  /** @param {any} p @param {CanvasRenderingContext2D} g @param {(t: number) => number} X @param {(v: number) => number} Y @param {(s: any, v: number) => number} U @param {any} s @param {any} it @param {any} r the window as measured by the page @param {any[]} sr */
  function draw(p, g, X, Y, U, s, it, r, sr) {
    // a window split at its valleys labels each part (the engine's rows, MANINT.parts); until they arrive, or without splits, the window has the one label
    const parts = it.splits && !it.fol && !it.none && window.MANINT ? MANINT.parts(it) : null; /** @type {{ x0: number, x1: number, y: number }[]} */ const placed = [];
    for (const q of parts || [{ rt: r.rt, area: r.area, height: r.height, wait: r.wait }]) {
      let ay = Math.max(M.t + 28, Y(U(s, s.ys[nearIdx(s.x, q.rt)])) - 20); const lab = "A = " + (it.none ? "—" : q.wait && !Number.isFinite(q.area) ? "…" : I18N.sciE(q.area)) + (sr.length > 1 ? " · " + s.name : "");   // clearly above the apex, never on the peak; the baseline mode is in the hover text
      g.font = fpx(13, "bold "); g.textAlign = "center"; g.lineWidth = 4; g.lineJoin = "round"; g.strokeStyle = ecss("--panel");
      const lw = g.measureText(lab).width, cx = X(q.rt), lh = 6 + 13 * fz();
      for (const o of placed) if (cx - lw / 2 < o.x1 + 4 && o.x0 < cx + lw / 2 + 4 && Math.abs(ay - o.y) < lh) ay = Math.max(M.t + 12 * fz(), Math.min(ay, o.y - lh));   // close parts: stacked above the previous label
      placed.push({ x0: cx - lw / 2, x1: cx + lw / 2, y: ay });
      g.strokeText(lab, cx, ay); g.fillStyle = ecss("--ink"); g.fillText(lab, cx, ay); g.lineWidth = 1;
      (p._a.lbls = p._a.lbls || []).push({ x: cx - lw / 2 - 3, y: ay - 2 - 13 * fz(), w: lw + 6, h: 5 + 13 * fz(), int: it, text: lab, copy: it.none ? null : I18N.sciE(q.area, 5), tip: (it.nm ? `<div><b>${EH(it.nm)}</b></div>` : "") + (it.none ? "" : I18N.t(p.kind === "pda" ? "tip.areaUv" : "tip.area", { area: I18N.sciE(q.area, 5), y: I18N.sciE(q.height) })) + (it.fol ? SEGUE.tip(p, it) : it.bm ? `<div class="sm">${I18N.t("int.base.tip", { b: I18N.t("int.base.tag." + it.bm) })}</div>` : "") + (it.none ? "" : `<div class="sm">${I18N.t("int.copy.tip")}</div>`) }); g.font = fpx(12);
    }
  }
  return { draw };
})();
