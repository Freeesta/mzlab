// @ts-strict
// @ts-noany
// The form of the window «Molecules» (molecules-ui.js): builds the fields of a molecule, fills them from a record and reads them back. The record goes to the engine
// (molecules-archive.js), which checks it, computes the mass and gives the id: the form only collects text.

/** @typedef {import("./molecules-archive.js").Molecule} Molecule */
/** @typedef {import("./molecules-archive.js").RowError} RowError */

/** @type {{ k: string, key: string, req?: boolean, area?: boolean, type?: string, hint?: string }[]} */
const FIELDS = [
  { k: "name", key: "mol.f.name", req: true },
  { k: "synonyms", key: "mol.f.synonyms", area: true, hint: "mol.f.perLine" },
  { k: "formula", key: "mol.f.formula", req: true },
  { k: "charge", key: "mol.f.charge", type: "number" },
  { k: "smiles", key: "mol.f.smiles" },
  { k: "inchikey", key: "mol.f.inchikey" },
  { k: "cas", key: "mol.f.cas" },
  { k: "adducts", key: "mol.f.adducts", hint: "mol.f.adductsHint" },
  { k: "collections", key: "mol.f.collections", area: true, hint: "mol.f.perLine" },
  { k: "notes", key: "mol.f.notes", area: true },
];

/** @param {(k: string, p?: any) => string} t */
export function fieldsHtml(t) {
  const field = (/** @type {typeof FIELDS[number]} */ c) => {
    const id = "mol-f-" + c.k, hint = c.hint ? `<span class="mol-hint" id="${id}-h"></span>` : "";
    const el = c.area ? `<textarea id="${id}" rows="2"${c.hint ? ` aria-describedby="${id}-h"` : ""}></textarea>`
      : `<input id="${id}" type="${c.type || "text"}"${c.type === "number" ? ' step="1"' : ""} autocomplete="off" spellcheck="false"${c.req ? ' aria-required="true"' : ""}${c.hint ? ` aria-describedby="${id}-h"` : ""}>`;
    return `<div class="mol-fld${c.area || c.k === "name" ? " mol-wide" : ""}"><label for="${id}"><span class="mol-lt"></span>${c.req ? ' <span aria-hidden="true">*</span>' : ""}</label>${hint}${el}</div>`;
  };
  return `<div class="mol-grid">${FIELDS.map(field).join("")}</div><p class="mol-hint">${t("mol.f.required")}</p>`;
}

/** Puts the texts of the labels and hints in the form (called at every opening, so the language follows). */
export function formTexts(/** @type {HTMLElement} */ form, /** @type {(k: string, p?: any) => string} */ t) {
  for (const c of FIELDS) {
    const lt = form.querySelector(`label[for="mol-f-${c.k}"] .mol-lt`), h = form.querySelector(`#mol-f-${c.k}-h`);
    if (lt) lt.textContent = t(c.key);
    if (h && c.hint) h.textContent = t(c.hint);
  }
}

const fieldOf = (/** @type {HTMLElement} */ form, /** @type {string} */ k) => /** @type {HTMLInputElement} */ (form.querySelector("#mol-f-" + k));

/** Fills the form from a record (an empty form for a new molecule). */
export function fill(/** @type {HTMLElement} */ form, /** @type {Molecule | null} */ m) {
  const v = (/** @type {string} */ k, /** @type {string} */ x) => { fieldOf(form, k).value = x; };
  v("name", m ? m.name : "");
  v("synonyms", m ? (m.synonyms || []).join("\n") : "");
  v("formula", m ? m.formula : "");
  v("charge", m && m.charge ? String(m.charge) : "");
  v("smiles", (m && m.smiles) || "");
  v("inchikey", (m && m.inchikey) || "");
  v("cas", (m && m.external && m.external.cas) || "");
  v("adducts", m ? (m.adducts || []).join(" ") : "");
  v("collections", m ? (m.collections || []).join("\n") : "");
  v("notes", (m && m.notes) || "");
}

/**
 * Reads the form back into a record: the fields that the form does not show (classes, properties, PubChem CID) stay as they were in `base`; the record is of the
 * user's level, created today if new, modified today. A charge that is not a whole number is an error (the engine would read it as 0).
 * @returns {{ rec: Partial<Molecule> } | { errors: RowError[] }}
 */
export function read(/** @type {HTMLElement} */ form, /** @type {Molecule | null} */ base) {
  const v = (/** @type {string} */ k) => fieldOf(form, k).value.trim();
  const lines = (/** @type {string} */ k) => v(k).split(/\r?\n/).map(s => s.trim()).filter(Boolean);
  const charge = v("charge") === "" ? 0 : Number(v("charge"));
  if (!Number.isInteger(charge)) return { errors: [{ line: 0, key: "error.molecules.badCharge", text: v("charge") }] };
  const today = new Date().toISOString().slice(0, 10);
  const external = { ...(base && base.external), cas: v("cas") || undefined };
  return {
    rec: {
      ...(base || {}), name: v("name"), synonyms: lines("synonyms"), formula: v("formula"), charge: charge, smiles: v("smiles") || null, inchikey: v("inchikey") || null,
      adducts: v("adducts").split(/[\s,]+/).filter(Boolean), collections: lines("collections"), external, notes: v("notes"),
      origin: { type: "user", created: (base && base.origin.type === "user" && base.origin.created) || today, modified: today },
    },
  };
}
