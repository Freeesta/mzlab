// @ts-check
// Spectrum panel bar: the title «MS¹» with the retention time, the overlay toggle and the compact «follows» symbol.
// Used by pannelli.js (ctl) and banco.js (header).
(() => {
  const T = (/** @type {string} */ k, /** @type {any} */ a) => I18N.t(k, a);
  /** overlay-files toggle: an icon button made of a checkbox (keeps the data-o="all" handler of the controls) @param {any} p @param {boolean} dis @param {string} why */
  const overlay = (p, dis, why) => `<label class="ovb${p.all && !dis ? " on" : ""}${dis ? " dis" : ""}" title="${dis ? why : T("ctl.spec.overlay")}"><input type="checkbox" data-o="all" class="sr-only" aria-label="${T("ctl.spec.overlay")}" ${p.all ? "checked" : ""}${dis ? " disabled" : ""}><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M1 12 4 5l3 5 3-8 5 10"/><path d="M1 14h14" opacity=".5"/></svg></label>`;
  /** «MS» + level in superscript for a plain spectrum; the time of the scan gets its tooltip @param {any} p @param {HTMLElement | null} t */
  function title(p, t) {
    const rl = p.el.querySelector(".rtl"); if (rl) rl.title = T("ctl.spec.rt");
    if (!t) return;
    if (p.uv || p.title !== PT.spec) { t.removeAttribute("role"); t.removeAttribute("aria-label"); return; }
    const n = p.level > 1 ? p.level : 1;
    if (t.firstChild?.nodeValue !== "MS" || t.querySelector("sup")?.textContent !== String(n)) { const u = document.createElement("sup"); u.textContent = String(n); t.replaceChildren("MS", u); }
    t.setAttribute("role", "img"); t.setAttribute("aria-label", T("ctl.spec.lv", { n }));
  }
  /** number of files drawn in the spectrum (several only in overlay) @param {any} p */
  const nFiles = p => p.all ? tabFiles(p.tab || E.tab).filter((/** @type {any} */ f) => f.vis && !f.gone).length : 1;
  window.SPECBAR = { overlay, title, nFiles };
})();
