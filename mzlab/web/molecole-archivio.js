// @ts-check
// The molecules database in the page (molecules/1): loads the catalogue and the molecules of the user into the Rust engine once, keeps the records of the user in
// the archive behind the routes api/molecules/* (IndexedDB in the browser, a canonical JSON file in the app), and does the add / edit / delete / import / export.
// Every check, id, mass and conversion is done by the engine; this file only moves documents. Used by molecole-ui.js (the window) and molecole-campo.js (the search box).

/**
 * @typedef {{ tipo: string, catalogo?: string[], file?: string, creato?: string, modificato?: string }} Origine
 * @typedef {{ id: string, nome: string, sinonimi?: string[], formula: string, carica?: number, massa?: number, smiles?: string | null, inchikey?: string | null,
 *   addotti?: string[], raccolte?: string[], classi?: string[], esterni?: { cas?: string, pubchem_cid?: string }, proprieta?: any[], note?: string, origine: Origine }} Molecola
 * @typedef {{ riga: number, chiave: string, testo: string }} ErroreRiga
 * @typedef {{ id: string, nome: string, formula: string, carica: number, massa: number, smiles: string | null, addotti: string[], punteggio: number }} Trovata
 */

const SCHEMA = "molecules/1";
/** @type {Promise<void> | null} */
let caricato = null;
/** @type {Molecola[]} */
let mie = [];
/** @type {ErroreRiga[]} */
let erroriArchivio = [];

/** One route of the engine: the answer as text (the CSV of `a_csv` is not JSON). */
async function testo(/** @type {string} */ op, /** @type {any} */ corpo) {
  const r = await fetch("api/molecules/" + op, { method: "POST", body: typeof corpo === "string" ? corpo : JSON.stringify(corpo) });
  if (!r.ok) throw new Error(`api/molecules/${op}: ${r.status}`);
  return r.text();
}

const json = async (/** @type {string} */ op, /** @type {any} */ corpo) => JSON.parse(await testo(op, corpo));
const conErrori = (/** @type {any} */ j) => !!j && Array.isArray(j.errori) && j.errori.length > 0;

/** Loads the catalogue and the archive of the user once; a stored document that the engine refuses leaves the catalogue alone (see `errori`). */
export function pronto() {
  caricato = caricato || (async () => {
    const [cat, doc] = await Promise.all([
      fetch("static/molecole/catalogo.csv").then(r => { if (!r.ok) throw new Error("catalogo.csv: " + r.status); return r.text(); }),
      // an app older than the page has no archive yet: the catalogue still works, saving fails with a message
      testo("archivio/leggi", "").catch(() => `{"molecole":[],"schema":"${SCHEMA}"}`),
    ]);
    const r = await json("carica", { catalogo: cat, utente: doc });
    if (conErrori(r)) {
      erroriArchivio = r.errori;
      mie = [];
      await json("carica", { catalogo: cat, utente: { schema: SCHEMA, molecole: [] } });
    } else mie = JSON.parse(doc).molecole;
  })().catch(e => { caricato = null; throw e; });
  return caricato;
}

/** Errors of the stored document of the user, when the engine did not accept it at the start. */
export const errori = () => erroriArchivio.slice();
/** The records of the user (imported and own), ordered by id. */
export const mieMolecole = () => mie.slice();
/** The level of a molecule found by the search: the one of the user's copy when there is one, else the catalogue. */
export const livello = (/** @type {string} */ id) => (mie.find(m => m.id === id) || { origine: { tipo: "catalogo" } }).origine.tipo;

export async function cerca(/** @type {string} */ q, limite = 8, ppm = 10) {
  await pronto();
  return /** @type {Promise<Trovata[]>} */ (json("cerca", { q, limite, ppm }));
}

export async function prendi(/** @type {string} */ id) {
  await pronto();
  return /** @type {Promise<Molecola | null>} */ (json("get", { id }));
}

/** Writes the list as the whole archive of the user (engine check first) and reloads it into the engine. */
async function scrivi(/** @type {Molecola[]} */ lista) {
  const doc = await testo("a_json", { schema: SCHEMA, molecole: lista });
  const j = JSON.parse(doc);
  if (conErrori(j)) return { errori: /** @type {ErroreRiga[]} */ (j.errori) };
  await testo("archivio/scrivi", doc);
  await testo("carica", { utente: doc });
  mie = j.molecole;
  return { ok: true };
}

/**
 * Adds a molecule, or saves the edit of the one with `id` (an edit of a catalogue record is the copy of the user with the same id).
 * `esiste` = a new molecule whose id is already taken (same InChIKey): the user edits that one instead.
 * @returns {Promise<{ ok: true, id: string } | { errori: ErroreRiga[] } | { esiste: Molecola }>}
 */
export async function salva(/** @type {Partial<Molecola>} */ rec, /** @type {string} */ id = "") {
  await pronto();
  const prima = await json("a_json", { schema: SCHEMA, molecole: [{ ...rec, id }] });
  if (conErrori(prima)) return { errori: prima.errori };
  const finale = /** @type {Molecola} */ (prima.molecole[0]);
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
    try { j = JSON.parse(contenuto); } catch (_) { return { errori: [{ riga: 0, chiave: "error.molecules.badJson", testo: "json" }] }; }
    for (const r of (j && Array.isArray(j.molecole) ? j.molecole : [])) if (r && typeof r === "object") r.origine = { tipo: "importata", file: nome };
    doc = await json("a_json", j);
  } else doc = await json("da_csv", { csv: contenuto, file: nome, livello: "importata", raccolta: nome.replace(/\.[^.]*$/, "") });
  if (conErrori(doc)) return { errori: doc.errori };
  const nuove = /** @type {Molecola[]} */ (doc.molecole);
  const tenuteId = new Set(nuove.filter(m => mie.some(x => x.id === m.id && x.origine.tipo === "utente")).map(m => m.id));
  const dentro = nuove.filter(m => !tenuteId.has(m.id));
  const ids = new Set(dentro.map(m => m.id));
  const sostituite = mie.filter(m => ids.has(m.id)).length;
  const r = await scrivi(mie.filter(m => !ids.has(m.id)).concat(dentro));
  return "ok" in r ? { n: dentro.length, sostituite, tenute: tenuteId.size } : r;
}

/** The records of the user as a file: `{ nome, testo, tipo }` (CSV or canonical JSON). */
export async function esporta(/** @type {"csv" | "json"} */ formato) {
  await pronto();
  const doc = await testo("a_json", { schema: SCHEMA, molecole: mie });
  return formato === "csv"
    ? { nome: "mzlab-molecules.csv", testo: await testo("a_csv", doc), tipo: "text/csv;charset=utf-8" }
    : { nome: "mzlab-molecules.json", testo: doc, tipo: "application/json;charset=utf-8" };
}
