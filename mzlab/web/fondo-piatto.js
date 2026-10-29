// @ts-strict
// Flat-window background of a chromatogram: the program picks the steadiest low stretch of the run (Rust, crates/mzlab-core/src/chrom_fondo.rs through QUALITA),
// the student can move it (handle on the chart, arrows, buttons) and go back to the automatic one. State: p.fondo = { mode: "tract", ib: { mode: "median", a: [t0, t1], flat: true, mod?: true } };
// «mod» = moved by the student: the window is no longer recomputed. Classic script, loaded after grafici.js (corrected(), ibkChip() and the chart events call it).
const FONDOP = (() => {
  const GRIP = 12;                                                  // height (px) of the handle drawn on top of the window
  /** @param {any} p */
  const ibf = p => { const ib = ibOf(p); return ib && ib.flat ? ib : null; };
  /** Automatic window of a trace (after the initial zone): sets ib.a, or tells the student when the trace is too short. @param {any} fo @param {number[]} x @param {number[]} y */
  async function auto(fo, x, y) {
    let k = 0; const z = FONDO.zoneEnd(x, fo); while (k < x.length && x[k] < z) k++;
    const w = window.QUALITA ? await QUALITA.flatWindow(y.slice(k)) : null;
    if (w) fo.ib.a = [x[k + w.i0], x[k + w.i1 - 1]]; else info(I18N.t("fondo.flat.none"));
  }
  /** Does the window still have to be found by the program? @param {any} fo */
  const wants = fo => !!fo && fo.mode === "tract" && !!fo.ib && !!fo.ib.flat && !fo.ib.mod;
  /** Is the background the flat window? @param {any} fo */
  const is = fo => !!fo && fo.mode === "tract" && !!fo.ib && !!fo.ib.flat;
  /** Entry of the «Fondo» corrections menu (and its icon, added once to CIC). @param {any} o @returns {any[]} */
  function item(o) {
    if (!o.parts || !o.parts.includes("flat")) return [];
    /** @type {any} */ (CIC).flat ??= ICN('<path d="M2 14c3 0 4-8 7-8s4 8 7 8"/><path d="M2 15.5h14" stroke-dasharray="2 2"/><path d="M5 12v5M13 12v5"/>');
    return [["flat", "flat", I18N.t("corr.flat"), I18N.t("corr.flat.desc")]];
  }
  /** Entry of the trace menu: back to the automatic window (only after a move). @param {any} p @returns {{ label: string, fn: () => void }[]} */
  const menu = p => { const ib = ibf(p); return ib && ib.mod ? [{ label: I18N.t("fondo.flat.restore"), fn: () => restore(p) }] : []; };
  /** Switches the background to the automatic flat window (the window itself is found when the traces are corrected). @param {any} p */
  function on(p) {
    pushCampi(p, "storia.fondo"); p.fondo = { ...p.fondo, mode: "tract", ib: { mode: "median", a: [0, 0], b: null, flat: true } };
  }
  /** Sets the window from t0 (min) keeping its width; marks it as moved. @param {any} p @param {number} t0 */
  function put(p, t0) {
    const ib = ibf(p), a = p._a; if (!ib) return;
    const w = ib.a[1] - ib.a[0], lo = a ? a.full[0] : t0, hi = a ? a.full[1] : t0 + w, s = Math.max(lo, Math.min(hi - w, t0));
    p.fondo = { ...p.fondo, ib: { ...ib, a: [s, s + w], mod: true } };
  }
  /** One step of the arrows or buttons: a quarter of the window. @param {any} p @param {number} dir -1 | 1 @param {boolean} big */
  function step(p, dir, big) {
    const ib = ibf(p); if (!ib) return;
    pushCampi(p, "storia.fondo"); put(p, ib.a[0] + dir * (ib.a[1] - ib.a[0]) / 4 * (big ? 5 : 1)); draw(p); uiSave();
  }
  /** Back to the automatic window. @param {any} p */
  function restore(p) {
    const ib = ibf(p); if (!ib) return;
    pushCampi(p, "storia.fondo"); const { mod: _m, ...rest } = ib; p.fondo = { ...p.fondo, ib: rest }; draw(p); uiSave();
  }
  /** Is (px, py) on the handle of the window? @param {any} p @param {number} px @param {number} py */
  function gripAt(p, px, py) {
    const ib = ibf(p), a = p._a; if (!ib || !a || p.imode) return false;
    return px >= a.X(ib.a[0]) - 3 && px <= a.X(ib.a[1]) + 3 && py >= M.t && py <= M.t + GRIP;
  }
  /** Start of a drag on the handle: one undo step for the whole drag. @param {any} p @param {number} x time under the mouse */
  function begin(p, x) { const ib = ibf(p); if (!ib) return null; pushCampi(p, "storia.fondo"); return { x0: x, t0: ib.a[0] }; }
  /** The handle of the window, drawn over the band. @param {CanvasRenderingContext2D} g @param {any} ib @param {(t: number) => number} X @param {number} W */
  function drawGrip(g, ib, X, W) {
    const xa = Math.max(X(ib.a[0]), M.l), xb = Math.min(X(ib.a[1]), W - M.r); if (!(xb > xa)) return;
    g.save(); g.fillStyle = ecss("--accent"); g.globalAlpha = ib.mod ? 0.55 : 0.3; g.fillRect(xa, M.t, xb - xa, GRIP); g.globalAlpha = 1;
    g.strokeStyle = ecss("--accent"); g.lineWidth = 2; const c = (xa + xb) / 2; g.beginPath(); for (const d of [-4, 0, 4]) { g.moveTo(c + d, M.t + 3); g.lineTo(c + d, M.t + GRIP - 3); } g.stroke(); g.restore();
  }
  /** The focus goes back to the arrow just used for a moment: the chart is redrawn (more than once) and the chip with it. @param {any} p @param {string} k */
  function keep(p, k) { p._flatFocus = k; clearTimeout(p._flatFT); p._flatFT = setTimeout(() => { p._flatFocus = null; }, 600); }
  /** Chip text and controls of the flat window; keys: ◀ ▶ (arrows with the focus on them, Shift = five steps), restore when moved. @param {any} p @param {any} ib @param {string} levels @param {HTMLElement} el the chip: it gets its role and name */
  function chip(p, ib, levels, el) {
    const st = I18N.t(ib.mod ? "fondo.flat.mod" : "fondo.flat.auto"), ranges = ibkRange(ib.a, 2);
    const html = `${I18N.t("fondo.flat.chip", { ranges, levels })} <span class="muted">(${st})</span> <button type="button" data-ib="fl-" title="${I18N.t("fondo.flat.left")}" aria-label="${I18N.t("fondo.flat.left")}">◀</button>`
      + `<button type="button" data-ib="fl+" title="${I18N.t("fondo.flat.right")}" aria-label="${I18N.t("fondo.flat.right")}">▶</button>`
      + (ib.mod ? ` <button type="button" data-ib="fl0">↺ ${I18N.t("fondo.flat.restore")}</button>` : "");
    el.setAttribute("role", "group"); el.setAttribute("aria-label", I18N.t("fondo.flat.aria", { state: st, ranges }));
    return html;
  }
  /** Wires the controls of chip() inside `el`; the focus comes back to the arrow just used after the chart is redrawn. @param {any} p @param {HTMLElement} el */
  function wire(p, el) {
    for (const [k, dir] of /** @type {[string, number][]} */ ([["fl-", -1], ["fl+", 1]])) {
      const b = el.querySelector(`[data-ib="${k}"]`); if (!(b instanceof HTMLElement)) continue;
      b.onclick = e => { keep(p, k); step(p, +dir, e.shiftKey); };
      b.onkeydown = e => { if (e.key === "ArrowLeft" || e.key === "ArrowRight") { e.preventDefault(); keep(p, e.key === "ArrowLeft" ? "fl-" : "fl+"); step(p, e.key === "ArrowLeft" ? -1 : 1, e.shiftKey); } };
    }
    const r = el.querySelector('[data-ib="fl0"]'); if (r instanceof HTMLElement) r.onclick = () => { keep(p, "fl-"); restore(p); };
    if (p._flatFocus) { const f = el.querySelector(`[data-ib="${p._flatFocus}"]`); if (f instanceof HTMLElement) f.focus(); }
  }
  return { is, item, wants, menu, auto, on, put, step, restore, gripAt, begin, drawGrip, chip, wire };
})();
