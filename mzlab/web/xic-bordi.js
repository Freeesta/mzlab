// @ts-check
// The «from / to» pairs of the «Range» popover (pannelli.js ctl): the «m/z» edges of a low-resolution XIC, and the m/z (nm of a PDA trace) and RT,
// looked up in the whole panel (a narrow header moves them into «More»). A pair is read when the focus leaves it (Enter, or a click elsewhere), so
// either end can be written first; a refused pair (not a number, from above to) brings the old values back with a calm note; an accepted one is
// undoable and saved, and the focus stays where it went.
(() => {
  /** @param {any} p @param {HTMLInputElement} x */
  function wire(p, x) {
    const go = () => setTimeout(() => {
      const t = p.traces[p._xt], q = /** @type {HTMLInputElement[]} */ ([...p.el.querySelectorAll('[data-o="xlo"],[data-o="xhi"]')]), a = /** @type {any} */ (document.activeElement);
      if (!t || !x.isConnected || q.length !== 2 || (q.includes(a) && a !== x) || q.every(i => i.value === i.defaultValue)) return;   // the other edge has the focus: wait for it
      const hrE = t.ion && window.HR && HR.xicEdges(t, p), nm = (/** @type {string} */ v) => hrE ? (isFinite(numIt(v)) && numIt(v) > 0 ? rh(numIt(v), 5) : null) : numMz1(v);
      const lo = nm(q[0].value), hi = nm(q[1].value), ok = lo != null && hi != null && hi > lo, d = hrE ? 5 : 2;
      if (!ok) { q.forEach(i => { i.value = i.defaultValue; }); return CAMPI.note(I18N.t(lo == null || hi == null ? "xic.edge.num" : "xic.edge.order"), x); }
      COMANDI.fai("xic.traces", p, () => { t.mz = rh((lo + hi) / 2, d); t.w = rh((hi - lo) / 2, d); t.label = hrE ? `m/z ${t.mz.toFixed(4)}` : `m/z ${lo}\u2013${hi}`; delete t.ion; delete t.obs; });
      keepFocus(p, () => { ctl(p); draw(p); }); uiSave();
    });
    x.onchange = go; x.onblur = go;
  }
  /** the m/z (or nm) and RT pairs of the «Range» popover: empty = no limit; RT may start at 0 @param {any} p @param {HTMLInputElement} x */
  function range(p, x) {
    const rt = x.dataset.o === "rt0" || x.dataset.o === "rt1", ks = rt ? ["rt0", "rt1"] : ["mz0", "mz1"], pda = !rt && p.kind === "pda";
    const go = () => setTimeout(() => {
      const q = /** @type {HTMLInputElement[]} */ (ks.map(k => p.el.querySelector(`[data-o="${k}"]`))), a = /** @type {any} */ (document.activeElement);
      if (!x.isConnected || q.some(i => !i) || (q.includes(a) && a !== x) || q.every(i => i.value === i.defaultValue)) return;   // the other end has the focus: wait for it
      const rd = (/** @type {string} */ v) => { if (!v.trim()) return null; const n = CAMPI.num(v); return n != null && isFinite(n) && (rt ? n >= 0 : n > 0) ? n : NaN; };
      let [lo, hi] = q.map(i => rd(i.value)), bad = Number.isNaN(lo) || Number.isNaN(hi) ? "num" : "";
      if (!bad && rt && (lo != null || hi != null)) { const A = p._a || {}; lo ??= A.xmin ?? A.x0; hi ??= A.xmax ?? A.x1; }   // one RT end: the other is the edge of the data
      if (!bad && lo != null && hi != null && (pda ? hi < lo : hi <= lo)) bad = "order";   // one wavelength is a range of its own
      if (bad) { q.forEach(i => { i.value = i.defaultValue; }); CAMPI.note(I18N.t(bad === "num" ? "ctl.range.num" : "ctl.range.order"), x); return; }
      keepFocus(p, () => {
        if (rt) { pushZh(p); if (lo == null) { p.zoom = null; p.zoomY = null; draw(p); } else zoomTo(p, lo, hi); }
        else { const r2 = (/** @type {number | null} */ v) => v == null ? null : Math.round(v * 100) / 100; COMANDI.fai("panel.source", p, () => { p.mz0 = r2(lo); p.mz1 = r2(hi); ctl(p); }); draw(p); }
        uiSave();
      });
      q.forEach(i => { i.defaultValue = i.value; });   // an RT zoom keeps these fields: the pair is applied, a later blur must not apply it again
    });
    x.onchange = go; x.onblur = go;
  }
  /** runs fn, which may draw the header again, and gives the focus back to the same control of the new header @param {any} p @param {() => void} fn */
  function keepFocus(p, fn) {
    const a = /** @type {HTMLElement | null} */ (document.activeElement), d = a && p.el.contains(a) ? a.dataset : null;
    const sel = d?.o ? `[data-o="${d.o}"]` : d?.a ? `[data-a="${d.a}"]` : "";
    fn();
    if (sel && a && !a.isConnected) /** @type {HTMLElement | null} */ (p.el.querySelector(sel))?.focus();
  }
  /** @type {any} */ (window).XICBORDI = { wire, range };
})();
