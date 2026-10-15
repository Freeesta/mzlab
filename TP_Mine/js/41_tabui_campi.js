// @ts-check
// TPMINE-PRIVATE  TPFIELD: the error of a field of Tabui. `set(fields, msgEl)` marks every field of the group aria-invalid="true" (red double border and a bar on the left,
// styled in 20_tpmine.js, so the colour is never alone) and links it to the message with aria-describedby; the group returns to normal as soon as the user clicks or types
// in any of its fields (`watch`), or with `clear`. One rule for every field error of Tabui.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  /** @type {Map<HTMLElement, HTMLElement[]>} */ const groups = new Map();
  /** @param {HTMLElement[]} fields */
  const clear = fields => { for (const f of fields) { f.removeAttribute("aria-invalid"); if (f.dataset.tfDesc !== undefined) { if (f.dataset.tfDesc) f.setAttribute("aria-describedby", f.dataset.tfDesc); else f.removeAttribute("aria-describedby"); delete f.dataset.tfDesc; } } };
  /** @param {HTMLElement[]} fields @param {HTMLElement | null} [msg] */
  const set = (fields, msg) => {
    for (const f of fields) {
      f.setAttribute("aria-invalid", "true");
      if (msg && msg.id) { if (f.dataset.tfDesc === undefined) f.dataset.tfDesc = f.getAttribute("aria-describedby") || ""; f.setAttribute("aria-describedby", msg.id); }
      groups.set(f, fields);
    }
  };
  /** Clears the whole group of the field that was clicked or typed in. @param {HTMLElement} root @param {() => void} [after] */
  const watch = (root, after) => {
    const go = (/** @type {Event} */ ev) => { const f = ev.target instanceof HTMLElement ? ev.target.closest("[aria-invalid=true]") : null; const g = f && groups.get(/** @type {HTMLElement} */ (f)); if (g) { clear(g); if (after) after(); } };
    root.addEventListener("pointerdown", go); root.addEventListener("input", go);
  };
  G.TPFIELD = { set, clear, watch };
})();
