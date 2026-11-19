// @ts-strict
// @ts-noany
// The molecules database in the page (molecules/1): loads the catalogue and the molecules of the user into the Rust engine once, keeps the records of the user in
// the archive behind the routes api/molecules/* (IndexedDB in the browser, a canonical JSON file in the app), and does the add / edit / delete / import / export.
// Every check, id, mass and conversion is done by the engine; this file only moves documents. Used by molecole-ui.js (the window) and molecole-campo.js (the search box).

/**
 * @typedef {{ type: string, catalog?: string[], file?: string, created?: string, modified?: string }} Origine
 * @typedef {{ id: string, name: string, synonyms?: string[], formula: string, charge?: number, mass?: number, smiles?: string | null, inchikey?: string | null,
 *   adducts?: string[], collections?: string[], classes?: string[], external?: { cas?: string, pubchem_cid?: string }, properties?: any[], notes?: string, origin: Origine }} Molecola
 * @typedef {{ line: number, key: string, text: string }} ErroreRiga
 * @typedef {{ id: string, name: string, formula: string, charge: number, mass: number, smiles: string | null, adducts: string[], score: number }} Trovata
 */

const SCHEMA = "molecules/1";
/** @type {Promise<void> | null} */
let caricato = null;
/** @type {Molecola[]} */
let mie = [];
/** @type {ErroreRiga[]} */
let erroriArchivio = [];

/** One route of the engine: the answer as text (the CSV of `to_csv` is not JSON). */
async function testo(/** @type {string} */ op, /** @type {any} */ corpo) {
  const r = await fetch("api/molecules/" + op, { method: "POST", body: typeof corpo === "string" ? corpo : JSON.stringify(corpo) });
  if (!r.ok) throw new Error(`api/molecules/${op}: ${r.status}`);
  return r.text();
}

const json = async (/** @type {string} */ op, /** @type {any} */ corpo) => JSON.parse(await testo(op, corpo));
const conErrori = (/** @type {any} */ j) => !!j && Array.isArray(j.errors) && j.errors.length > 0;

/** Loads the catalogue and the archive of the user once; a stored document that the engine refuses leaves the catalogue alone (see `errori`). */
export function pronto() {
  caricato = caricato || (async () => {
    const [cat, doc] = await Promise.all([
      fetch("static/molecole/catalogo.csv").then(r => { if (!r.ok) throw new Error("catalogo.csv: " + r.status); return r.text(); }),
      // an app older than the page has no archive yet: the catalogue still works, saving fails with a message
      testo("archive/read", "").catch(() => `{"schema":"${SCHEMA}","molecules":[]}`),
    ]);
    const r = await json("load", { catalog: cat, user: doc });
    if (conErrori(r)) {
      erroriArchivio = r.errors;
      mie = [];
      await json("load", { catalog: cat, user: { schema: SCHEMA, molecules: [] } });
    } else mie = JSON.parse(doc).molecules;
  })().catch(e => { caricato = null; throw e; });
  return caricato;
}

/** Errors of the stored document of the user, when the engine did not accept it at the start. */
export const errori = () => erroriArchivio.slice();
/** The records of the user (imported and own), ordered by id. */
export const mieMolecole = () => mie.slice();
/** The level of a molecule found by the search: the one of the user's copy when there is one, else the catalogue. */
export const livello = (/** @type {string} */ id) => (mie.find(m => m.id === id) || { origin: { type: "catalog" } }).origin.type;

export async function cerca(/** @type {string} */ q, limite = 8, ppm = 10, ampia = false) {
  await pronto();
  return /** @type {Promise<Trovata[]>} */ (json("search", ampia ? { q, limit: limite, ppm, wide: ampia } : { q, limit: limite, ppm }));
}

/** The names of the adduct table of the engine. */
export const addotti = async () => /** @type {Promise<string[]>} */ (json("adducts", {}));

/** What a typed formula (an ion with the atoms of `adduct` inside) or m/z is as a neutral molecule: `{kind, adduct, formula, mass, mz}`; `adduct` is `neutral` or a name of `addotti()`. Rejects with the key of the engine's error. */
export async function ione(/** @type {string} */ q, /** @type {string} */ adduct) {
  const r = await fetch("api/molecules/ion", { method: "POST", body: JSON.stringify({ q, adduct }) });
  const j = await r.json();
  if (!r.ok || j.error_key) throw Object.assign(new Error(j.error_key || "ione"), { key: j.error_key || "", params: j.params || {} });
  return /** @type {{ kind: "formula" | "mass", adduct: string, formula: string, mass: number, mz: number | null }} */ (j);
}

