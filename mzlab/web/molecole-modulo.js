// @ts-strict
// @ts-noany
// The form of the window «Molecules» (molecole-ui.js): builds the fields of a molecule, fills them from a record and reads them back. The record goes to the engine
// (molecole-archivio.js), which checks it, computes the mass and gives the id: the form only collects text.

/** @typedef {import("./molecole-archivio.js").Molecola} Molecola */
/** @typedef {import("./molecole-archivio.js").ErroreRiga} ErroreRiga */

/** @type {{ k: string, key: string, req?: boolean, area?: boolean, type?: string, hint?: string }[]} */
const CAMPI = [
  { k: "nome", key: "mol.f.name", req: true },
  { k: "sinonimi", key: "mol.f.synonyms", area: true, hint: "mol.f.perLine" },
  { k: "formula", key: "mol.f.formula", req: true },
  { k: "carica", key: "mol.f.charge", type: "number" },
  { k: "smiles", key: "mol.f.smiles" },
  { k: "inchikey", key: "mol.f.inchikey" },
  { k: "cas", key: "mol.f.cas" },
  { k: "addotti", key: "mol.f.adducts", hint: "mol.f.adductsHint" },
  { k: "raccolte", key: "mol.f.collections", area: true, hint: "mol.f.perLine" },
  { k: "note", key: "mol.f.notes", area: true },
];

/** @param {(k: string, p?: any) => string} t */
export function campiHtml(t) {
  const campo = (/** @type {typeof CAMPI[number]} */ c) => {
    const id = "mol-f-" + c.k, hint = c.hint ? `<span class="mol-hint" id="${id}-h"></span>` : "";
    const el = c.area ? `<textarea id="${id}" rows="2"${c.hint ? ` aria-describedby="${id}-h"` : ""}></textarea>`
      : `<input id="${id}" type="${c.type || "text"}"${c.type === "number" ? ' step="1"' : ""} autocomplete="off" spellcheck="false"${c.req ? ' aria-required="true"' : ""}${c.hint ? ` aria-describedby="${id}-h"` : ""}>`;
    return `<div class="mol-fld${c.area || c.k === "nome" ? " mol-wide" : ""}"><label for="${id}"><span class="mol-lt"></span>${c.req ? ' <span aria-hidden="true">*</span>' : ""}</label>${hint}${el}</div>`;
  };
  return `<div class="mol-grid">${CAMPI.map(campo).join("")}</div><p class="mol-hint">${t("mol.f.required")}</p>`;
}

/** Puts the texts of the labels and hints in the form (called at every opening, so the language follows). */
export function testiModulo(/** @type {HTMLElement} */ form, /** @type {(k: string, p?: any) => string} */ t) {
  for (const c of CAMPI) {
    const lt = form.querySelector(`label[for="mol-f-${c.k}"] .mol-lt`), h = form.querySelector(`#mol-f-${c.k}-h`);
    if (lt) lt.textContent = t(c.key);
    if (h && c.hint) h.textContent = t(c.hint);
  }
}

const campoDi = (/** @type {HTMLElement} */ form, /** @type {string} */ k) => /** @type {HTMLInputElement} */ (form.querySelector("#mol-f-" + k));

/** Fills the form from a record (an empty form for a new molecule). */
export function riempi(/** @type {HTMLElement} */ form, /** @type {Molecola | null} */ m) {
  const v = (/** @type {string} */ k, /** @type {string} */ x) => { campoDi(form, k).value = x; };
  v("nome", m ? m.name : "");
  v("sinonimi", m ? (m.synonyms || []).join("\n") : "");
  v("formula", m ? m.formula : "");
  v("carica", m && m.charge ? String(m.charge) : "");
  v("smiles", (m && m.smiles) || "");
  v("inchikey", (m && m.inchikey) || "");
  v("cas", (m && m.external && m.external.cas) || "");
  v("addotti", m ? (m.adducts || []).join(" ") : "");
  v("raccolte", m ? (m.collections || []).join("\n") : "");
  v("note", (m && m.notes) || "");
}

/**
 * Reads the form back into a record: the fields that the form does not show (classes, properties, PubChem CID) stay as they were in `base`; the record is of the
 * user's level, created today if new, modified today. A charge that is not a whole number is an error (the engine would read it as 0).
 * @returns {{ rec: Partial<Molecola> } | { errori: ErroreRiga[] }}
 */
export function leggi(/** @type {HTMLElement} */ form, /** @type {Molecola | null} */ base) {
  const v = (/** @type {string} */ k) => campoDi(form, k).value.trim();
  const righe = (/** @type {string} */ k) => v(k).split(/\r?\n/).map(s => s.trim()).filter(Boolean);
  const carica = v("carica") === "" ? 0 : Number(v("carica"));
  if (!Number.isInteger(carica)) return { errori: [{ line: 0, key: "error.molecules.badCharge", text: v("carica") }] };
  const oggi = new Date().toISOString().slice(0, 10);
  const external = { ...(base && base.external), cas: v("cas") || undefined };
  return {
    rec: {
      ...(base || {}), name: v("nome"), synonyms: righe("sinonimi"), formula: v("formula"), charge: carica, smiles: v("smiles") || null, inchikey: v("inchikey") || null,
      adducts: v("addotti").split(/[\s,]+/).filter(Boolean), collections: righe("raccolte"), external, notes: v("note"),
      origin: { type: "user", created: (base && base.origin.type === "user" && base.origin.created) || oggi, modified: oggi },
    },
  };
}
