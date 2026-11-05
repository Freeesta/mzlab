// @ts-strict
"use strict";
// Names of contaminants/compounds in the ion field of «Extract ions» (lazy, carica("xic-nomi.js")): suggestions under the field and the reading of a name (with its shift)
// through the lists (liste-nomi.js). A name only fills the formula: a candidate input, never an identification. If the lists do not load, formulas and m/z work as before.
const XICNOMI = (() => {
  let /** @type {any} */ sug = null, /** @type {HTMLInputElement | null} */ on = null;
  const load = () => Promise.all([carica("ask-suggerisci.js"), carica("liste-nomi.js")]).then(() => LISTE.ready());
  const SHIFT = /\s*([+-]\s*[\d.,]*)\s*$/;   // the signed number closing the field (the engine reads it; here it is only kept or cut)
  /** The text the engine could not read as a formula (e = its error): {text: formula + shift, adduct} when its head is exactly one name of the lists, else null. */
  async function resolve(/** @type {string} */ t, /** @type {Bag} */ e) {
    const p = e.params || {}, head = p.head || t, sh = p.offset == null ? "" : (String(p.offset).startsWith("-") ? "" : "+") + p.offset;
    try { await load(); } catch (_) { return null; }
    const nm = LISTENOMI.byName(head);
    return nm ? { text: nm.formula + sh, adduct: nm.adduct } : null;
  }
  /** The answer of api/formula for a text the engine refused: a name of the lists (adduct of the row, else the one of the list) with its shift; else the first error stands. */
  async function read(/** @type {string} */ t, /** @type {string} */ ad, /** @type {Bag} */ e) {
    if (e.key === "err.formula.offset") throw e;
    const n = await resolve(t, e); if (!n) throw e;
    return getFormula(n.text, ad === "none" && n.adduct ? n.adduct : ad);
  }
  async function focus(/** @type {HTMLInputElement} */ inp) {
    try { await load(); } catch (_) { return; }
    if (document.activeElement !== inp) return;
    blur(on); on = inp;
    sug = ASKSUG.attach(inp, {
      rows: (/** @type {string} */ q) => LISTENOMI.suggest(q.replace(SHIFT, "")),
      text: (/** @type {Bag} */ r) => { const sh = (inp.value.match(SHIFT) || ["", ""])[1].replace(/\s+/g, ""); setTimeout(() => inp.dispatchEvent(new Event("input", { bubbles: true }))); return r.formula + (/\d/.test(sh) ? sh : ""); },
      ready: () => LISTE.ready()
    });
    const ul = document.getElementById("asksug"), row = inp.closest(".xrw");   // the list sits under the row, across its grid
    if (ul && row) { ul.style.gridColumn = "1 / -1"; row.insertBefore(ul, row.querySelector(".xr-sum")); }
  }
  function blur(/** @type {HTMLInputElement | null} */ inp) { if (sug && inp === on) { sug.detach(); sug = null; on = null; } }
  const key = (/** @type {KeyboardEvent} */ e) => !!sug && e.target === on && sug.key(e);
  return { read, resolve, focus, blur, key };
})();
