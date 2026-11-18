// @ts-check
// @ts-strict
"use strict";
// The `samples/1` document of the open files (roles, subtypes, times and their confirmation): this file packs E.files into the entries of the notebook
// (NB.session) and of the .mzflow, and reads them back; the Rust of mzlab-core (`api/samples/<op>`) canonicalises and hashes.
// The only writer of the fields is CAMPIONI.apply (campioni.js); an entry of E.files keeps `role` (samples/1 `kind`), `tconf`
// (the time is confirmed: undefined = yes, it was written by the user) and `cx` (the fields the table does not edit: istd, note, compound, series).
const CDOC = /** @type {any} */ ((() => {
  const BASE = /** @type {Record<string, string>} */ ({ sample: "sample", blank: "blank", dark: "sample", infusion: "sample", std: "standard", calibration: "standard", qc: "sample" });
  // QA/QC vocabulary -> samples/1
  const QROLE = /** @type {Record<string, string>} */ ({ sample: "sample", blank: "blank", qc: "qc", standard: "std", dark: "dark" });
  // the roles the program writes; a document of the old shape may also say `t0` (a sample at 0) or a `blank_kind` (ignored)
  const KINDS = ["sample", "blank", "dark", "infusion", "std", "calibration", "qc"];
  const known = (/** @type {string} */ k) => k === "t0" || KINDS.includes(k);
  const TIMED = new Set(["sample", "dark"]);
  const raw = (/** @type {string} */ op, /** @type {any} */ body) => fetch("api/samples/" + op, { method: "POST", body: JSON.stringify(body) }).then(r => r.text());
  const post = (/** @type {string} */ op, /** @type {any} */ body) => raw(op, body).then(t => JSON.parse(t));

  /** the samples/1 role of a file: the one set, else the one the name suggests (QC, dark), else the base type */
  function kindOf(/** @type {any} */ f) {
    if (f.role) return f.role === "calibration" ? "std" : f.role === "t0" ? "sample" : f.role;
    const q = window.QAQC ? QAQC.guessRole(f) : f.type === "blank" ? "blank" : f.type === "standard" ? "standard" : "sample";
    return QROLE[q] || "sample";
  }
  /** the entry of a file; `rep` (the replicate as typed) is the only key outside the schema, the Rust ignores it */
  function entry(/** @type {any} */ f, /** @type {number} */ i) {
    const k = kindOf(f), x = f.cx || {};
    const t = TIMED.has(k) && f.time != null ? (k === "dark" ? darkTime(+f.time) : +f.time) : null;
    return { compound: x.compound || "", concentration: f.conc ?? null, file: f.file, istd: !!x.istd, note: x.note || "", injection_order: i + 1, replicate: 1, series: x.series || "", sha256: f.sha256 || "",
      time: t, time_confirmed: t != null && f.tconf !== false, time_min: t, kind: k, blank_kind: null,
      concentration_unit: f.conc != null ? f.cunit || "mg/L" : null, time_unit: "min", rep: f.rep || undefined };
  }
  /** the replicates counted as the Rust does: among the files with the same series, role (a sample at 0 min is a t0), minutes and concentration */
  function number(/** @type {any[]} */ es) {
    const role = (/** @type {any} */ e) => (e.kind === "sample" && e.time_min === 0 ? "t0" : e.kind);
    es.forEach((e, i) => { e.replicate = 1 + es.slice(0, i).filter(o => o.series === e.series && role(o) === role(e) && o.time_min === e.time_min && o.concentration === e.concentration).length; });
    return es;
  }
  /** the entries (NB.session) of the files */
  const session = (/** @type {any[]} */ files) => number(files.map(entry));
  /** the document sent to the Rust (the keys outside the schema are left out) */
  const doc = (/** @type {any[]} */ files) => ({ schema: "samples/1", samples: session(files).map(({ rep, ...e }) => e) });
  /** the fields of E.files an entry gives */
  function fields(/** @type {any} */ e) {
    // old documents: `t0` = a sample at 0 minutes, `blank_kind` is dropped
    const k = e.kind === "calibration" ? "std" : e.kind === "t0" ? "sample" : e.kind;
    const tm = e.kind === "t0" ? 0 : TIMED.has(k) ? e.time_min ?? null : null;
    return { role: k, type: BASE[k] || "sample", time: k === "dark" ? darkTime(tm) : tm, tconf: tm == null || e.kind === "t0" || e.time_confirmed !== false,
      conc: e.concentration ?? null, cunit: e.concentration_unit ?? null, rep: e.rep || undefined, cx: { istd: e.istd, note: e.note, compound: e.compound, series: e.series } };
  }
  /** the canonical JSON and its SHA-256 (the Rust of mzlab-core) */
  async function canonical(/** @type {any} */ d) { return { json: await raw("canonical", d), sha256: (await raw("digest", d)).trim() }; }
  return { KINDS, known, BASE, QROLE, TIMED, post, kindOf, entry, number, session, doc, fields, canonical };
})());
window.CDOC = CDOC;
