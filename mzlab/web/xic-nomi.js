// @ts-strict
"use strict";
// Names of contaminants/compounds in the ion field of «Extract ions» (lazy, carica("xic-nomi.js")): suggestions under the field and the reading of a name (with its shift)
// through the lists (liste-nomi.js). A name only fills the formula: a candidate input, never an identification. If the lists do not load, formulas and m/z work as before.
const XICNOMI = (() => {
  let /** @type {any} */ sug = null, /** @type {HTMLInputElement | null} */ on = null;
  const load = () => Promise.all([carica("ask-suggerisci.js"), carica("liste-nomi.js")]).then(() => LISTE.ready());
  const CALM = new Set(["err.formula.offset", "err.formula.mzLow"]);   // errors of the engine shown as they are (not a name to look up, not «invalid formula»)
  const SHIFT = /\s*([+-]\s*[\d.,]*)\s*$/;   // the signed number closing the field (the engine reads it; here it is only kept or cut)
  /** The text the engine could not read as a formula (e = its error): {text: formula + shift, adduct} when its head is exactly one name of the lists, else null. */
  async function resolve(/** @type {string} */ t, /** @type {Bag} */ e) {
    const p = e.params || {}, head = p.head || t, sh = p.offset == null ? "" : (String(p.offset).startsWith("-") ? "" : "+") + p.offset;
    try { await load(); } catch (_) { return null; }
    const nm = LISTENOMI.byName(head);
    return nm ? { text: nm.formula + sh, adduct: nm.adduct } : null;
  }
  /** The answer of api/formula for a text the engine refused: a name of the lists (adduct of the row, else the one of the list) with its shift; else the first error stands. */
  async function ask(/** @type {string} */ t, /** @type {string} */ ad) {
    try { return await getFormula(t, ad); } catch (e_) {
      const e = /** @type {Bag} */ (e_); if (CALM.has(e.key)) throw e;
      const n = await resolve(t, e); if (!n) throw e;
      return getFormula(n.text, ad === "none" && n.adduct ? n.adduct : ad);
    }
  }
  /** What a row of «Extract ions» takes from the text: label, unit window, exact m/z and the note under the field. A signed shift closing the text (+16, -18) was added by the
   *  engine to the m/z after the adduct as literal Da; the result is a candidate to extract, never an identification. */
  async function row(/** @type {string} */ t, /** @type {string} */ ad, /** @type {unknown} */ hr) {
    const f = await ask(t, ad).catch((/** @type {Bag} */ e) => { throw CALM.has(e.key) ? e : new Error(I18N.t("xic.badFormula")); });
    if (f.range) {  // «250.8-251.5»: a range of m/z written by the student, extracted as it is (not a unit window, not a shift)
      const d = hr ? 4 : 1, [lo, hi] = f.range.map((/** @type {number} */ v) => rh(v, d));
      if (!(hi > lo)) throw new Error(I18N.t("xic.rangeBad"));
      return { label: "", range: true, win: [lo, hi], ex: null, note: I18N.t("xic.range") + " " };
    }
    const off = f.offset ?? null, sh = off == null ? "" : (off < 0 ? "-" : "+") + String(Math.abs(off));
    const interp = f.formula && off == null && t.replace(/\s+/g, "") !== f.formula ? I18N.t("xic.interpreted", { formula: fmtFormula(f.formula) }) + " " : "";
    const shown = off == null ? "" : I18N.t("xic.shifted", { base: String(rh(f.mz - off, 4)), shift: sh, mz: String(rh(f.mz, 4)) }) + " ";
    return { label: !f.formula ? t.replace(/\s+/g, "") : (f.adduct ? `${f.formula} ${f.adduct}` : `${f.formula} ion`) + sh, win: xicWin(f.mz, false), ex: hr ? f.mz5 ?? f.mz : null, note: interp + shown };
  }
  const wired = new WeakSet();
  /** On the focus of a row's field: the list's keys come before the field's own, the list goes away on blur, the suggestions are attached. */
  function wire(/** @type {HTMLInputElement} */ inp) {
    if (!wired.has(inp)) {
      wired.add(inp);
      inp.addEventListener("keydown", e => { if (key(e)) e.stopImmediatePropagation(); }, true);
      inp.addEventListener("blur", () => blur(inp));
    }
    return focus(inp);
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
  return { row, wire };
})();
