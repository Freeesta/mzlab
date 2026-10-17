// @ts-check
"use strict";
// The `campioni/2` document of the open files (roles, subtypes, times and their confirmation): this file packs E.files into the entries of the notebook
// (NB.session) and of the .mzflow, and reads them back; the Rust of mzlab-core (`api/campioni/<op>`) canonicalises, hashes and migrates the old shapes.
// The only writer of the fields is CAMPIONI.apply (campioni.js); an entry of E.files keeps `role` (campioni/2 `tipo`), `tconf`
// (the time is confirmed: undefined = yes, it was written by the user) and `cx` (the fields the table does not edit: istd, note, composto, serie).
const CDOC = /** @type {any} */ ((() => {
  const BASE = /** @type {Record<string, string>} */ ({ sample: "sample", blank: "blank", dark: "sample", infusion: "sample", std: "standard", calibration: "standard", qc: "sample" });
  // QA/QC vocabulary -> campioni/2
  const QROLE = /** @type {Record<string, string>} */ ({ sample: "sample", blank: "blank", qc: "qc", standard: "std", dark: "dark" });
  // the roles the program writes; a document of the old shape may also say `t0` (a sample at 0) or a `tipo_bianco` (ignored)
  const KINDS = ["sample", "blank", "dark", "infusion", "std", "calibration", "qc"];
  const known = (/** @type {string} */ k) => k === "t0" || KINDS.includes(k);
  const TIMED = new Set(["sample", "dark"]);
  const raw = (/** @type {string} */ op, /** @type {any} */ body) => fetch("api/campioni/" + op, { method: "POST", body: JSON.stringify(body) }).then(r => r.text());
  const post = (/** @type {string} */ op, /** @type {any} */ body) => raw(op, body).then(t => JSON.parse(t));

  /** the campioni/2 role of a file: the one set, else the one the name suggests (QC, dark), else the base type */
  function kindOf(/** @type {any} */ f) {
    if (f.role) return f.role === "calibration" ? "std" : f.role === "t0" ? "sample" : f.role;
    const q = window.QAQC ? QAQC.guessRole(f) : f.type === "blank" ? "blank" : f.type === "standard" ? "standard" : "sample";
    return QROLE[q] || "sample";
  }
  /** the entry of a file; `rep` (the replicate as typed) is the only key outside the schema, the Rust ignores it */
  function entry(/** @type {any} */ f, /** @type {number} */ i) {
    const k = kindOf(f), x = f.cx || {};
    const t = TIMED.has(k) && f.time != null ? (k === "dark" ? darkTime(+f.time) : +f.time) : null;
    return { composto: x.composto || "", concentrazione: f.conc ?? null, file: f.file, istd: !!x.istd, note: x.note || "", ordine_iniezione: i + 1, replica: 1, serie: x.serie || "", sha256: f.sha256 || "",
      tempo: t, tempo_confermato: t != null && f.tconf !== false, tempo_min: t, tipo: k, tipo_bianco: null,
      unita: f.conc != null ? f.cunit || "mg/L" : null, unita_tempo: "min", rep: f.rep || undefined };
  }
  /** the replicates counted as the Rust does: among the files with the same series, role (a sample at 0 min is a t0), minutes and concentration */
  function number(/** @type {any[]} */ es) {
    const role = (/** @type {any} */ e) => (e.tipo === "sample" && e.tempo_min === 0 ? "t0" : e.tipo);
    es.forEach((e, i) => { e.replica = 1 + es.slice(0, i).filter(o => o.serie === e.serie && role(o) === role(e) && o.tempo_min === e.tempo_min && o.concentrazione === e.concentrazione).length; });
    return es;
  }
  /** the entries (NB.session) of the files */
  const session = (/** @type {any[]} */ files) => number(files.map(entry));
  /** the document sent to the Rust (the keys outside the schema are left out) */
  const doc = (/** @type {any[]} */ files) => ({ schema: "campioni/2", campioni: session(files).map(({ rep, ...e }) => e) });
  /** the fields of E.files an entry gives */
  function fields(/** @type {any} */ e) {
    // old documents: `t0` = a sample at 0 minutes, `tipo_bianco` is dropped
    const k = e.tipo === "calibration" ? "std" : e.tipo === "t0" ? "sample" : e.tipo;
    const tm = e.tipo === "t0" ? 0 : TIMED.has(k) ? e.tempo_min ?? null : null;
    return { role: k, type: BASE[k] || "sample", time: k === "dark" ? darkTime(tm) : tm, tconf: tm == null || e.tipo === "t0" || e.tempo_confermato !== false,
      conc: e.concentrazione ?? null, cunit: e.unita ?? null, rep: e.rep || undefined, cx: { istd: e.istd, note: e.note, composto: e.composto, serie: e.serie } };
  }
  /** a notebook of the old shape (v1: `type`, `time`, no `tipo`) -> entries of campioni/2, with the QC and dark roles of qqq.qaqc */
  async function migrate(/** @type {any[]} */ old, /** @type {Record<string, string>} */ roles) {
    const r = await post("da_legacy", { session: old, qaqc: { roles } });
    if (!r || !Array.isArray(r.campioni)) return old;
    r.campioni.forEach((/** @type {any} */ e) => { const o = old.find(x => x && x.file === e.file); if (o && o.rep) e.rep = o.rep; });
    return r.campioni;
  }
  const isOld = (/** @type {any} */ s) => Array.isArray(s) && s.some(x => x && !x.tipo);
  /** the canonical JSON and its SHA-256 (the Rust of mzlab-core) */
  async function canonical(/** @type {any} */ d) { return { json: await raw("canonico", d), sha256: (await raw("digest", d)).trim() }; }
  return { KINDS, known, BASE, QROLE, TIMED, post, kindOf, entry, number, session, doc, fields, migrate, isOld, canonical };
})());
window.CDOC = CDOC;
