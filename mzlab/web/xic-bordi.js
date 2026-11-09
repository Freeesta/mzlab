// @ts-check
// The «m/z from / to» fields of a low-resolution XIC header (pannelli.js ctl): ABSOLUTE edges, read when the focus leaves the pair (Enter, or a click
// elsewhere), so the window can move to another mass whichever edge is written first. A refused pair (not a positive number, from >= to) brings the old
// edges back with a calm note under the header; an accepted one is undoable (xic.traces) and saved with the session.
(() => {
  /** @param {any} p @param {HTMLElement} c @param {HTMLInputElement} x */
  function wire(p, c, x) {
    const go = () => setTimeout(() => {
      const t = p.traces[p._xt], q = /** @type {HTMLInputElement[]} */ ([...c.querySelectorAll('[data-o="xlo"],[data-o="xhi"]')]), a = /** @type {any} */ (document.activeElement);
      if (!t || !x.isConnected || q.length !== 2 || (q.includes(a) && a !== x) || q.every(i => i.value === i.defaultValue)) return;   // the other edge has the focus: wait for it
      const hrE = t.ion && window.HR && HR.xicEdges(t, p), nm = (/** @type {string} */ v) => hrE ? (isFinite(numIt(v)) && numIt(v) > 0 ? rh(numIt(v), 5) : null) : numMz1(v);
      const lo = nm(q[0].value), hi = nm(q[1].value), ok = lo != null && hi != null && hi > lo, d = hrE ? 5 : 2;
      if (ok) COMANDI.fai("xic.traces", p, () => { t.mz = rh((lo + hi) / 2, d); t.w = rh((hi - lo) / 2, d); t.label = `m/z ${hrE ? t.mz.toFixed(4) : t.mz.toFixed(mzd(p))}`; delete t.ion; delete t.obs; });
      ctl(p); draw(p); if (ok) uiSave();
      else note(p, I18N.t(lo == null || hi == null ? "xic.edge.num" : "xic.edge.order"));
    });
    x.onchange = go; x.onblur = go;
  }
  /** a short status line under the header of panel p; it goes away by itself @param {any} p @param {string} msg */
  function note(p, msg) {
    let m = /** @type {HTMLElement | null} */ (p.el.querySelector(".xr-msg"));
    if (!m) { m = document.createElement("div"); m.className = "xr-msg"; m.setAttribute("role", "status"); p.el.appendChild(m); }
    const hd = /** @type {HTMLElement} */ (p.el.querySelector(".hd")), el = m;
    el.style.top = hd.offsetTop + hd.offsetHeight + 2 + "px"; el.hidden = false; el.textContent = "";
    setTimeout(() => { el.textContent = msg; }, 20);   // set after the clear: a screen reader announces every refusal
    clearTimeout(p._xrNote); p._xrNote = setTimeout(() => { el.hidden = true; }, 6000);
  }
  /** @type {any} */ (window).XICBORDI = { wire };
})();
