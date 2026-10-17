// @ts-strict
"use strict";
// Sample metadata («Statistica» panel, first section): one row per file with the fields of the mzmine metadata table
//   filename, sample_type (sample | blank | qc | standard | dark | infusion), time + time_unit, concentration + concentration_unit, replicate (a dark has a negative time: the minutes before the time zero).
// The values are the ones the file name gave (project.py, read at loading) and can be edited here; the table is the only place that
// writes them after loading (`apply`, the only writer): f.role / f.type / f.time (minutes) / f.tconf / f.conc / f.cunit / f.rep of E.files,
// the server copy (/api/meta), the notebook (uiSave: NB.session in the format campioni/2, campioni-doc.js) and the .mzflow file (operation
// `campioni`). Export and import of the table as TSV (mzmine column names, the columns of campioni/2 at the end).
// The pure functions at the top (TSV, replicate groups, time scale) are also run by node in tests/test_campioni.py.
const CAMPIONI = (() => {
  // sample_type of the table (an old export may say «t0»: a sample at 0 minutes)
  const TYPES = ["sample", "blank", "qc", "standard", "dark", "infusion"];
  // sample_type -> role of campioni/2 (the others are the same word)
  const KIND = { standard: "std", calibration: "std" };
  // role of campioni/2 -> sample_type
  const WORD = { std: "standard", calibration: "standard" };
  // an old export has a «blank_type» column too: ignored
  const COLS = ["filename", "sample_type", "time", "time_unit", "concentration", "concentration_unit", "replicate", "time_confirmed"];
  const C_UNITS = ["mg/L", "µg/L", "ng/L", "g/L"];

  // ---------------------------------------------------------------- pure functions
  // text of the table, tab separated; `rows`: [{filename, type, time (in `tunit`), conc, cunit, rep}]
  function toTsv(rows, tunit = "min") {
    const clean = v => String(v == null ? "" : v).replace(/[\t\r\n]+/g, " ");
    const out = [COLS.join("\t")];
    rows.forEach(r => out.push([r.filename, r.type, r.time ?? "", r.time == null ? "" : tunit, r.conc ?? "", r.conc == null ? "" : (r.cunit || ""), r.rep || "", r.time == null || r.tconf == null ? "" : r.tconf ? "yes" : "no"].map(clean).join("\t")));
    return out.join("\n") + "\n";
  }
  // TSV (or CSV with ; or ,) -> {rows: [{filename, type, time (minutes), conc, cunit, rep}], warnings: [text]}
  function parseTsv(text) {
    // «# mzLab ...» = provenance line of our own exports
    const lines = String(text || "").replace(/^﻿/, "").split(/\r?\n/).filter(l => l.trim() !== "" && !l.startsWith("#"));
    const warn = [];
    if (!lines.length) return { rows: [], warnings: ["empty"] };
    const sep = lines[0].includes("\t") ? "\t" : lines[0].includes(";") ? ";" : ",";
    const head = lines[0].split(sep).map(h => h.trim().toLowerCase().replace(/[\s-]+/g, "_"));
    const col = n => head.indexOf(n);
    if (col("filename") < 0) return { rows: [], warnings: ["filename"] };
    const num = (s, signed = false) => { const v = parseDecimal(s, signed); return v == null || Number.isNaN(v) ? null : v; };
    const rows = [];
    lines.slice(1).forEach((ln, i) => {
      const c = ln.split(sep).map(s => s.trim()), g = n => (col(n) < 0 ? "" : c[col(n)] || "");
      if (!g("filename")) return;
      const ty = g("sample_type").toLowerCase();
      let type = null;
      if (ty === "t0") type = "sample";
      else if (ty) { if (TYPES.includes(ty)) type = ty; else warn.push(`${g("filename")}: ${ty}`); }
      const tu = g("time_unit").toLowerCase() || "min";
      let time = num(g("time"), type === "dark");
      if (ty === "t0" && time == null) time = 0;
      if (time != null) { if (TIME_UNITS[tu]) time = timeStored(time, tu); else { warn.push(`${g("filename")}: ${tu}`); time = null; } }
      const cu = C_UNITS.find(u => u.toLowerCase() === g("concentration_unit").toLowerCase()) || (g("concentration_unit") ? null : undefined);
      const tc = g("time_confirmed").toLowerCase();
      rows.push({ filename: g("filename"), type, time, conc: num(g("concentration")), cunit: cu === undefined ? null : cu, rep: g("replicate") || null, tconf: ["yes", "no"].includes(tc) ? tc === "yes" : null, line: i + 2 });
    });
    return { rows, warnings: warn };
  }
  // name of a file without folder and extension, lower case: how the table and the files are matched
  const baseName = n => (String(n || "").split(/[\\/]/).pop() || "").replace(/#.*$/, "").replace(/\.(mzml|mzxml|raw|wiff|d|dam)$/i, "").toLowerCase();
  // replicate group of a file: the one typed, else files of the same type, time and concentration (a file with neither is alone)
  function repKey(f, role) {
    if (f.rep) return String(f.rep);
    if (f.time == null && f.conc == null) return "#" + (f.file || f.label || "");
    return `${role}|${f.time ?? ""}|${f.conc ?? ""}${f.conc != null ? "|" + (f.cunit || "") : ""}`;
  }
  // [{key, files: [f...]}] in the order of the first appearance of each group
  function groups(list, roleOf) {
    const m = new Map();
    list.forEach(f => { const k = repKey(f, roleOf(f)); if (!m.has(k)) m.set(k, []); m.get(k).push(f); });
    return [...m].map(([key, files]) => ({ key, files }));
  }
  // position 0..1 of a time in the scale of a list of times (null when there is no scale)
  function timeFrac(t, times) {
    const v = times.filter(x => x != null);
    if (t == null || !v.length) return null;
    const lo = Math.min(...v), hi = Math.max(...v);
    return hi > lo ? (t - lo) / (hi - lo) : 0.5;
  }

  // ---------------------------------------------------------------- the files of the table
  const t = (k, p) => I18N.t(k, p);
  const eh = s => EH(s == null ? "" : s);
  const files = () => (typeof E !== "undefined" && E.files ? E.files : []).filter(f => !f.gone && (f.lv || 1) === 1 && f.kind !== "empty");
  // role of the QA/QC (sample | blank | qc | standard | dark), for the statistics
  const roleOf = f => (window.QAQC ? QAQC.roleOf(f) : f.type || "sample");
  const kindOf = f => CDOC.kindOf(f);                                              // role of campioni/2
  const roleName = r => t("qaqc.role." + r);
  const repOf = f => repKey(f, roleOf(f));
  const shownRep = f => (f.rep ? f.rep : f.time == null && f.conc == null ? "" : repKey(f, roleOf(f)).replace(/\|/g, " "));
  /** value of the role select of a file: sample, blank, dark, infusion, std, qc */
  const selVal = f => kindOf(f);

  function setKind(f, k) {
    const was = kindOf(f);
    // the old role: a sample at 0 minutes
    if (k === "t0") { k = "sample"; if (!(f.time === 0)) { f.time = 0; f.tconf = true; } }
    f.role = k; f.type = CDOC.BASE[k] || "sample";
    if (was !== k && !CDOC.TIMED.has(k)) { f.time = null; f.tconf = true; }
    // a negative time belongs to the dark only
    else if (was === "dark" && k !== "dark" && f.time != null && f.time < 0) { f.time = null; f.tconf = true; }
  }
  // apply the edits {file, role? (or type: the sample_type of the table), time? (minutes), tconf?, conc?, cunit?, rep?} to the open files and tell everyone
  // (`quiet`: the views are not redrawn, for the loading of files). The only writer of roles, subtypes, times and their confirmation.
  function apply(entries, quiet = false) {
    const srv = [];
    entries.forEach(e => {
      const f = files().find(x => x.file === e.file || x.file.split("#")[0] === e.file); if (!f) return;
      const k = KIND[e.role || e.type] || e.role || e.type;
      if (k && CDOC.known(k)) setKind(f, k);
      if ("time" in e) { f.time = e.time; f.tconf = e.tconf !== false; } else if ("tconf" in e) f.tconf = !!e.tconf;
      // minutes of darkness before the time zero: never positive
      if (kindOf(f) === "dark") f.time = darkTime(f.time);
      if ("conc" in e) { f.conc = e.conc; if (e.conc == null) f.cunit = null; }
      if ("cunit" in e) f.cunit = e.cunit;
      if ("rep" in e) f.rep = e.rep || undefined;
      srv.push({ file: f.file, type: f.type, role: kindOf(f), tconf: f.tconf !== false, time: f.time ?? null, conc: f.conc ?? null, cunit: f.cunit ?? null, rep: f.rep || null });
    });
    if (!srv.length) return 0;
    fetch("api/meta", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ files: srv }) }).catch(() => {});
    if (!quiet) {
      if (typeof paintFiles === "function") paintFiles();
      if (typeof renderFileList === "function") renderFileList();
      if (typeof redrawAll === "function") redrawAll();
    }
    if (typeof uiSave === "function") uiSave();
    document.dispatchEvent(new CustomEvent("campioni"));
    return srv.length;
  }
  function rows() {
    const u = tunitGet();
    return files().map(f => { const k = kindOf(f); return { filename: f.file, type: WORD[k] || k, time: timeShown(f.time, u), conc: f.conc ?? null, cunit: f.cunit || (f.conc != null ? "mg/L" : ""), rep: f.rep || "", tconf: f.time == null ? null : f.tconf !== false }; });
  }
  // table imported from TSV -> the open files; returns {changed, unknown: [filenames without a file], warnings}
  function importTsv(text) {
    const p = parseTsv(text);
    const byBase = new Map(files().map(f => [baseName(f.file), f]));
    const entries = [], unknown = [];
    p.rows.forEach(r => {
      const f = byBase.get(baseName(r.filename));
      if (!f) { unknown.push(r.filename); return; }
      const e = { file: f.file, time: r.time, conc: r.conc, rep: r.rep };
      if (r.type) e.role = r.type;
      if (r.time != null && r.tconf === false) e.tconf = false;
      if (r.conc != null) e.cunit = r.cunit || f.cunit || "mg/L";
      entries.push(e);
    });
    return { changed: apply(entries), unknown, warnings: p.warnings };
  }

  // ---------------------------------------------------------------- the legend of the time gradient (also used by the other sections)
  // colour bar from the first to the last time of the files that have one; "" when no file has a time
  function gradientLegend(fs) {
    // the dark (negative time) is not a point of the gradient
    const tm = (fs || files()).filter(f => f.time != null && f.type === "sample" && kindOf(f) !== "dark");
    if (tm.length < 2) return "";
    const lo = Math.min(...tm.map(f => f.time)), hi = Math.max(...tm.map(f => f.time));
    if (!(hi > lo)) return "";
    const stops = Array.from({ length: 6 }, (_, i) => `${typeof seqColor === "function" ? seqColor(i / 5) : "#888"} ${i * 20}%`).join(",");
    return `<span class="cmp-grad" role="img" aria-label="${eh(t("meta.grad.aria", { a: fmtTime(lo), b: fmtTime(hi) }))}"><span>${eh(fmtTime(lo))}</span><i style="background:linear-gradient(90deg,${stops})"></i><span>${eh(fmtTime(hi))}</span></span>`;
  }

  // ---------------------------------------------------------------- workflow file
  const exportState = () => files().map(f => ({ nome: f.file, ruolo: kindOf(f), concentrazione: f.conc ?? null, unita_conc: f.cunit ?? null, replica: f.rep || null }));
  function importState(list) {
    if (!Array.isArray(list)) return 0;
    const ent = [];
    list.forEach(x => { if (x && x.nome) { const e = { file: x.nome }; const r = KIND[x.ruolo] || x.ruolo; if (CDOC.known(r)) e.role = r; if ("concentrazione" in x) { e.conc = x.concentrazione; e.cunit = x.unita_conc || null; } if ("replica" in x) e.rep = x.replica || ""; ent.push(e); } });
    return apply(ent);
  }
  /** the document campioni/2 of the open files (entries of NB.session plus `rep`, the keys of the schema in the order of the Rust) */
  const docNow = () => CDOC.doc(files());
  /** the entries of a campioni/2 document (the operation `campioni` of a .mzflow, or NB.session) applied to the open files; returns how many */
  function importDoc(d) {
    const list = d && Array.isArray(d.campioni) ? d.campioni : [];
    return apply(list.filter(e => e && e.file && CDOC.known(e.tipo)).map(e => { const x = CDOC.fields(e); const o = { file: e.file, role: x.role, time: x.time, tconf: x.tconf, conc: x.conc, rep: x.rep || "" }; if (x.conc != null) o.cunit = x.cunit; return o; }), true);
  }
  /** the fields of the entries of NB.session set on the files just built (first loading): role, subtype, confirmation, the fields the table does not edit */
  function restore(list) {
    if (!Array.isArray(list)) return;
    for (const f of files()) {
      const e = list.find(x => x && x.file === f.file && x.tipo); if (!e) continue;
      const x = CDOC.fields(e); f.role = x.role; f.time = x.time; f.tconf = x.tconf; f.cx = x.cx; if (x.rep) f.rep = x.rep;
    }
  }
  /** what a file keeps when the list is rebuilt (a file added later) */
  const keep = old => (old ? { role: old.role, tconf: old.tconf, cx: old.cx } : {});

  return { TYPES, COLS, KIND, toTsv, parseTsv, baseName, repKey, groups, timeFrac, files, roleOf, kindOf, roleName, repOf, rows, apply, importTsv, gradientLegend, selVal, shownRep, exportState, importState, docNow, importDoc, restore, keep };
})();
if (typeof window !== "undefined") window.CAMPIONI = CAMPIONI;
if (typeof module !== "undefined") module.exports = CAMPIONI;
