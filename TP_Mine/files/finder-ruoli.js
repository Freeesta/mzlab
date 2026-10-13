// @ts-check
// TPMINE-PRIVATE  Roles and times of the files of mzFinder (campioni/2), module imported by 34_pipeline_run.js. Packs and shows: the proposals (from the names), the warnings and the indices
// per role are the Rust of mzlab-core (`campioni`: ops proponi, valida, per_tp, da_legacy, canonico, digest), reached through the binding of the table .wasm (`call`). The record lives in the notebook (`NB.campioni`,
// canonical campioni/2); where it has no entry for a file, the Data view's NB.session (v1) and the roles of the QA/QC panel (localStorage qqq.qaqc) are migrated, and the rest is proposed.
// A time proposed from a name stays unconfirmed (`tempo_confermato` false) until the user confirms it.

export const SCHEMA = "campioni/2";
/** the roles of campioni/2, in the order of the select */
export const ROLES = ["sample", "t0", "blank", "dark", "infusion", "std", "calibration", "qc"];
/** the roles whose file has a treatment time */
const TIMED = ["sample", "t0", "dark"];
export const UNITS = ["min", "s", "h", "d"];

/** @typedef {{file: string, sha256: string, tipo: string, tipo_bianco: string | null, tempo: number | null, unita_tempo: string, tempo_min: number | null, tempo_confermato: boolean, serie: string, replica: number, concentrazione: number | null, unita: string | null, ordine_iniezione: number, istd: boolean, note: string, composto: string}} Campione */
/** @typedef {{codice: string, gravita: string, file: string, serie: string}} Avviso */
/** @typedef {(op: string, json: string) => string} Call */

/** name of a file without the extension of the raw or index file */
export const stemOf = (/** @type {string} */ n) => String(n || "").replace(/\.(mzidx|mzml|raw|d|wiff)$/i, "");
/** the name a campioni/2 entry has for an index file */
export const fileOf = (/** @type {string} */ n) => stemOf(n) + ".mzML";
const doc = (/** @type {Campione[]} */ cs) => JSON.stringify({ schema: SCHEMA, campioni: cs });

/**
 * the model of the files `names` (.mzidx names of OPFS), in their order
 * @param {Call} call @param {string[]} names @param {{campioni?: any, session?: any} | null} nb @param {Record<string, string> | null} [qaqc] role by file name of the QA/QC panel
 * @returns {Campione[]}
 */
export function build(call, names, nb, qaqc) {
  const files = names.map(fileOf), stored = new Map(((nb && nb.campioni && nb.campioni.campioni) || []).map((/** @type {Campione} */ c) => [c.file, c]));
  const rest = files.filter(f => !stored.has(f)), want = new Set(rest.map(f => f.toLowerCase()));
  const session = ((nb && Array.isArray(nb.session) && nb.session) || []).filter((/** @type {any} */ s) => want.has(fileOf(s.file).toLowerCase())).map((/** @type {any} */ s) => ({ ...s, file: rest.find(f => f.toLowerCase() === fileOf(s.file).toLowerCase()) }));
  /** @type {Map<string, Campione>} */ const made = new Map();
  if (session.length) {
    const roles = {}; for (const s of session) for (const k of [s.file, ...Object.keys(qaqc || {}).filter(q => stemOf(q).toLowerCase() === stemOf(s.file).toLowerCase())]) if (qaqc && qaqc[k]) /** @type {any} */ (roles)[s.file] = qaqc[k];
    for (const c of /** @type {Campione[]} */ (JSON.parse(call("da_legacy", JSON.stringify({ session, qaqc: { roles } }))).campioni)) made.set(c.file, c);
  }
  const todo = rest.filter(f => !made.has(f));
  if (todo.length) for (const c of /** @type {Campione[]} */ (JSON.parse(call("proponi", JSON.stringify({ files: todo.map(name => ({ name, sha256: "", kind: "" })) }))).campioni)) made.set(c.file, c);
  return files.map((f, i) => ({ ...(stored.get(f) || made.get(f)), ordine_iniezione: i + 1 }));
}

/** warnings of the Rust (`valida`) for the files: codes only, the words are the page's */
export const check = (/** @type {Call} */ call, /** @type {Campione[]} */ cs) => /** @type {{avvisi: Avviso[], bloccante: boolean}} */ (JSON.parse(call("valida", doc(cs))));
/** the indices per role and `minuti[]` (feature_pipeline/1) of the Rust (`per_tp`) */
export const forTp = (/** @type {Call} */ call, /** @type {Campione[]} */ cs) => /** @type {{treated: number[], t0: number[], blank: number[], dark: number[], infusion: number[], minuti: (number | null)[]}} */ (JSON.parse(call("per_tp", doc(cs))));
/** canonical campioni/2 of the files and its SHA-256, both of the Rust (what the operation `campioni` of the .mzflow records) */
export const canonical = (/** @type {Call} */ call, /** @type {Campione[]} */ cs) => ({ json: call("canonico", doc(cs)), sha256: call("digest", doc(cs)) });

/** sets the role of file `i`: a role without time forgets it, a t0 is at 0 minutes (confirmed) */
export function setRole(/** @type {Campione[]} */ cs, /** @type {number} */ i, /** @type {string} */ role) {
  const c = cs[i]; c.tipo = role;
  if (role === "t0") { c.tempo = 0; c.tempo_confermato = true; } else if (!TIMED.includes(role)) { c.tempo = null; c.tempo_confermato = false; }
  if (role !== "blank") c.tipo_bianco = null;
}
/** sets the time of file `i` as written, in `unit` (the user wrote it: confirmed); null or NaN = no time */
export function setTime(/** @type {Campione[]} */ cs, /** @type {number} */ i, /** @type {number | null} */ t, /** @type {string} */ unit) {
  const c = cs[i], ok = t != null && Number.isFinite(t) && t >= 0;
  c.tempo = ok ? t : null; c.unita_tempo = unit; c.tempo_confermato = ok;
}
/** the proposed times of the files that take one become confirmed (the button «Confirm times») */
export function confirmAll(/** @type {Campione[]} */ cs) { for (const c of cs) if (c.tempo != null && TIMED.includes(c.tipo)) c.tempo_confermato = true; }
/** the unit of the series: the times as written are read in `unit` (the Rust derives the minutes) */
export function setUnit(/** @type {Campione[]} */ cs, /** @type {string} */ unit) { for (const c of cs) c.unita_tempo = unit; }

/** keeps the model in the notebook (the entries of other files already there stay); the caller saves the notebook */
export function store(/** @type {{campioni?: any}} */ nb, /** @type {Campione[]} */ cs) {
  const mine = new Set(cs.map(c => c.file)), old = ((nb.campioni && nb.campioni.campioni) || []).filter((/** @type {Campione} */ c) => !mine.has(c.file));
  nb.campioni = { schema: SCHEMA, campioni: [...old, ...cs] };
}
