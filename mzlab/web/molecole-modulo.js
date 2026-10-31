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
  v("nome", m ? m.nome : "");
  v("sinonimi", m ? (m.sinonimi || []).join("\n") : "");
  v("formula", m ? m.formula : "");
  v("carica", m && m.carica ? String(m.carica) : "");
  v("smiles", (m && m.smiles) || "");
  v("inchikey", (m && m.inchikey) || "");
  v("cas", (m && m.esterni && m.esterni.cas) || "");
  v("addotti", m ? (m.addotti || []).join(" ") : "");
  v("raccolte", m ? (m.raccolte || []).join("\n") : "");
  v("note", (m && m.note) || "");
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
  if (!Number.isInteger(carica)) return { errori: [{ riga: 0, chiave: "error.molecules.badCharge", testo: v("carica") }] };
  const oggi = new Date().toISOString().slice(0, 10);
  const esterni = { ...(base && base.esterni), cas: v("cas") || undefined };
  return {
    rec: {
      ...(base || {}), nome: v("nome"), sinonimi: righe("sinonimi"), formula: v("formula"), carica, smiles: v("smiles") || null, inchikey: v("inchikey") || null,
      addotti: v("addotti").split(/[\s,]+/).filter(Boolean), raccolte: righe("raccolte"), esterni, note: v("note"),
      origine: { tipo: "utente", creato: (base && base.origine.tipo === "utente" && base.origine.creato) || oggi, modificato: oggi },
    },
  };
}
