// @ts-strict
// @ts-noany
// The molecules database in the page (molecules/1): loads the catalogue and the molecules of the user into the Rust engine once, keeps the records of the user in
// the archive behind the routes api/molecules/* (IndexedDB in the browser, a canonical JSON file in the app), and does the add / edit / delete / import / export.
// Every check, id, mass and conversion is done by the engine; this file only moves documents. Used by molecules-ui.js (the window) and molecules-field.js (the search box).

/**
 * @typedef {{ type: string, catalog?: string[], file?: string, created?: string, modified?: string }} Origin
 * @typedef {{ id: string, name: string, synonyms?: string[], formula: string, charge?: number, mass?: number, smiles?: string | null, inchikey?: string | null,
 *   adducts?: string[], collections?: string[], classes?: string[], external?: { cas?: string, pubchem_cid?: string }, properties?: any[], notes?: string, origin: Origin }} Molecule
 * @typedef {{ line: number, key: string, text: string }} RowError
 * @typedef {{ id: string, name: string, formula: string, charge: number, mass: number, smiles: string | null, adducts: string[], score: number }} Found
 */

const SCHEMA = "molecules/1";
/** @type {Promise<void> | null} */
let loaded = null;
/** @type {Molecule[]} */
let mine = [];
/** @type {RowError[]} */
let loadErrors = [];

/** One route of the engine: the answer as text (the CSV of `to_csv` is not JSON). */
async function text(/** @type {string} */ op, /** @type {any} */ body) {
  const r = await fetch("api/molecules/" + op, { method: "POST", body: typeof body === "string" ? body : JSON.stringify(body) });
  if (!r.ok) throw new Error(`api/molecules/${op}: ${r.status}`);
  return r.text();
}

const json = async (/** @type {string} */ op, /** @type {any} */ body) => JSON.parse(await text(op, body));
const hasErrors = (/** @type {any} */ j) => !!j && Array.isArray(j.errors) && j.errors.length > 0;

/** Loads the catalogue and the archive of the user once; a stored document that the engine refuses leaves the catalogue alone (see `errors`). */
export function ready() {
  loaded = loaded || (async () => {
    const [cat, doc] = await Promise.all([
      fetch("static/molecole/catalogo.csv").then(r => { if (!r.ok) throw new Error("catalogo.csv: " + r.status); return r.text(); }),
      // an app older than the page has no archive yet: the catalogue still works, saving fails with a message
      text("archive/read", "").catch(() => `{"schema":"${SCHEMA}","molecules":[]}`),
    ]);
    const r = await json("load", { catalog: cat, user: doc });
    if (hasErrors(r)) {
      loadErrors = r.errors;
      mine = [];
      await json("load", { catalog: cat, user: { schema: SCHEMA, molecules: [] } });
    } else mine = JSON.parse(doc).molecules;
  })().catch(e => { loaded = null; throw e; });
  return loaded;
}

/** Errors of the stored document of the user, when the engine did not accept it at the start. */
export const errors = () => loadErrors.slice();
/** The records of the user (imported and own), ordered by id. */
export const myMolecules = () => mine.slice();
/** The level of a molecule found by the search: the one of the user's copy when there is one, else the catalogue. */
export const level = (/** @type {string} */ id) => (mine.find(m => m.id === id) || { origin: { type: "catalog" } }).origin.type;

export async function search(/** @type {string} */ q, limit = 8, ppm = 10, wide = false) {
  await ready();
  return /** @type {Promise<Found[]>} */ (json("search", wide ? { q, limit, ppm, wide } : { q, limit, ppm }));
}

/** The names of the adduct table of the engine. */
export const adducts = async () => /** @type {Promise<string[]>} */ (json("adducts", {}));

/** What a typed formula (an ion with the atoms of `adduct` inside) or m/z is as a neutral molecule: `{kind, adduct, formula, mass, mz}`; `adduct` is `neutral` or a name of `adducts()`. Rejects with the key of the engine's error. */
export async function ion(/** @type {string} */ q, /** @type {string} */ adduct) {
  const r = await fetch("api/molecules/ion", { method: "POST", body: JSON.stringify({ q, adduct }) });
  const j = await r.json();
  if (!r.ok || j.error_key) throw Object.assign(new Error(j.error_key || "ion"), { key: j.error_key || "", params: j.params || {} });
  return /** @type {{ kind: "formula" | "mass", adduct: string, formula: string, mass: number, mz: number | null }} */ (j);
}

