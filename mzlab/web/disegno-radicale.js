// @ts-check
// The radical mark of the Drawing (item «Radical •» of the charge menu, disegno-barra.js). Ketcher has no button for it, only the atom dialog:
// with atoms selected the item puts the mark on them (or takes it off); without, it turns on a tool where a click (or a tap) on an atom does the same.
// The mark is written into the .ket (one undo step, Ketcher takes one H off as for `M  RAD` doublet); Esc or another tool turn the tool off.
import { ketIndex } from "./disegno-parziali.js";

/** the atom of `st` nearest to the point `p` (Ketcher coordinates, a bond is about 1), or null beyond `max` */
export function atomAt(/** @type {any} */ st, /** @type {{ x: number, y: number }} */ p, max = 0.45) {
  let best = /** @type {number | null} */ (null), d = max;
  st.atoms.forEach((/** @type {any} */ a, /** @type {number} */ id) => { const e = Math.hypot(a.pp.x - p.x, a.pp.y - p.y); if (e < d) { d = e; best = id; } });
  return best;
}

/** marks Ketcher's atoms `ids` as doublet radicals, or clears them when they all have one; false when the drawing could not be written */
export async function toggleRadicals(/** @type {any} */ K, /** @type {number[]} */ ids) {
  const st = K.editor.struct(), j = JSON.parse(await K.getKet()), at = ketIndex(j, st, ids);
  if (!at) return false;
  const clear = ids.every(id => st.atoms.get(id)?.radical);
  for (const { mol, idx } of at) {
    const a = j[mol].atoms[idx];
    if (clear) delete a.radical; else a.radical = 2;
    delete a.implicitHCount;                                                      // Ketcher counts the H again from the valence
  }
  await K.setMolecule(JSON.stringify(j));
  return true;
}

/**
 * The tool in Ketcher's document `d`. `changed` runs after every mark (labels, bar state).
 * @param {{ d: Document, K: () => any, changed: () => void }} o
 */
export function mountRadical(o) {
  let on = false;
  const set = (/** @type {boolean} */ v) => { on = v; o.d.documentElement.classList.toggle("tp-rad", v); o.changed(); };
  const mark = (/** @type {number[]} */ ids) => { toggleRadicals(o.K(), ids).then(() => o.changed(), () => o.changed()); };
  const el = (/** @type {EventTarget | null} */ t) => (t instanceof Element ? t : null);
  // a button of Ketcher's bars, or an item of the drop-down bar that is not this tool (the menu button of the bar only opens a menu)
  const other = (/** @type {EventTarget | null} */ t) => { const e = el(t); if (!e) return false; const m = e.closest("#tp-bar [role=menuitem]"); return m ? !m.hasAttribute("data-tp-own") : !!e.closest('[class*="Toolbar-module"] button,[data-testid$="-toolbar"] button') && !e.closest("#tp-bar"); };
  let swallow = false;
  o.d.addEventListener("pointerdown", e => {
    swallow = false;
    if (!on || el(e.target)?.closest('#tp-bar,[class*="Toolbar-module"],[role=dialog]')) return;
    const K = o.K(), p = K.editor.render.page2obj({ clientX: e.clientX, clientY: e.clientY }), id = atomAt(K.editor.struct(), p);
    if (id == null) return;
    e.preventDefault(); e.stopImmediatePropagation(); swallow = true; mark([id]);
  }, true);
  for (const k of ["pointerup", "mousedown", "mouseup", "click"]) o.d.addEventListener(k, e => { if (swallow) { e.stopImmediatePropagation(); e.preventDefault(); if (k === "click") swallow = false; } }, true);
  // another tool of Ketcher (its own buttons, or an item of the bar that clicks one) ends this one
  o.d.addEventListener("click", e => { if (on && other(e.target)) set(false); }, true);
  o.d.addEventListener("keydown", e => { if (on && e.key === "Escape") set(false); });
  document.addEventListener("keydown", e => { if (on && e.key === "Escape") set(false); });
  return {
    on: () => on,
    off: () => { if (on) set(false); },
    /** the item of the bar: the selected atoms get or lose the mark; with none selected the tool turns on (or off) */
    run: () => { const sel = o.K().editor.selection(), ids = sel && sel.atoms ? [...sel.atoms] : []; if (ids.length) { if (on) set(false); mark(ids); } else set(!on); },
  };
}
