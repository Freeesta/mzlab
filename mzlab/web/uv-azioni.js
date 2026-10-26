// @ts-strict
// Actions of the UV spectrum (DAD) that open a wavelength trace; used by the menu and the keys of a `spec` panel with `uv: true`. Classic script, loaded after eventi.js (wlOpen).
(() => {
  const T = I18N.t.bind(I18N);
  /** @param {any} o */
  const reg = o => AZIONI.registra({ viste: ["uv", "spec"], gruppo: "xic", ...o });
  // the DAD trace at the wavelength under the cursor, or over the nm range in view (zoom with the keys or the axis: no dragging needed)
  reg({ id: "uv.wlAt", visibile: /** @param {any} c */ c => !!c.p.uv, nome: /** @param {any} c */ c => T("ctx.uv.wlAt", { nm: Math.round(c.x) }),
    esegui: /** @param {any} c */ c => wlOpen(c.p, +c.x.toFixed(1), +c.x.toFixed(1), c.p.k) });
  reg({ id: "uv.wlView", visibile: /** @param {any} c */ c => !!c.p.uv, disponibile: /** @param {any} c */ c => !!c.p.zoom, scorciatoia: { tasto: "x", dove: "chart", et: "scorc.wl" }, nome: () => T("ctx.uv.wlView"),
    esegui: /** @param {any} c */ c => wlOpen(c.p, +c.p._a.x0.toFixed(1), +c.p._a.x1.toFixed(1), c.p.k) });
})();