export async function getMolecule(/** @type {string} */ id) {
  await ready();
  return /** @type {Promise<Molecule | null>} */ (json("get", { id }));
}

/** Writes the list as the whole archive of the user (engine check first) and reloads it into the engine. */
async function write(/** @type {Molecule[]} */ list) {
  const doc = await text("to_json", { schema: SCHEMA, molecules: list });
  const j = JSON.parse(doc);
  if (hasErrors(j)) return { errors: /** @type {RowError[]} */ (j.errors) };
  await text("archive/write", doc);
  await text("load", { user: doc });
  mine = j.molecules;
  return { ok: true };
}

/**
 * Adds a molecule, or saves the edit of the one with `id` (an edit of a catalogue record is the copy of the user with the same id).
 * `exists` = a new molecule whose id is already taken (same InChIKey): the user edits that one instead.
 * @returns {Promise<{ ok: true, id: string } | { errors: RowError[] } | { exists: Molecule }>}
 */
export async function save(/** @type {Partial<Molecule>} */ rec, /** @type {string} */ id = "") {
  await ready();
  const before = await json("to_json", { schema: SCHEMA, molecules: [{ ...rec, id }] });
  if (hasErrors(before)) return { errors: before.errors };
  const final = /** @type {Molecule} */ (before.molecules[0]);
  if (!id && (mine.some(m => m.id === final.id) || await getMolecule(final.id))) return { exists: final };
  const r = await write(mine.filter(m => m.id !== final.id).concat(final));
  return "ok" in r ? { ok: true, id: final.id } : r;
}

/** Removes the record of the user; for a catalogue molecule that had a copy the catalogue one is back. */
export async function remove(/** @type {string} */ id) {
  await ready();
  return write(mine.filter(m => m.id !== id));
}

/**
 * Imports a CSV or JSON file as records of the level «imported» (a CSV also as a collection named like the file; the short suspect columns name, formula, mass, comment are read). A molecule whose id the user already owns (own level) is kept as it is and counted in `kept`;
 * a wrong row refuses the whole file with one error per row.
 * @returns {Promise<{ errors: RowError[] } | { n: number, replaced: number, kept: number }>}
 */
export async function importFile(/** @type {string} */ name, /** @type {string} */ content) {
  await ready();
  let doc;
  if (/\.json$/i.test(name)) {
    let j;
    try { j = JSON.parse(content); } catch (_) { return { errors: [{ line: 0, key: "error.molecules.badJson", text: "json" }] }; }
    for (const r of (j && Array.isArray(j.molecules) ? j.molecules : [])) if (r && typeof r === "object") r.origin = { type: "imported", file: name };
    doc = await json("to_json", j);
  } else doc = await json("from_csv", { csv: content, file: name, level: "imported", collection: name.replace(/\.[^.]*$/, "") });
  if (hasErrors(doc)) return { errors: doc.errors };
  const added = /** @type {Molecule[]} */ (doc.molecules);
  const keptIds = new Set(added.filter(m => mine.some(x => x.id === m.id && x.origin.type === "user")).map(m => m.id));
  const fresh = added.filter(m => !keptIds.has(m.id));
  const ids = new Set(fresh.map(m => m.id));
  const replaced = mine.filter(m => ids.has(m.id)).length;
  const r = await write(mine.filter(m => !ids.has(m.id)).concat(fresh));
  return "ok" in r ? { n: fresh.length, replaced, kept: keptIds.size } : r;
}

/** The records of the user as a file: `{ name, text, type }` (CSV or canonical JSON). */
export async function exportAs(/** @type {"csv" | "json"} */ format) {
  await ready();
  const doc = await text("to_json", { schema: SCHEMA, molecules: mine });
  return format === "csv"
    ? { name: "mzlab-molecules.csv", text: await text("to_csv", doc), type: "text/csv;charset=utf-8" }
    : { name: "mzlab-molecules.json", text: doc, type: "application/json;charset=utf-8" };
}
