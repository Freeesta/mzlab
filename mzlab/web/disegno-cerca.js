// @ts-strict
// @ts-noany
// Quick draw box (Disegno): the SMILES field is also a search box over the molecules database (name, synonym, formula; the one search of the engine).
// A text that reads as a SMILES is never searched; choosing a suggestion puts its SMILES in the field and draws it with the «Draw» button, as a pasted SMILES.
import { campo } from "./molecole-campo.js";
import { pronto, cerca } from "./molecole-archivio.js";

/** True when `t` is made only of SMILES tokens (atoms, bonds, branches, ring digits, charges): "CC(=O)O", "c1ccccc1", "[Na+].[Cl-]". A name such as "caffeine" has letters no SMILES atom uses. */
export function looksLikeSmiles(/** @type {string} */ t) {
  return /^(?:Cl|Br|[BCNOPSFIcnops]|\[[^\]\s]+\]|[0-9%=#$:/\\.@+\-()*])+$/.test(t.trim()) && /[A-Za-z\[]/.test(t);
}

/** The typed text without accents, as the names of the database have none that matter ("caffeìna" finds "caffeina"). */
export const withoutAccents = (/** @type {string} */ t) => t.normalize("NFD").replace(/[̀-ͯ]/g, "");

/** True for a text that is drawn as typed: a SMILES or an InChI. Anything else is a compound name. */
export const isStructureText = (/** @type {string} */ t) => looksLikeSmiles(t) || /^InChI=/i.test(t.trim());

/** What `#ex-load` draws for the typed text: a SMILES or InChI as typed, a name as the SMILES of its first suggestion (as a click on it would); `null` (with the note shown) when nothing is found or the compound has no structure.
 * @param {string} text @param {(text: string) => void} note @param {(key: string) => string} t translator */
export async function nameToSmiles(text, note, t) {
  if (isStructureText(text)) return text;
  const r = await cerca(withoutAccents(text.trim()), 8, 5).catch(() => []);
  if (!r.length) { note(t("draw.quick.notFound")); return null; }
  if (!r[0].smiles) { /** @type {HTMLInputElement} */ (document.getElementById("ex-smi")).value = ""; note(t("draw.quick.noStructure")); return null; }
  return r[0].smiles;
}

/** Watches the paste events of the editor document: a single line that is neither a SMILES, an InChI nor a molfile/KET is a name (or junk) and is not handed to Ketcher, which would draw query atoms; `note` says so.
 * @param {Document} doc @param {() => void} note shows the «not found» note */
export function guardPaste(doc, note) {
  doc.addEventListener("paste", e => {
    const t = ((e.clipboardData && e.clipboardData.getData("text/plain")) || "").trim();
    if (!t || /[\r\n]/.test(t) || t.startsWith("{") || isStructureText(t)) return;
    e.preventDefault(); e.stopImmediatePropagation(); note();
  }, true);
}

let started = false;
/** Once, when the Disegno tab is first shown: turns `#ex-smi` into the combobox, Enter draws as the button does, and the database loads on the first focus (not before).
 * @param {(key: string, p?: Record<string, any>) => string} t translator
 * @param {(text: string) => void} note shows a message under the box (empty text clears it) */
export function initCerca(t, note) {
  if (started) return;
  started = true;
  const smi = /** @type {HTMLInputElement} */ (document.getElementById("ex-smi")), load = /** @type {HTMLButtonElement} */ (document.getElementById("ex-load"));
  campo(/** @type {HTMLElement} */ (smi.parentElement), {
    input: smi,
    testi: { label: t("draw.quick.label"), none: t("draw.quick.none"), count: n => t("draw.quick.count", { n }) },
    gestisci: null,
    limite: 8,
    ppm: 5,
    salta: looksLikeSmiles,
    interroga: async q => withoutAccents(q),
    onScegli: m => {
      if (!m.smiles) { smi.value = ""; note(t("draw.quick.noStructure")); return; }
      smi.value = m.smiles;
      load.click();
    },
  });
  smi.addEventListener("focus", () => { pronto().catch(() => { /* the search tries again when the user types */ }); }, { once: true });
  smi.addEventListener("keydown", e => { if (e.key === "Enter" && !e.defaultPrevented) load.click(); });   // after the combobox's own handler; a name with no highlighted row is resolved by the button's handler (first suggestion)
}
