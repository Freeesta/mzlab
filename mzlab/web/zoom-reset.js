// @ts-check
// «Reset zoom» inside the «Range» popover of chromatogram, XIC and MRM panels: the same action as the four-corners button of the header (whole run, x and y).
// aria-disabled rather than disabled, so the button stays in the tab order of the popover; after a reset the RT fields show the full range.
// Classic helpers used by name: I18N, EH, pushZh, draw, uiSave, fmt2.
(() => {
  const zoomed = (/** @type {any} */ p) => !!(p.zoom || p.zoomY);
  /** the button, placed after the fields of the popover @param {any} p */
  const btn = p => `<button class="bt zreset" type="button" data-a="zreset" aria-disabled="${!zoomed(p)}" title="${EH(I18N.t(zoomed(p) ? "ctl.zreset.title" : "ctl.zreset.off"))}">${EH(I18N.t("ctl.zreset"))}</button>`;
  /** back to the whole run; the header may be rebuilt meanwhile (the «Range» accent follows the zoom), so focus and fields are found again @param {any} p */
  async function reset(p) {
    if (!zoomed(p)) return;
    const had = document.activeElement?.matches('[data-a="zreset"]');
    pushZh(p); p.zoom = null; p.zoomY = null; await draw(p); uiSave();
    const q = /** @type {NodeListOf<HTMLInputElement>} */ (p.el.querySelectorAll('.cpop [data-o="rt0"], .cpop [data-o="rt1"]')), f = p._a && p._a.full;
    if (q.length === 2 && f) q.forEach((i, n) => { i.value = i.defaultValue = fmt2(f[n]); });
    if (had) /** @type {HTMLElement | null} */ (p.el.querySelector('[data-a="zreset"]'))?.focus();
  }
  /** @param {any} p @param {Element} c the controls of the header */
  const wire = (p, c) => { const b = /** @type {HTMLElement | null} */ (c.querySelector('[data-a="zreset"]')); if (b) b.onclick = () => reset(p); };
  /** the state follows the zoom after every drawing @param {any} p @param {boolean} z */
  function sync(p, z) {
    const b = p.el.querySelector('[data-a="zreset"]'); if (!b) return;
    b.setAttribute("aria-disabled", String(!z)); b.setAttribute("title", I18N.t(z ? "ctl.zreset.title" : "ctl.zreset.off"));
  }
  window.ZRESET = { btn, wire, sync, reset };
})();
