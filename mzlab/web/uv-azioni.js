// @ts-strict
// Actions of the UV spectrum (DAD) that open a wavelength trace; used by the menu and the keys of a `spec` panel with `uv: true`. Classic script, loaded after eventi.js (wlOpen).
(() => {
  const T = I18N.t.bind(I18N);
  /** @param {any} o */
  const reg = o => AZIONI.register({ views: ["uv", "spec"], group: "xic", ...o });
  // the DAD trace at the wavelength under the cursor, or over the nm range in view (zoom with the keys or the axis: no dragging needed)
  // the double click on the UV spectrum is the way back: the trace of the DAD at that wavelength (the PDA trace gives the spectrum, the spectrum gives the trace)
  reg({ id: "uv.wlAt", visible: /** @param {any} c */ c => !!c.p.uv, name: /** @param {any} c */ c => T("ctx.uv.wlAt", { nm: Math.round(c.x) }), doubleClickOrder: 3,
    doubleClick: /** @param {any} c */ c => !!c.p.uv && !!c.p._a && !c.p.el.classList.contains("max"),
    run: /** @param {any} c */ c => wlOpen(c.p, +c.x.toFixed(1), +c.x.toFixed(1), c.p.k) });
  reg({ id: "uv.wlView", visible: /** @param {any} c */ c => !!c.p.uv, available: /** @param {any} c */ c => !!c.p.zoom, shortcut: { key: "x", where: "chart", label: "scorc.wl" }, name: () => T("ctx.uv.wlView"),
    run: /** @param {any} c */ c => wlOpen(c.p, +c.p._a.x0.toFixed(1), +c.p._a.x1.toFixed(1), c.p.k) });
})();
