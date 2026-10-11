// TPMINE-PRIVATE  Untargeted export: the annotated table (xlsx with xlsx.js, CSV with «;»), the MS2 of the features (MGF) and the licences of the library records cited. The key of a row is (polarity, id of the feature)
// and the SHA-256 of the file of the table it comes from, so that a later step (SIRIUS, the chemist's curation) can join the rows without ambiguity. Pure functions (node tests them) + the buttons of the window.
(() => {
  "use strict";
  const G = typeof window !== "undefined" ? window : globalThis;
  const HEAD = ["key", "polarity", "feature_id", "mz", "rt_min", "level", "level_proposed", "level_decided", "best_candidate", "library", "formula", "inchikey", "smiles", "entropy", "cosine", "shared_peaks", "delta_ppm",
    "ms2_peaks", "ms2_scans", "library_record", "library_authors", "library_license", "evidence", "external_evidence", "table_file", "table_sha256"];
  const NOTE = "Candidates with a level and their evidence, never identifications. Levels: Schymanski et al. 2014. The level 2b is only ever the chemist's decision.";
  const pol = f => (f.pol > 0 ? "POS" : f.pol < 0 ? "NEG" : "");
  const r4 = v => (v == null || !Number.isFinite(v) ? "" : +v.toFixed(4)), r2 = v => (v == null || !Number.isFinite(v) ? "" : +v.toFixed(2));

  /** one row (array, same order as HEAD) per feature, then the area of every sample */
  function rowOf(table, f, r) {
    const L = G.LIVELLI, lv = L.shown(r.lv), lic = r.license || (r.accession || r.name ? "not stated" : "");
    return [f.key, pol(f), f.id, r4(f.mz), r2(f.rt), lv, r.level, r.decided || "", r.name || "", r.lib || "", r.formula || "", r.ik || "", r.smiles || "", r4(r.ent), r4(r.cos), r.shared == null ? "" : r.shared, r2(r.dppm),
      f.ms2 ? f.ms2.mz.length : "", f.nScans || "", r.accession || "", r.authors || "", lic, (r.evidence || []).join(" | "), (r.external || []).join(" | "), table.file.name || "", table.file.sha256 || ""]
      .concat(table.samples.map(s => (f.areas[s] != null ? f.areas[s] : "")));
  }
  const head = table => HEAD.concat(table.samples);
  const rows = (table, res, feats) => feats.map(f => rowOf(table, f, res.get(f.key)));

  /** CSV with «;», UTF-8 with BOM (Excel); text fields quoted when they hold «;», quotes or new lines; numbers with the decimal point */
  function csv(table, res, feats) {
    const q = v => { const s = v == null ? "" : String(v); return /[;"\r\n]/.test(s) ? '"' + s.replace(/"/g, '""') + '"' : s; };
    return "﻿" + [head(table)].concat(rows(table, res, feats)).map(r => r.map(q).join(";")).join("\r\n") + "\r\n";
  }

  /** the library records cited by the best candidates of these features, with authors and licence (the licence text as the record states it) */
  function sources(res, feats) {
    const m = new Map();
    for (const f of feats) {
      const r = res.get(f.key); if (!r || !r.name || !r.lib) continue;
      const k = r.lib + "|" + (r.accession || r.name), o = m.get(k) || { library: r.lib, accession: r.accession || "", name: r.name, authors: r.authors || "", license: r.license || "not stated", features: 0 };
      o.features++; m.set(k, o);
    }
    return [...m.values()].sort((a, b) => a.library.localeCompare(b.library) || a.name.localeCompare(b.name));
  }
  const licenceSummary = srcs => { const c = new Map(); for (const s of srcs) c.set(s.license, (c.get(s.license) || 0) + 1); return [...c].map(([l, n]) => `${l}: ${n} record${n === 1 ? "" : "s"}`).join("; "); };

  /** MGF of the MS2 of the features that have one; comment lines (#) list the licences of the cited records */
  function mgf(table, res, feats) {
    const srcs = sources(res, feats), out = [`# mzFinder untargeted export · table ${table.file.name || ""} · SHA-256 ${table.file.sha256 || "n/a"}`, `# ${NOTE}`];
    out.push("# Library records cited (" + srcs.length + ")" + (srcs.length ? ": " + licenceSummary(srcs) : ""));
    for (const s of srcs) out.push(`# ${s.library} · ${s.accession || s.name} · ${s.authors || "authors not stated"} · ${s.license}`);
    for (const f of feats) {
      if (!f.ms2 || !f.ms2.mz.length) continue; const r = res.get(f.key), lv = G.LIVELLI.shown(r.lv);
      out.push("BEGIN IONS", `FEATURE_ID=${f.id}`, `PEPMASS=${(f.ms2.prec > 0 ? f.ms2.prec : f.mz).toFixed(4)}`, `CHARGE=${f.pol < 0 ? "1-" : "1+"}`, "MSLEVEL=2", `RTINSECONDS=${(f.rt * 60).toFixed(1)}`, `IONMODE=${f.pol < 0 ? "Negative" : "Positive"}`, `LEVEL=${lv}`);
      if (r.name && ["1", "2a", "2b", "3"].includes(lv)) { out.push(`NAME=${String(r.name).replace(/[\r\n]+/g, " ")}`); if (r.accession) out.push(`LIBRARY_RECORD=${String(r.accession).replace(/[\r\n]+/g, " ")}`); out.push(`LIBRARY_LICENSE=${String(r.license || "not stated").replace(/[\r\n]+/g, " ")}`); }
      for (let i = 0; i < f.ms2.mz.length; i++) out.push(`${f.ms2.mz[i]} ${f.ms2.it[i]}`);
      out.push("END IONS");
    }
    return out.join("\n") + "\n";
  }

  /** sheets for xlsx.js: features, evidence (one line each), cited records with their licences, parameters */
  function sheets(table, res, feats, o = {}) {
    const srcs = sources(res, feats), ev = [];
    for (const f of feats) { const r = res.get(f.key); for (const x of r.evidence || []) ev.push([f.key, G.LIVELLI.shown(r.lv), "level", x]); for (const x of r.external || []) ev.push([f.key, G.LIVELLI.shown(r.lv), "external (never a level)", x]); }
    const T = G.LIVELLI.thresholds(), prof = G.LIVELLI.profileName();
    return [
      { name: "Features", head: head(table), rows: rows(table, res, feats) },
      { name: "Evidence", head: ["key", "level", "kind", "text"], rows: ev, widths: [14, 8, 24, 100] },
      { name: "Library records", head: ["library", "record", "name", "authors", "license", "features"], rows: srcs.map(s => [s.library, s.accession, s.name, s.authors, s.license, s.features]), widths: [28, 18, 30, 30, 22, 10] },
      { name: "Parameters", head: ["parameter", "value"], rows: [["table file", table.file.name || ""], ["table SHA-256", table.file.sha256 || ""], ["program of the table", table.program], ["polarity", table.pol > 0 ? "positive" : table.pol < 0 ? "negative" : ""],
        ["files read", table.used ? table.used.join(", ") : ""], ["precursor tolerance (ppm)", o.tol ?? ""], ["profile of the levels", prof], ["thresholds", JSON.stringify(T)], ["features exported", feats.length],
        ["licences of the records cited", licenceSummary(srcs)], ["note", NOTE]], widths: [30, 90] },
    ];
  }

  const stamp = () => new Date().toISOString().slice(0, 10).replace(/-/g, "");
  const base = table => "untargeted_" + String(table.file.name || "table").replace(/\.[^.]+$/, "").replace(/[^\w.-]+/g, "_").slice(0, 60) + "_" + stamp();

  // ------------------------------------------------------------ the buttons of the window (27_untarget_ui.js asks for them and gives the state)
  const buttons = () => `<label class="mut"><input type="checkbox" id="unt-xf"> only the filtered features</label> <button type="button" id="unt-x-xlsx">Excel (.xlsx)</button><button type="button" id="unt-x-csv" title="Text with ';' as separator, UTF-8">CSV (;)</button><button type="button" id="unt-x-mgf" title="MS2 of the features with their level and the licences of the library records">MGF</button>` + (G.UNTF ? G.UNTF.buttons() : "");
  function bind(dlg, state) {
    const pick = () => { const s = state(), only = dlg.querySelector("#unt-xf").checked; return { s, feats: only ? s.visible() : s.all() }; };
    dlg.querySelector("#unt-x-xlsx").onclick = () => { const { s, feats } = pick(); G.dlx(base(s.table) + ".xlsx", sheets(s.table, s.res, feats, { tol: s.tol })); };
    dlg.querySelector("#unt-x-csv").onclick = () => { const { s, feats } = pick(); G.dl(base(s.table) + ".csv", csv(s.table, s.res, feats), "text/csv;charset=utf-8"); };
    dlg.querySelector("#unt-x-mgf").onclick = () => { const { s, feats } = pick(); G.dl(base(s.table) + ".mgf", mgf(s.table, s.res, feats), "text/plain;charset=utf-8"); };
    if (G.UNTF && dlg.querySelector("#unt-x-gnps")) G.UNTF.bind(dlg, pick);
  }

  G.UNTX = { HEAD, head, rows, csv, mgf, sheets, sources, licenceSummary, buttons, bind, base };
  if (typeof module !== "undefined") module.exports = G.UNTX;
})();
