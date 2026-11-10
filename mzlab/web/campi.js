// @ts-check
// One gate for typed numbers (<input type=number> or data-num) before the field's handlers: a refused text gets its old value back and one calm note.
// data-num, space-separated: «a:b» bounds, «>a» exclusive, «i» integer, «v» empty allowed, «<=ID» / «>=ID» against field #ID.
(() => {
  /** @type {WeakMap<HTMLInputElement, string>} */
  const last = new WeakMap(), fed = new WeakSet();   // the value before the first key of an edit; an edit live handlers saw
  /** number | null (empty) | NaN (letters, «1,2.3», 1e999) @param {unknown} t */
  function num(t) {
    const s = String(t ?? "").trim().replace(/−/g, "-"), c = s.split(",").length - 1;
    if (s === "") return null;
    if (c > 1 || (c && s.includes("."))) return NaN;
    const d = s.replace(",", "."), v = Number(d);
    return /^[-+]?(\d+\.?\d*|\.\d+)(e[-+]?\d+)?$/i.test(d) && isFinite(v) ? v : NaN;
  }
  /** @param {HTMLInputElement} x */
  function spec(x) {
    const n = (/** @type {any} */ a) => (a == null || a === "" || !isFinite(+a) ? null : +a), r = x.dataset.num;
    const o = { lo: n(x.getAttribute("min")), hi: n(x.getAttribute("max")), gt: /** @type {number | null} */ (null), int: false, empty: r == null, le: "", ge: "" };
    for (const t of (r || "").split(" ")) {
      if (t === "i") o.int = true; else if (t === "v") o.empty = true;
      else if (t.startsWith("<=")) o.le = t.slice(2); else if (t.startsWith(">=")) o.ge = t.slice(2);
      else if (t[0] === ">") o.gt = n(t.slice(1));
      else if (t.includes(":")) { const [a, b] = t.split(":"); o.lo = n(a); o.hi = n(b); }
    }
    return o;
  }
  /** the refusal text, or "" @param {HTMLInputElement} x */
  function why(x) {
    const s = spec(x), v = num(x.value), T = I18N.t;
    const ot = (/** @type {string} */ id) => id ? num(/** @type {any} */ (document.getElementById(id))?.value) : null;
    if (x.validity.badInput || Number.isNaN(v)) return T("campo.num");
    if (v === null) return s.empty ? "" : T("campo.empty");
    if (s.int && !Number.isInteger(v)) return T("campo.int");
    if (s.gt != null && v <= s.gt) return T("campo.gt", { a: s.gt });
    if ((s.lo != null && v < s.lo) || (s.hi != null && v > s.hi))
      return s.lo != null && s.hi != null ? T("campo.range", { a: s.lo, b: s.hi }) : s.lo != null ? T("campo.min", { a: s.lo }) : T("campo.max", { b: s.hi });
    const le = ot(s.le), ge = ot(s.ge);
    return (le != null && v > le) || (ge != null && v < ge) ? T("campo.order") : "";
  }
  /** under the panel header (in its popover), else after the label; set after a clear so it is read again @param {string} msg @param {HTMLElement} x */
  function note(msg, x) {
    const pnl = /** @type {HTMLElement | null} */ (x.closest(".pnl")), pop = x.closest(".cpop, .sp-pop"), box = pop || pnl, at = x.closest("label") || x;
    /** @type {any} */
    let m = box ? box.querySelector(":scope > .xr-msg") : at.nextElementSibling?.classList.contains("campo-msg") ? at.nextElementSibling : null;
    if (!m) {
      m = document.createElement(box ? "div" : "span"); m.className = box ? (pop ? "xr-msg xr-in" : "xr-msg") : "campo-msg";
      m.setAttribute("role", "status"); m.setAttribute("aria-live", "polite");
      if (box) box.appendChild(m); else at.after(m);
    }
    const hd = pnl && !pop ? /** @type {HTMLElement | null} */ (pnl.querySelector(".hd")) : null, el = m;
    if (hd) el.style.top = hd.offsetTop + hd.offsetHeight + 2 + "px";
    el.hidden = false; el.textContent = "";
    setTimeout(() => { el.textContent = msg; }, 20);
    clearTimeout(el._t); el._t = setTimeout(() => { el.hidden = true; x.removeAttribute("aria-invalid"); }, 6000);
  }
  /** @param {any} t @returns {HTMLInputElement | null} */
  const field = t => t instanceof HTMLInputElement && (t.type === "number" || t.dataset.num != null) ? t : null;
  /** @param {HTMLInputElement} x @param {string} msg */
  function refuse(x, msg) {
    x.value = last.get(x) ?? x.defaultValue; x.setAttribute("aria-invalid", "true");
    if (fed.has(x)) x.dispatchEvent(new Event("input", { bubbles: true }));   // live views follow the value that is back
    last.delete(x); fed.delete(x); note(msg, x);
  }
  const on = (/** @type {string} */ k, /** @type {(e: any, x: HTMLInputElement) => void} */ f) => document.addEventListener(k, e => { const x = field(e.target); if (x) f(e, x); }, true);
  on("focusin", (e, x) => { if (x.type !== "number") { x.inputMode ||= "decimal"; x.autocomplete = "off"; } last.delete(x); });
  on("beforeinput", (e, x) => { if (!last.has(x)) last.set(x, x.value); });
  on("input", (e, x) => { if (!e.isTrusted) return; if (why(x)) e.stopImmediatePropagation(); else fed.add(x); });
  on("change", (e, x) => { const m = why(x); if (!m) { last.delete(x); fed.delete(x); x.removeAttribute("aria-invalid"); } else { e.stopImmediatePropagation(); refuse(x, m); } });
  // Enter submits a form without a change
  document.addEventListener("submit", e => {
    const x = [.../** @type {HTMLFormElement} */ (e.target).querySelectorAll("input")].map(field).find(i => i && why(i));
    if (x) { e.preventDefault(); e.stopImmediatePropagation(); refuse(x, why(x)); x.focus(); }
  }, true);
  /** @type {any} */ (window).CAMPI = { num, note, why, spec };
})();