export async function prendi(/** @type {string} */ id) {
  await pronto();
  return /** @type {Promise<Molecola | null>} */ (json("get", { id }));
}

/** Writes the list as the whole archive of the user (engine check first) and reloads it into the engine. */
async function scrivi(/** @type {Molecola[]} */ lista) {
  const doc = await testo("to_json", { schema: SCHEMA, molecules: lista });
  const j = JSON.parse(doc);
  if (conErrori(j)) return { errori: /** @type {ErroreRiga[]} */ (j.errors) };
  await testo("archive/write", doc);
  await testo("load", { user: doc });
  mie = j.molecules;
  return { ok: true };
}

/**
 * Adds a molecule, or saves the edit of the one with `id` (an edit of a catalogue record is the copy of the user with the same id).
 * `esiste` = a new molecule whose id is already taken (same InChIKey): the user edits that one instead.
 * @returns {Promise<{ ok: true, id: string } | { errori: ErroreRiga[] } | { esiste: Molecola }>}
 */
export async function salva(/** @type {Partial<Molecola>} */ rec, /** @type {string} */ id = "") {
  await pronto();
  const prima = await json("to_json", { schema: SCHEMA, molecules: [{ ...rec, id }] });
  if (conErrori(prima)) return { errori: prima.errors };
  const finale = /** @type {Molecola} */ (prima.molecules[0]);
  if (!id && (mie.some(m => m.id === finale.id) || await prendi(finale.id))) return { esiste: finale };
  const r = await scrivi(mie.filter(m => m.id !== finale.id).concat(finale));
  return "ok" in r ? { ok: true, id: finale.id } : r;
}

/** Removes the record of the user; for a catalogue molecule that had a copy the catalogue one is back. */
export async function elimina(/** @type {string} */ id) {
  await pronto();
  return scrivi(mie.filter(m => m.id !== id));
}

/**
 * Imports a CSV or JSON file as records of the level «imported» (a CSV also as a collection named like the file; the short suspect columns name, formula, mass, comment are read). A molecule whose id the user already owns (own level) is kept as it is and counted in `tenute`;
 * a wrong row refuses the whole file with one error per row.
 * @returns {Promise<{ errori: ErroreRiga[] } | { n: number, sostituite: number, tenute: number }>}
 */
export async function importa(/** @type {string} */ nome, /** @type {string} */ contenuto) {
  await pronto();
  let doc;
  if (/\.json$/i.test(nome)) {
    let j;
    try { j = JSON.parse(contenuto); } catch (_) { return { errori: [{ line: 0, key: "error.molecules.badJson", text: "json" }] }; }
    for (const r of (j && Array.isArray(j.molecules) ? j.molecules : [])) if (r && typeof r === "object") r.origin = { type: "imported", file: nome };
    doc = await json("to_json", j);
  } else doc = await json("from_csv", { csv: contenuto, file: nome, level: "imported", collection: nome.replace(/\.[^.]*$/, "") });
  if (conErrori(doc)) return { errori: doc.errors };
  const nuove = /** @type {Molecola[]} */ (doc.molecules);
  const tenuteId = new Set(nuove.filter(m => mie.some(x => x.id === m.id && x.origin.type === "user")).map(m => m.id));
  const dentro = nuove.filter(m => !tenuteId.has(m.id));
  const ids = new Set(dentro.map(m => m.id));
  const sostituite = mie.filter(m => ids.has(m.id)).length;
  const r = await scrivi(mie.filter(m => !ids.has(m.id)).concat(dentro));
  return "ok" in r ? { n: dentro.length, sostituite, tenute: tenuteId.size } : r;
}

/** The records of the user as a file: `{ nome, testo, tipo }` (CSV or canonical JSON). */
export async function esporta(/** @type {"csv" | "json"} */ formato) {
  await pronto();
  const doc = await testo("to_json", { schema: SCHEMA, molecules: mie });
  return formato === "csv"
    ? { nome: "mzlab-molecules.csv", testo: await testo("to_csv", doc), tipo: "text/csv;charset=utf-8" }
    : { nome: "mzlab-molecules.json", testo: doc, tipo: "application/json;charset=utf-8" };
}
