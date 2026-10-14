// @ts-check
// TPMINE-PRIVATE  Step «Export and report» of the Feature table (window.PIPEX): buttons that download the files the engine writes for the view of the table chosen in the step Table
// (CSV for mzmine, CSV with the estimated cells, Excel, CSV with `;`, FBMN zip, SIRIUS) and the printable report. Every file and every number comes from the .wasm of the table
// (PIPEQC session: `file`, `report`; the MS2 files through the pool, which reads the .mzidx); this page only asks, lays out the report (HTML) and downloads. Texts in English.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"]/g, c => /** @type {any} */ ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
  const TXT = "text/plain;charset=utf-8";
  /** [id, label, title] of the buttons; the order is the order on the page */
  const BUTTONS = [
    ["mzmine", "CSV for mzmine", "Feature table in the format of mzmine; filled and targeted cells in the words of mzmine."],
    ["full", "CSV (estimated cells marked)", "The same table with the origin of every cell (detected, targeted, filled)."],
    ["xlsx", "Excel (.xlsx)", "The rows of the view as one sheet."],
    ["semi", "CSV (;)", "Text with ';' as separator, UTF-8 with BOM, for Excel in Italian."],
    ["fbmn", "GNPS2 (FBMN)", "Quantification table + MS2 (.mgf) in a .zip. Files only: nothing is sent."],
    ["sirius", "SIRIUS (.ms)", "MS2 of the rows of the view for SIRIUS 6. Files only: nothing is sent."],
    ["report", "Report (HTML)", "A page to print or save as PDF: the series, the view and the quality control."],
  ];
  const css = `#pipe .px-bar{display:flex;gap:8px;flex-wrap:wrap;margin:6px 0}#pipe .px-bar button{min-height:32px}@media print{#pipe .px-bar{display:none}}`;
  const html = () => `<details open><summary>Export and report</summary><p class="mut">Files of the rows kept by the filters of the step Table, with the areas normalised as chosen there.</p>
    <div class="px-bar">${BUTTONS.map(b => `<button type="button" id="px-${b[0]}" title="${esc(b[2])}">${esc(b[1])}</button>`).join("")}</div><div id="px-msg" class="mut" role="status"></div></details>`;

  /** the report as a page: all the numbers are those of `report/1` */
  function reportHtml(/** @type {any} */ r, /** @type {string[]} */ names) {
    const row = (/** @type {any[]} */ c, /** @type {string} */ t = "td") => `<tr>${c.map(x => `<${t}>${esc(x)}</${t}>`).join("")}</tr>`, f = G.PIPEQC.fmt, q = r.qc, cl = r.celle, v = r.vista;
    return `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Tabui report</title><style>body{font:14px/1.45 system-ui,sans-serif;max-width:900px;margin:24px auto;padding:0 16px;color:#111;background:#fff}table{border-collapse:collapse;margin:8px 0}td,th{border:1px solid #bbb;padding:3px 8px;text-align:left}code{word-break:break-all}</style></head><body>
<h1>Tabui feature table</h1><p>mzlab-feat ${esc(r.crate)}<br>Table SHA-256 <code>${esc(r.tabella_sha256)}</code><br>Samples SHA-256 <code>${esc(r.campioni_sha256)}</code></p>
<h2>Series</h2><p>${r.righe} rows, ${r.file} files: ${names.map(esc).join(", ")}.</p>
<table>${row(["Detected cells", "Targeted", "Filled", "Absent"], "th")}${row([cl.rilevate, cl.mirate, cl.riempite, cl.assenti])}</table>
<h2>View</h2>${v ? `<p>${v.conteggi.tenute} of ${v.conteggi.righe} rows kept.</p><p>Parameters: <code>${esc(JSON.stringify(v.parametri))}</code></p>` : "<p>No filter or normalisation: the whole table, areas as measured.</p>"}
<h2>Quality control</h2><table>${row(["File", "Detected", "Targeted", "Filled", "Absent", "Total area"], "th")}${q.file.map((/** @type {any} */ x) => row([x.file, x.rilevati, x.mirati, x.riempiti, x.assenti, f(x.area_totale)])).join("")}</table>
${q.gruppi.length ? `<table>${row(["Type", "Group", "Time (min)", "Files", "Rows", "Median CV"], "th")}${q.gruppi.map((/** @type {any} */ g) => row([g.tipo, g.gruppo, f(g.tempo_min), g.file, g.righe, f(g.cv_mediano)])).join("")}</table>` : "<p>No condition has two or more replicates.</p>"}
<p>Stability of the internal standard (CV of its area): ${q.standard_cv == null ? "no standard chosen" : f(q.standard_cv)}.</p></body></html>`;
  }

  /** mounts the step in `box`. o = { session (PIPEQC), spec() (JSON of the view), names, pool(), pol, say, stem } */
  function mount(/** @type {HTMLElement} */ box, /** @type {any} */ o) {
    box.innerHTML = html();
    const msg = (/** @type {string} */ t) => { const e = /** @type {HTMLElement} */ (box.querySelector("#px-msg")); e.textContent = t; };
    const btn = (/** @type {string} */ id) => /** @type {HTMLButtonElement} */ (box.querySelector("#px-" + id));
    const stem = o.stem || "tabui";
    /** the MS2 text of the engine (the pool reads the .mzidx of the files that hold the best MS2 of each row) */
    const ms2 = async (/** @type {string} */ kind) => (await o.pool()).ms2File({ table: o.session.bytes, samples: o.session.samples, spec: o.spec(), sha: o.session.sha, pol: o.pol, kind, paths: o.names.map((/** @type {string} */ n) => "mzlab/idx/" + n + ".mzidx") });
    const enc = new TextEncoder();
    /** @type {Record<string, () => Promise<void> | void>} */
    const act = {
      mzmine: async () => G.dl(stem + "_mzmine.csv", await o.session.file(o.spec(), "full_mzmine"), "text/csv;charset=utf-8"),
      full: async () => G.dl(stem + "_cells.csv", await o.session.file(o.spec(), "full"), "text/csv;charset=utf-8"),
      xlsx: async () => { const rows = await o.session.rows(o.spec()); G.dlx(stem + ".xlsx", [{ name: "Features", head: rows[0], rows: rows.slice(1) }]); },
      semi: async () => G.dl(stem + ".csv", "﻿" + await o.session.file(o.spec(), "csv_semicolon"), "text/csv;charset=utf-8"),
      fbmn: async () => G.dl("gnps2_" + stem + ".zip", G.zipStore([["quant.csv", enc.encode(await o.session.file(o.spec(), "quant"))], ["ms2.mgf", enc.encode(await ms2("mgf"))]]), "application/zip"),
      sirius: async () => G.dl("sirius_" + stem + ".ms", await ms2("sirius"), TXT),
      report: async () => G.dl(stem + "_report.html", reportHtml(await o.session.report(o.spec()), o.names), "text/html;charset=utf-8"),
    };
    for (const b of BUTTONS) btn(b[0]).onclick = async () => {
      btn(b[0]).disabled = true; msg("");
      try { await act[b[0]](); msg(`${b[1]}: done.`); } catch (/** @type {any} */ e) { msg(o.say ? o.say(e) : String(e.message || e)); }
      btn(b[0]).disabled = false;
    };
    return { act };
  }

  G.PIPEX = { mount, reportHtml, BUTTONS, css };
})();
