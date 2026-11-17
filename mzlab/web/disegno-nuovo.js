// @ts-strict
// Draw: Ketcher's «New» (`clear-canvas`) and its keys (Ctrl/⌘+Delete, Ctrl/⌘+Backspace) ask before a non-empty drawing is cleared; an empty one is cleared at once.
// The question is `confirmDiscard` of disegno-pulito.js (alertdialog, Tab inside, Esc and a click outside cancel; «not saved» when the drawing has unsaved changes, else «Clear the drawing?»); the clear itself stays Ketcher's, so undo is unchanged.
import { confirmDiscard } from "./disegno-pulito.js";

const W = /** @type {any} */ (window);
const t = (/** @type {string} */ k) => (W.I18N ? W.I18N.t(k) : k);
const BTN = '[data-testid="clear-canvas"]';

/** @param {any} K @returns {boolean} true when nothing is drawn (structures, text, arrows, shapes, images) */
function blank(K) { try { return K.editor.struct().isBlank(); } catch (_) { return true; } }

/** @param {any} K @returns {Promise<boolean>} true = clear (also when the drawing is empty); the question is `confirmDiscard` of disegno-pulito.js */
function askNew(K) { return blank(K) ? Promise.resolve(true) : confirmDiscard("new"); }

/** @param {Document} d the document of Ketcher's iframe @param {any} K */
export function mountNew(d, K) {
  let pass = false;
  const clear = (/** @type {HTMLElement | null} */ b) => {              // the button clicked, or the shown one for the keys
    b = b || /** @type {HTMLElement | undefined} */ ([...d.querySelectorAll(BTN)].find(e => e.getClientRects().length)) || null;
    if (b) { pass = true; try { b.click(); } finally { pass = false; } } else K.editor.struct(null);
  };
  const ask = async (/** @type {Event} */ ev, /** @type {HTMLElement | null} */ b) => { ev.preventDefault(); ev.stopImmediatePropagation(); if (await askNew(K)) clear(b); };
  d.addEventListener("click", ev => {
    const tg = /** @type {any} */ (ev.target), b = tg && typeof tg.closest === "function" ? tg.closest(BTN) : null;     // the iframe has its own Element: no instanceof
    if (pass || !b || /** @type {HTMLButtonElement} */ (b).disabled || blank(K)) return;
    ask(ev, b);
  }, true);
  /** @type {Window} */ (d.defaultView).addEventListener("keydown", ev => {
    if (!(ev.ctrlKey || ev.metaKey) || ev.altKey || ev.shiftKey || (ev.key !== "Delete" && ev.key !== "Backspace")) return;
    const el = /** @type {any} */ (ev.target);                              // the same fields where Ketcher leaves the keys to the text
    if (el && el.hasAttribute && !el.hasAttribute("data-cliparea") && (["INPUT", "TEXTAREA", "SELECT"].includes(el.nodeName) || el.isContentEditable)) return;
    if (blank(K)) return;
    ask(ev, null);
  }, true);
}
