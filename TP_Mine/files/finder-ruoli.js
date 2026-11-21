// @ts-check
// @ts-strict
// TPMINE-PRIVATE  Roles and times of the files of mzFinder (samples/1), module imported by 34_pipeline_run.js. Packs and shows: the proposals (from the names), the warnings and the indices
// per role are the Rust of mzlab-core (`samples`: ops propose, validate, per_tp, canonical, digest), reached through `call`: the binding of the table .wasm (synchronous) or, in the desktop app, the native route finder/samples (asynchronous); every function that calls it is async and awaits the answer. The record lives in the notebook (`NB.samples`,
// canonical samples/1); where it has no entry for a file, the Rust proposes one from the name.
// A time proposed from a name stays unconfirmed (`time_confirmed` false) until the user confirms it.

export const SCHEMA = "samples/1";
/** the roles of samples/1, in the order of the select */
export const ROLES = ["sample", "t0", "blank", "dark", "infusion", "std", "calibration", "qc"];
/** the roles whose file has a treatment time */
const TIMED = ["sample", "t0", "dark"];
export const UNITS = ["min", "s", "h", "d"];

/** @typedef {{file: string, sha256: string, kind: string, blank_kind: string | null, time: number | null, time_unit: string, time_min: number | null, time_confirmed: boolean, series: string, replicate: number, concentration: number | null, concentration_unit: string | null, injection_order: number, istd: boolean, note: string, compound: string}} Sample */
/** @typedef {{code: string, severity: string, file: string, series: string}} Warning */
/** @typedef {(op: string, json: string) => string | Promise<string>} Call */

/** name of a file without the extension of the raw or index file */
export const stemOf = (/** @type {string} */ n) => String(n || "").replace(/(\.(mzidx|mzml|raw|d|wiff))+$/i, "");
/** the name a samples/1 entry has for an index file */
export const fileOf = (/** @type {string} */ n) => stemOf(n) + ".mzML";
const doc = (/** @type {Sample[]} */ cs) => JSON.stringify({ schema: SCHEMA, samples: cs });

/**
 * the model of the files `names` (.mzidx names of OPFS), in their order
 * @param {Call} call @param {string[]} names @param {{samples?: any} | null} nb
 * @returns {Promise<Sample[]>}
 */
export async function build(call, names, nb) {
  const files = names.map(fileOf), stored = new Map(((nb && nb.samples && nb.samples.samples) || []).map((/** @type {Sample} */ c) => [c.file, c]));
  /** @type {Map<string, Sample>} */ const made = new Map();
  const todo = files.filter(f => !stored.has(f));
  if (todo.length) for (const c of /** @type {Sample[]} */ (JSON.parse(await call("propose", JSON.stringify({ files: todo.map(name => ({ name, sha256: "", kind: "" })) }))).samples)) made.set(c.file, c);
  return files.map((f, i) => ({ ...(stored.get(f) || made.get(f)), injection_order: i + 1 }));
}

/** warnings of the Rust (`validate`) for the files: codes only, the words are the page's */
export const check = async (/** @type {Call} */ call, /** @type {Sample[]} */ cs) => /** @type {{warnings: Warning[], blocking: boolean}} */ (JSON.parse(await call("validate", doc(cs))));
/** the indices per role and `minutes[]` (feature_pipeline/3) of the Rust (`per_tp`) */
export const forTp = async (/** @type {Call} */ call, /** @type {Sample[]} */ cs) => /** @type {{treated: number[], t0: number[], blank: number[], dark: number[], infusion: number[], minutes: (number | null)[]}} */ (JSON.parse(await call("per_tp", doc(cs))));
/** canonical samples/1 of the files and its SHA-256, both of the Rust (what the operation `samples` of the .mzflow records) */
export const canonical = async (/** @type {Call} */ call, /** @type {Sample[]} */ cs) => ({ json: await call("canonical", doc(cs)), sha256: await call("digest", doc(cs)) });

/** sets the role of file `i`: a role without time forgets it, a t0 is at 0 minutes (confirmed) */
export function setRole(/** @type {Sample[]} */ cs, /** @type {number} */ i, /** @type {string} */ role) {
  const c = cs[i]; c.kind = role;
  if (role === "t0") { c.time = 0; c.time_confirmed = true; } else if (!TIMED.includes(role)) { c.time = null; c.time_confirmed = false; }
  if (role !== "blank") c.blank_kind = null;
}
/** sets the time of file `i` as written, in `unit` (the user wrote it: confirmed); null or NaN = no time */
export function setTime(/** @type {Sample[]} */ cs, /** @type {number} */ i, /** @type {number | null} */ t, /** @type {string} */ unit) {
  const c = cs[i], ok = t != null && Number.isFinite(t) && t >= 0;
  c.time = ok ? t : null; c.time_unit = unit; c.time_confirmed = ok;
}
/** the proposed times of the files that take one become confirmed (the button «Confirm times») */
export function confirmAll(/** @type {Sample[]} */ cs) { for (const c of cs) if (c.time != null && TIMED.includes(c.kind)) c.time_confirmed = true; }
/** the unit of the series: the times as written are read in `unit` (the Rust derives the minutes) */
export function setUnit(/** @type {Sample[]} */ cs, /** @type {string} */ unit) { for (const c of cs) c.time_unit = unit; }

/** keeps the model in the notebook (the entries of other files already there stay); the caller saves the notebook */
export function store(/** @type {{samples?: any}} */ nb, /** @type {Sample[]} */ cs) {
  const mine = new Set(cs.map(c => c.file)), old = ((nb.samples && nb.samples.samples) || []).filter((/** @type {Sample} */ c) => !mine.has(c.file));
  nb.samples = { schema: SCHEMA, samples: [...old, ...cs] };
}
