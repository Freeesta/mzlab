// @ts-strict
"use strict";
// Study report (HTML to print + JSON), built from the workflow file of the session and the QA/QC measures; nothing leaves the computer.
// Sections in the order of the NORMAN guide (Hollender et al., Environ. Sci. Eur. 35:75, 2023) and the BP4NTA Study Reporting Tool:
//   1 study design (files, SHA-256, type of each file) · 2 acquisition (instrument and method, as the mzML says it)
//   3 processing (program version and commit, parameters, operations) · 4 QA/QC (measures of qaqc.js; only with FEATURES.qaqc) · 5 confidence (4 without QA/QC) levels (HR, Schymanski)
// Numbers and evidence only: the report never says what a compound is.
const RAPPORTO = (() => {
  const FORMAT = "mzreport/1";
  const t = (k, p) => I18N.t(k, p);
  const eh = s => EH(s == null ? "" : s);
  const isHr = () => !!(window.HR && typeof BANCO !== "undefined" && BANCO.on());
  // candidates annotated in the session (confidence level and evidence), kept in the workflow file
  const cands = new Map();

  // libreria.js calls this every time it assesses the candidates of one spectrum (and again after the chemist's decision)
  function note(lv, q) {
    if (!lv) return;
    const b = lv.best || lv.nearest || lv.analog || null;
    const key = (q && q.title || "") + "|" + (q && q.prec != null ? q.prec : "");
    cands.set(key, { spectrum: q && q.title || "", precursor: q && q.prec != null ? q.prec : null, candidate: b ? (b.name || "") : "", library: b ? (b.lib || "") : "",
      proposed: lv.proposed, decided: lv.decided || null, level: lv.decided || lv.proposed, profile: lv.profile || "", evidence: (lv.lines || []).slice(), reason: lv.decisive || "" });
  }
  const candidates = () => [...cands.values()];
  const setCandidates = a => { cands.clear(); (a || []).forEach(c => cands.set((c.spectrum || "") + "|" + (c.precursor != null ? c.precursor : ""), c)); };

  async function fileInfo(f) {
    try { const j = await J(`api/fileinfo?k=${f.k}`); return { name: f.file, instrument: j.instrument || "", serial: j.serial || "", components: j.components || [], analyzers: j.analyzers || [], software: j.software || [],
      conversion: j.conversion || [], start: j.start || "", scans: j.scans, levels: j.levels || {}, rt: j.rt || [], filtri: Object.keys(j.filters || {}).length ? j.filters : undefined, mz: j.mz_range || undefined }; }
    catch (e_) { const e = /** @type {Lanciato} */ (e_); return { name: f.file, error: String(e && e.message || e) }; }
  }
  const slim = r => ({ mz: r.mz, rt: r.rt, rt_window: r.win, tolerance: r.tol, unit: r.unit, median_area: r.median_area, rt_drift_per_file: r.rt_drift_per_file, area_drift_per_file: r.area_drift_per_file,
    cv_area: r.cv_area, cv_area_qc: r.cv_area_qc, mass_accuracy: r.mass_accuracy, rows: r.rows });

  // The JSON: from the workflow file (current session, or the one the caller passes) plus the measures
  const qaFeat = () => !!(window.FEATURES && FEATURES.qaqc);      // QA/QC dormant: no measures, no section 4
  async function build(wf) {
    wf = wf || WORKFLOW.buildWorkflow();
    if (window.QAQC && qaFeat()) { if (!wf.parameters.qaqc) wf.parameters.qaqc = QAQC.exportState(); }
    const files = (typeof E !== "undefined" ? E.files : []).filter(f => !f.gone);
    const roleOf = n => { const f = files.find(x => x.file === n); return f && window.QAQC ? QAQC.roleOf(f) : "sample"; };
    const hr = isHr();
    /** @type {any} */ const qa = { ions: [], sample_blank_ratio: null, feature_mzfinder: null };
    if (window.QAQC && qaFeat()) {
      const r = await QAQC.run();
      qa.ions = r.ions.filter(o => !o.error).map(o => ({ label: o.ion.label, function: o.ion.kind === "known" ? "known ion" : "internal standard", files: o.files, roles: o.roles, ...slim(o.r) }));
      if (QAQC.state.ratio.text) { try { const q = await QAQC.runRatio(); qa.sample_blank_ratio = { mz: q.mz, rt: q.rt, threshold: q.thr, mean_blank_area: q.r.ratio.blank_area, files: q.files, roles: q.roles, areas: q.r.rows.map(x => (x ? x.area : 0)), ratios: q.r.ratio.rows }; } catch (_) { /* ion not found */ } }
      if (hr) { try { const f = await QAQC.runFeatures(); if (f) qa.feature_mzfinder = { threshold: f.thr, blanks: f.blanks, samples: f.samples, rows: f.rows }; } catch (_) { /* no table */ } }
    }
    const acq = [];
    for (const f of files.filter(x => (x.lv || 1) === 1 || true)) acq.push(await fileInfo(f));
    return {
      format: FORMAT, created: new Date().toISOString(), language: wf.language, high_resolution: hr,
      program: wf.program,
      study_design: { files: wf.files.map(f => ({ ...f, role: roleOf(f.name) })), n_files: wf.files.length, roles: Object.fromEntries(["sample", "blank", "qc", "standard", "dark"].map(r => [r, wf.files.filter(f => roleOf(f.name) === r).length])) },
      acquisition: acq,
      processing: { parameters: wf.parameters, panels: wf.panels, operations: wf.operations },
      qaqc: qaFeat() ? qa : undefined,
      confidence_levels: hr ? (wf.candidates || candidates()) : undefined,
    };
  }

  // ---------------------------------------------------------------- HTML (standalone, printable)
  const CSS = `:root{--bg:#fff;--ink:#1d2127;--muted:#5a6270;--line:#d4d8de;--accent:#2b5c8a}@media (prefers-color-scheme:dark){:root{--bg:#14171b;--ink:#e4e7eb;--muted:#98a2b0;--line:#323944;--accent:#7db4e6}}
body{background:var(--bg);color:var(--ink);font:14px/1.45 system-ui,sans-serif;max-width:980px;margin:0 auto;padding:16px}h1{font-size:22px}h2{font-size:17px;border-bottom:2px solid var(--accent);padding-bottom:3px;margin-top:26px;color:var(--accent)}h3{font-size:14px;margin:14px 0 4px}
table{border-collapse:collapse;width:100%;font-size:12px;margin:6px 0}th,td{border-bottom:1px solid var(--line);padding:3px 6px;text-align:left;vertical-align:top}td.n{text-align:right;font-variant-numeric:tabular-nums}.m{color:var(--muted)}code,pre{font:11px/1.35 ui-monospace,monospace;word-break:break-all;white-space:pre-wrap}
@media print{body{max-width:none;padding:0}h2{break-after:avoid}table{break-inside:auto}tr{break-inside:avoid}}`;
  const num = (v, d = 2) => I18N.sci(v, d);
  const tab = (head, rows) => `<table><thead><tr>${head.map(h => `<th scope="col">${eh(h)}</th>`).join("")}</tr></thead><tbody>${rows.map(r => `<tr>${r.map(c => `<td${typeof c === "number" ? ' class="n"' : ""}>${eh(typeof c === "number" ? num(c, 3) : c)}</td>`).join("")}</tr>`).join("")}</tbody></table>`;
  const roleTxt = r => (window.QAQC ? QAQC.roleName(r) : r);
  const kv = o => Object.entries(o || {}).map(([k, v]) => [k, typeof v === "object" && v !== null ? JSON.stringify(v) : String(v)]);

  function html(rep) {
    const P = rep.program || {}, D = rep.study_design, A = rep.acquisition, X = rep.processing, Q = rep.qaqc;
    const qaOn = !!Q;      // section 4 (QA/QC) exists only when QA/QC is on: the numbering of the next one follows
    let h = `<!doctype html><html lang="${eh(rep.language || "en")}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>${eh(t("rep.title"))}</title><style>${CSS}</style></head><body>`;
    h += `<h1>${eh(t("rep.title"))}</h1><p class="m">${eh(t("rep.sub", { name: P.name || "", ver: P.version || "", commit: P.commit || "", date: (rep.created || "").slice(0, 19).replace("T", " ") }))}</p><p class="m">${eh(t(qaOn ? "rep.note" : "rep.note.plain"))}</p>`;
    h += `<h2>1. ${eh(t("rep.s1"))}</h2><p>${eh(t("rep.s1.sum", { n: D.n_files, s: D.roles.sample, b: D.roles.blank, q: D.roles.qc, st: D.roles.standard, d: D.roles.dark }))}</p>`;
    h += tab([t("qaqc.col.file"), t("qaqc.col.type"), t("rep.size"), t("rep.time"), "SHA-256"], D.files.map(f => [f.name, roleTxt(f.role), f.size, f.time == null ? "–" : String(f.time), `<${f.sha256}>`].map((c, i) => (i === 4 ? String(c).slice(1, -1) : c))));
    h += `<h2>2. ${eh(t("rep.s2"))}</h2><p class="m">${eh(t("rep.s2.note"))}</p>`;
    h += tab([t("qaqc.col.file"), t("rep.instr"), t("rep.analyzers"), t("rep.software"), t("rep.start"), t("rep.scans"), "RT (min)"], A.map(a => [a.name, a.error || [a.instrument, a.serial].filter(Boolean).join(" · "), (a.analyzers || []).join(", "), (a.software || []).join(", "), a.start || "–", a.scans == null ? "–" : String(a.scans), a.rt && a.rt.length ? a.rt.join(" – ") : "–"]));
    h += `<h2>3. ${eh(t("rep.s3"))}</h2><p>${eh(t("rep.s3.prog", { name: P.name || "", ver: P.version || "", commit: P.commit || "" }))}</p><h3>${eh(t("rep.params"))}</h3>${tab([t("rep.param"), t("rep.value")], kv(X.parameters).filter(r => r[0] !== "qaqc"))}`;
    h += `<h3>${eh(t("rep.ops"))}</h3>` + (X.operations && X.operations.length ? tab([t("rep.when"), t("rep.op"), t("rep.detail")], X.operations.map(o => { const { op, t: when, ...rest } = o; return [when || "", op || "", Object.keys(rest).length ? JSON.stringify(rest) : ""]; })) : `<p class="m">–</p>`);
    if (qaOn) {
      h += `<h2>4. ${eh(t("rep.s4"))}</h2><p class="m">${eh(t("rep.s4.note"))}</p>`;
      if (!Q.ions.length) h += `<p class="m">${eh(t("rep.s4.none"))}</p>`;
      Q.ions.forEach(i => {
        h += `<h3>${eh(i.label)} — m/z ${num(i.mz, 4)}, RT ${num(i.rt, 2)} (${eh(i.function)})</h3>`;
        h += tab([t("qaqc.col.file"), t("qaqc.col.type"), "RT", "ΔRT", t("qaqc.col.area"), t("qaqc.col.rel"), "Δppm"], i.files.map((f, k) => { const r = i.rows[k]; return [f, roleTxt(i.roles[k]), r ? r.rt : "–", r ? r.drt : "–", r ? r.area : "–", r ? r.rel : "–", r && r.dppm != null ? r.dppm : "–"]; }));
        h += `<p class="m">${eh(t("qaqc.sum.cv", { v: num(i.cv_area, 1) }))}${i.cv_area_qc != null ? " · " + eh(t("qaqc.sum.cvqc", { v: num(i.cv_area_qc, 1) })) : ""} · ${eh(t("qaqc.sum.drift", { v: num(i.rt_drift_per_file, 4) }))}${i.mass_accuracy ? " · " + eh(t("qaqc.sum.ppm", { mean: num(i.mass_accuracy.mean, 2), max: num(i.mass_accuracy.max_abs, 2), n: i.mass_accuracy.n })) : ""}</p>`;
      });
      const R = Q.sample_blank_ratio;
      if (R) h += `<h3>${eh(t("qaqc.ratio"))}</h3><p class="m">${eh(t("qaqc.ratio.sum", { mz: num(R.mz, 4), thr: R.threshold, blank: num(R.mean_blank_area, 0) }))}</p>` + tab([t("qaqc.col.file"), t("qaqc.col.type"), t("qaqc.col.area"), t("qaqc.ratio.col"), t("qaqc.ratio.atleast", { thr: R.threshold })], R.files.map((f, k) => { const q = R.ratios[k]; return [f, roleTxt(R.roles[k]), R.areas[k], !q ? "–" : q.absent_in_blank ? t("qaqc.ratio.absent") : q.ratio, !q ? "–" : t(q.at_least ? "qaqc.yes" : "qaqc.no")]; }));
      const F = Q.feature_mzfinder;
      if (F) { const n = F.rows.filter(x => x.at_least).length; h += `<h3>${eh(t("rep.feat"))}</h3><p class="m">${eh(t("qaqc.feat.sum", { n, tot: F.rows.length, thr: F.threshold, b: F.blanks.length }))}</p>`; }
    }
    if (rep.confidence_levels) {
      h += `<h2>${qaOn ? 5 : 4}. ${eh(t("rep.s5"))}</h2><p class="m">${eh(t("rep.s5.note"))}</p>`;
      h += rep.confidence_levels.length ? tab([t("rep.spectrum"), t("rep.cand"), t("rep.level"), t("rep.proposed"), t("rep.decided"), t("rep.evidence")], rep.confidence_levels.map(c => [c.spectrum, [c.candidate, c.library].filter(Boolean).join(" · "), c.level, c.proposed, c.decided || "–", (c.evidence || []).join("; ")])) : `<p class="m">${eh(t("rep.s5.none"))}</p>`;
    }
    return h + `</body></html>`;
  }

  async function exportReport(kind) {
    if (typeof E === "undefined" || !E.files || !E.files.filter(f => !f.gone).length) { if (typeof info === "function") info(t("err.report.noFiles")); return null; }
    const rep = await build();
    const base = "report_" + new Date().toISOString().slice(0, 10);
    if (kind === "json") dl(base + ".json", JSON.stringify(rep, null, 2), "application/json");
    else dl(base + ".html", html(rep), "text/html");
    if (window.WORKFLOW && WORKFLOW.logOp) WORKFLOW.logOp("export_report", { format: kind });
    return rep;
  }
  return { FORMAT, build, html, export: exportReport, note, candidates, setCandidates };
})();
window.RAPPORTO = RAPPORTO;
