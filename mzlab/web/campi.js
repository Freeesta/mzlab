// @ts-check
// Typed numeric fields: one gate for the whole page. A change on <input type=number> (or on a text field marked data-num) that is not a number in its
// bounds is stopped before the field's own handler runs: the last accepted value comes back and one calm note (role=status) says why. Bounds come from
// min/max (data-min/data-max on text fields), data-gt is an exclusive lower bound, data-vuoto allows the empty field; the handlers then read only valid text.
(() => {
  /** @type {WeakMap<HTMLInputElement, string>} */
  const last = new WeakMap();
  /** Typed text -> number | null (empty) | NaN: one decimal comma or point, an exponent, a sign; anything else (letters, «1,2.3», 1e999) is NaN.
   * @param {unknown} t */
  function num(t) {
    const s = String(t ?? "").trim().replace(/−/g, "-");
    if (s === "") return null;
    const c = s.split(",").length - 1;
    if (c > 1 || (c === 1 && s.includes("."))) return NaN;
    const d = s.replace(",", ".");
    if (!/^[-+]?(\d+\.?\d*|\.\d+)(e[-+]?\d+)?$/i.test(d)) return NaN;
    const v = Number(d);
    return isFinite(v) ? v : NaN;
  }
  /** @param {HTMLInputElement} x @param {string} k */
  const bound = (x, k) => { const a = x.getAttribute(k) ?? x.getAttribute("data-" + k); const v = a == null || a === "" ? NaN : Number(a); return isFinite(v) ? v : null; };
  /** The refusal text for a field, or "" when the text is acceptable. @param {HTMLInputElement} x */
  function why(x) {
    const lo = bound(x, "min"), hi = bound(x, "max"), gt = bound(x, "gt");
    if (x.type === "number" && x.validity.badInput) return I18N.t("campo.num");
    const v = num(x.value);
    if (v === null) return x.hasAttribute("data-vuoto") ? "" : I18N.t("campo.empty");
    const out = Number.isNaN(v) || (lo != null && v < lo) || (hi != null && v > hi) || (gt != null && v <= gt);
    if (!out) return "";
    if (gt != null) return I18N.t("campo.gt", { a: gt });
    if (lo != null && hi != null) return I18N.t("campo.range", { a: lo, b: hi });
    if (lo != null) return I18N.t("campo.min", { a: lo });
    if (hi != null) return I18N.t("campo.max", { b: hi });
    return I18N.t("campo.num");
  }
  /** One note next to the field: under the panel header (or inside its popover) in a panel, after the field's label elsewhere; set after a clear so
   * a screen reader announces every refusal, hidden after 6 s. @param {string} msg @param {HTMLElement} x */
  function note(msg, x) {
    const pnl = /** @type {HTMLElement | null} */ (x.closest(".pnl")), pop = /** @type {HTMLElement | null} */ (x.closest(".cpop, .sp-pop"));
    const box = pop || pnl, at = /** @type {HTMLElement} */ (x.closest("label") || x);
    /** @type {HTMLElement | null} */
    let m = box ? box.querySelector(":scope > .xr-msg") : at.nextElementSibling?.classList.contains("campo-msg") ? /** @type {HTMLElement} */ (at.nextElementSibling) : null;
    if (!m) {
      m = document.createElement(box ? "div" : "span"); m.className = box ? (pop ? "xr-msg xr-in" : "xr-msg") : "campo-msg";
      m.setAttribute("role", "status"); m.setAttribute("aria-live", "polite");
      if (box) box.appendChild(m); else at.after(m);
    }
    const el = m, hd = pnl && !pop ? /** @type {HTMLElement | null} */ (pnl.querySelector(".hd")) : null;
    if (hd) el.style.top = hd.offsetTop + hd.offsetHeight + 2 + "px";
    el.hidden = false; el.textContent = "";
    setTimeout(() => { el.textContent = msg; }, 20);
    const t = /** @type {any} */ (el);
    clearTimeout(t._t); t._t = setTimeout(() => { el.hidden = true; x.removeAttribute("aria-invalid"); }, 6000);
  }
  /** @param {EventTarget | null} t */
  const field = t => t instanceof HTMLInputElement && (t.type === "number" || t.hasAttribute("data-num")) ? t : null;
  document.addEventListener("focusin", e => { const x = field(e.target); if (x && !last.has(x)) last.set(x, x.value); }, true);
  // capture on the document: a refused change never reaches the field's own handler (nor a request built from it)
  document.addEventListener("change", e => {
    const x = field(e.target); if (!x) return;
    const msg = why(x);
    if (!msg) { last.set(x, x.value); x.removeAttribute("aria-invalid"); return; }
    e.stopImmediatePropagation();
    x.value = last.get(x) ?? x.defaultValue;
    x.setAttribute("aria-invalid", "true");
    note(msg, x);
  }, true);
  /** @type {any} */ (window).CAMPI = { num, note, why };
})();
