// @ts-check
// MS² spectrum card: marks it with the class `ms2c` (the CSS wraps its header so the fragments controls (`.ctl`) take their own line) and names that line for assistive tech.
// No info rows under the header: the RT is in the title, the sample in the header selector and the legend, the CE in the peak table; an MS¹ card keeps its single header row.
(() => {
  /** @param {any} p */
  function update(p) {
    if (p.type !== "spec" || !p.el) return;
    const on = p.level > 1; p.el.classList.toggle("ms2c", on);
    const ctl = p.el.querySelector(".ctl");
    if (on && ctl) { ctl.setAttribute("role", "group"); ctl.setAttribute("aria-label", I18N.t("srow.frag")); }
  }
  window.SPECROWS = { update };
})();
