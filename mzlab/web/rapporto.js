"use strict";
// Study report (HTML to print + JSON), built from the workflow file of the session and the QA/QC measures; nothing leaves the computer.
// Sections in the order of the NORMAN guide (Hollender et al., Environ. Sci. Eur. 35:75, 2023) and the BP4NTA Study Reporting Tool:
//   1 study design (files, SHA-256, type of each file) · 2 acquisition (instrument and method, as the mzML says it)
//   3 processing (program version and commit, parameters, operations) · 4 QA/QC (measures of qaqc.js) · 5 confidence levels (HR, Schymanski)
// Numbers and evidence only: the report never says what a compound is.
const RAPPORTO = (() => {
  const FORMAT = "mzreport/1";
  const t = (k, p) => I18N.t(k, p);
  const eh = s => String(s == null ? "" : s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  const isHr = () => !!(window.HR && typeof BANCO !== "undefined" && BANCO.on());
  const cands = new Map();                           // candidates annotated in the session (confidence level and evidence), kept in the workflow file

  // libreria.js calls this every time it assesses the candidates of one spectrum (and again after the chemist's decision)
  function note(lv, q) {
    if (!lv) return;
    const b = lv.best || lv.nearest || lv.analog || null;
    const key = (q && q.title || "") + "|" + (q && q.prec != null ? q.prec : "");
    cands.set(key, { spettro: q && q.title || "", precursore: q && q.prec != null ? q.prec : null, candidato: b ? (b.name || "") : "", libreria: b ? (b.lib || "") : "",
      proposto: lv.proposed, deciso: lv.decided || null, livello: lv.decided || lv.proposed, profilo: lv.profile || "", prove: (lv.lines || []).slice(), motivo: lv.decisive || "" });
  }
  const candidates = () => [...cands.values()];
  const setCandidates = a => { cands.clear(); (a || []).forEach(c => cands.set((c.spettro || "") + "|" + (c.precursore != null ? c.precursore : ""), c)); };

  async function fileInfo(f) {
    try { const j = await J(`api/fileinfo?k=${f.k}`); return { nome: f.file, strumento: j.instrument || "", seriale: j.serial || "", componenti: j.components || [], analizzatori: j.analyzers || [], software: j.software || [],
      conversione: j.conversion || [], inizio: j.start || "", scansioni: j.scans, livelli: j.levels || {}, rt: j.rt || [], filtri: Object.keys(j.filters || {}).length ? j.filters : undefined, mz: j.mz_range || undefined }; }
    catch (e) { return { nome: f.file, errore: String(e && e.message || e) }; }
  }
  const slim = r => ({ mz: r.mz, rt: r.rt, finestra_rt: r.win, tolleranza: r.tol, unita: r.unit, mediana_area: r.median_area, deriva_rt_per_file: r.rt_drift_per_file, deriva_area_per_file: r.area_drift_per_file,
    cv_area: r.cv_area, cv_area_qc: r.cv_area_qc, accuratezza_massa: r.mass_accuracy, righe: r.rows });

  // The JSON: from the workflow file (current session, or the one the caller passes) plus the measures
  async function build(wf) {
    wf = wf || WORKFLOW.buildWorkflow();
    if (window.QAQC) { if (!wf.parametri.qaqc) wf.parametri.qaqc = QAQC.exportState(); }
    const files = (typeof E !== "undefined" ? E.files : []).filter(f => !f.gone);
    const roleOf = n => { const f = files.find(x => x.file === n); return f && window.QAQC ? QAQC.roleOf(f) : "sample"; };
    const hr = isHr();
    const qa = { ioni: [], rapporto_campione_bianco: null, feature_mzfinder: null };
    if (window.QAQC) {
      const r = await QAQC.run();
      qa.ioni = r.ions.filter(o => !o.error).map(o => ({ etichetta: o.ion.label, funzione: o.ion.kind === "known" ? "ione noto" : "standard interno", file: o.files, ruoli: o.roles, ...slim(o.r) }));
      if (QAQC.state.ratio.text) { try { const q = await QAQC.runRatio(); qa.rapporto_campione_bianco = { mz: q.mz, rt: q.rt, soglia: q.thr, area_media_bianchi: q.r.ratio.blank_area, file: q.files, ruoli: q.roles, aree: q.r.rows.map(x => (x ? x.area : 0)), rapporti: q.r.ratio.rows }; } catch (_) { /* ion not found */ } }
      if (hr) { try { const f = await QAQC.runFeatures(); if (f) qa.feature_mzfinder = { soglia: f.thr, bianchi: f.blanks, campioni: f.samples, righe: f.rows }; } catch (_) { /* no table */ } }
    }
    const acq = [];
    for (const f of files.filter(x => (x.lv || 1) === 1 || true)) acq.push(await fileInfo(f));
    return {
      formato: FORMAT, creato: new Date().toISOString(), lingua: wf.lingua, alta_risoluzione: hr,
      programma: wf.programma,
      disegno_studio: { file: wf.file.map(f => ({ ...f, ruolo: roleOf(f.nome) })), n_file: wf.file.length, ruoli: Object.fromEntries(["sample", "blank", "qc", "standard", "dark"].map(r => [r, wf.file.filter(f => roleOf(f.nome) === r).length])) },
      acquisizione: acq,
      elaborazione: { parametri: wf.parametri, pannelli: wf.pannelli, operazioni: wf.operazioni },
      qaqc: qa,
      livelli_confidenza: hr ? (wf.candidati || candidates()) : undefined,
    };
  }

  // ---------------------------------------------------------------- HTML (standalone, printable)
  const CSS = `:root{--bg:#fff;--ink:#1d2127;--muted:#5a6270;--line:#d4d8de;--accent:#2b5c8a}@media (prefers-color-scheme:dark){:root{--bg:#14171b;--ink:#e4e7eb;--muted:#98a2b0;--line:#323944;--accent:#7db4e6}}
body{background:var(--bg);color:var(--ink);font:14px/1.45 system-ui,sans-serif;max-width:980px;margin:0 auto;padding:16px}h1{font-size:22px}h2{font-size:17px;border-bottom:2px solid var(--accent);padding-bottom:3px;margin-top:26px;color:var(--accent)}h3{font-size:14px;margin:14px 0 4px}
table{border-collapse:collapse;width:100%;font-size:12px;margin:6px 0}th,td{border-bottom:1px solid var(--line);padding:3px 6px;text-align:left;vertical-align:top}td.n{text-align:right;font-variant-numeric:tabular-nums}.m{color:var(--muted)}code,pre{font:11px/1.35 ui-monospace,monospace;word-break:break-all;white-space:pre-wrap}
@media print{body{max-width:none;padding:0}h2{break-after:avoid}table{break-inside:auto}tr{break-inside:avoid}}`;
  const num = (v, d = 2) => (v == null || !isFinite(v) ? "–" : Math.abs(v) >= 1e4 || (v !== 0 && Math.abs(v) < 0.01) ? Number(v).toExponential(2) : Number(v).toFixed(d));
  const tab = (head, rows) => `<table><thead><tr>${head.map(h => `<th scope="col">${eh(h)}</th>`).join("")}</tr></thead><tbody>${rows.map(r => `<tr>${r.map(c => `<td${typeof c === "number" ? ' class="n"' : ""}>${eh(typeof c === "number" ? num(c, 3) : c)}</td>`).join("")}</tr>`).join("")}</tbody></table>`;
  const roleTxt = r => (window.QAQC ? QAQC.roleName(r) : r);
  const kv = o => Object.entries(o || {}).map(([k, v]) => [k, typeof v === "object" && v !== null ? JSON.stringify(v) : String(v)]);

  function html(rep) {
    const P = rep.programma || {}, D = rep.disegno_studio, A = rep.acquisizione, X = rep.elaborazione, Q = rep.qaqc;
    let h = `<!doctype html><html lang="${eh(rep.lingua || "it")}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>${eh(t("rep.title"))}</title><style>${CSS}</style></head><body>`;
    h += `<h1>${eh(t("rep.title"))}</h1><p class="m">${eh(t("rep.sub", { name: P.nome || "", ver: P.versione || "", commit: P.commit || "", date: (rep.creato || "").slice(0, 19).replace("T", " ") }))}</p><p class="m">${eh(t("rep.note"))}</p>`;
    h += `<h2>1. ${eh(t("rep.s1"))}</h2><p>${eh(t("rep.s1.sum", { n: D.n_file, s: D.ruoli.sample, b: D.ruoli.blank, q: D.ruoli.qc, st: D.ruoli.standard, d: D.ruoli.dark }))}</p>`;
    h += tab([t("qaqc.col.file"), t("qaqc.col.type"), t("rep.size"), t("rep.time"), "SHA-256"], D.file.map(f => [f.nome, roleTxt(f.ruolo), f.dimensione, f.tempo == null ? "–" : String(f.tempo), `<${f.sha256}>`].map((c, i) => (i === 4 ? String(c).slice(1, -1) : c))));
    h += `<h2>2. ${eh(t("rep.s2"))}</h2><p class="m">${eh(t("rep.s2.note"))}</p>`;
    h += tab([t("qaqc.col.file"), t("rep.instr"), t("rep.analyzers"), t("rep.software"), t("rep.start"), t("rep.scans"), "RT (min)"], A.map(a => [a.nome, a.errore || [a.strumento, a.seriale].filter(Boolean).join(" · "), (a.analizzatori || []).join(", "), (a.software || []).join(", "), a.inizio || "–", a.scansioni == null ? "–" : String(a.scansioni), a.rt && a.rt.length ? a.rt.join(" – ") : "–"]));
    h += `<h2>3. ${eh(t("rep.s3"))}</h2><p>${eh(t("rep.s3.prog", { name: P.nome || "", ver: P.versione || "", commit: P.commit || "" }))}</p><h3>${eh(t("rep.params"))}</h3>${tab([t("rep.param"), t("rep.value")], kv(X.parametri).filter(r => r[0] !== "qaqc"))}`;
    h += `<h3>${eh(t("rep.ops"))}</h3>` + (X.operazioni && X.operazioni.length ? tab([t("rep.when"), t("rep.op"), t("rep.detail")], X.operazioni.map(o => { const { op, t: when, ...rest } = o; return [when || "", op || "", Object.keys(rest).length ? JSON.stringify(rest) : ""]; })) : `<p class="m">–</p>`);
    h += `<h2>4. ${eh(t("rep.s4"))}</h2><p class="m">${eh(t("rep.s4.note"))}</p>`;
    if (!Q.ioni.length) h += `<p class="m">${eh(t("rep.s4.none"))}</p>`;
    Q.ioni.forEach(i => {
      h += `<h3>${eh(i.etichetta)} — m/z ${num(i.mz, 4)}, RT ${num(i.rt, 2)} (${eh(i.funzione)})</h3>`;
      h += tab([t("qaqc.col.file"), t("qaqc.col.type"), "RT", "ΔRT", t("qaqc.col.area"), t("qaqc.col.rel"), "Δppm"], i.file.map((f, k) => { const r = i.righe[k]; return [f, roleTxt(i.ruoli[k]), r ? r.rt : "–", r ? r.drt : "–", r ? r.area : "–", r ? r.rel : "–", r && r.dppm != null ? r.dppm : "–"]; }));
      h += `<p class="m">${eh(t("qaqc.sum.cv", { v: num(i.cv_area, 1) }))}${i.cv_area_qc != null ? " · " + eh(t("qaqc.sum.cvqc", { v: num(i.cv_area_qc, 1) })) : ""} · ${eh(t("qaqc.sum.drift", { v: num(i.deriva_rt_per_file, 4) }))}${i.accuratezza_massa ? " · " + eh(t("qaqc.sum.ppm", { mean: num(i.accuratezza_massa.mean, 2), max: num(i.accuratezza_massa.max_abs, 2), n: i.accuratezza_massa.n })) : ""}</p>`;
    });
    const R = Q.rapporto_campione_bianco;
    if (R) h += `<h3>${eh(t("qaqc.ratio"))}</h3><p class="m">${eh(t("qaqc.ratio.sum", { mz: num(R.mz, 4), thr: R.soglia, blank: num(R.area_media_bianchi, 0) }))}</p>` + tab([t("qaqc.col.file"), t("qaqc.col.type"), t("qaqc.col.area"), t("qaqc.ratio.col"), t("qaqc.ratio.atleast", { thr: R.soglia })], R.file.map((f, k) => { const q = R.rapporti[k]; return [f, roleTxt(R.ruoli[k]), R.aree[k], !q ? "–" : q.absent_in_blank ? t("qaqc.ratio.absent") : q.ratio, !q ? "–" : t(q.at_least ? "qaqc.yes" : "qaqc.no")]; }));
    const F = Q.feature_mzfinder;
    if (F) { const n = F.righe.filter(x => x.at_least).length; h += `<h3>${eh(t("rep.feat"))}</h3><p class="m">${eh(t("qaqc.feat.sum", { n, tot: F.righe.length, thr: F.soglia, b: F.bianchi.length }))}</p>`; }
    if (rep.livelli_confidenza) {
      h += `<h2>5. ${eh(t("rep.s5"))}</h2><p class="m">${eh(t("rep.s5.note"))}</p>`;
      h += rep.livelli_confidenza.length ? tab([t("rep.spectrum"), t("rep.cand"), t("rep.level"), t("rep.proposed"), t("rep.decided"), t("rep.evidence")], rep.livelli_confidenza.map(c => [c.spettro, [c.candidato, c.libreria].filter(Boolean).join(" · "), c.livello, c.proposto, c.deciso || "–", (c.prove || []).join("; ")])) : `<p class="m">${eh(t("rep.s5.none"))}</p>`;
    }
    return h + `</body></html>`;
  }

  async function exportReport(kind) {
    if (typeof E === "undefined" || !E.files || !E.files.filter(f => !f.gone).length) { if (typeof info === "function") info(t("err.workflow.noFiles")); return null; }
    const rep = await build();
    const base = "rapporto_" + new Date().toISOString().slice(0, 10);
    if (kind === "json") dl(base + ".json", JSON.stringify(rep, null, 2), "application/json");
    else dl(base + ".html", html(rep), "text/html");
    if (window.WORKFLOW && WORKFLOW.logOp) WORKFLOW.logOp("export_report", { formato: kind });
    return rep;
  }
  return { FORMAT, build, html, export: exportReport, note, candidates, setCandidates };
})();
window.RAPPORTO = RAPPORTO;
