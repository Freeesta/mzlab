// @ts-check
// TPMINE-PRIVATE  Table engine and step «Quality control» of the Feature table (window.PIPEQC). The third .wasm of mzFinder (crates/mzlab-feat-table-wasm, loaded here the first time a table is shown)
// gives every number: qc/1, the view (counts of the rows kept) and the rows of the sheet; this file only loads it, draws and formats. CONTRACT lists the keys of those JSON that the page reads
// (tests/test_pipeline_contract.py checks them against the real output of the engine). Every chart has its table (TDATI). Texts in English (mzFinder convention).
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"]/g, c => /** @type {any} */ ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
  /** keys of the JSON of the engine that the page shows or uses */
  const CONTRACT = {
    qc: ["schema", "file", "gruppi", "standard_cv", "bianchi", "m1_su_m", "scartate"], blank: ["file", "righe", "righe_con_picco"], m1: ["righe", "min", "mediana", "max"], rejected: ["regola", "righe"], qcFile: ["file", "rilevati", "mirati", "riempiti", "assenti", "area_totale"], qcGroup: ["tipo", "gruppo", "tempo_min", "file", "righe", "cv_mediano"],
    view: ["digest", "righe", "fattori", "conteggi"],
    row: ["schema", "id", "mz", "rt", "rt_lo", "rt_hi", "charge", "quality", "iso_m1", "ms2", "ident", "cells"], rowQuality: ["shape", "snr", "fwhm", "flags"], rowMs2: ["file", "purity"], rowIdent: ["neutral", "adduct", "sources"], rowCell: ["file", "type", "time", "area", "height", "apex_rt", "width", "state"],
    traces: ["schema", "id", "mz", "ppm", "xic", "ms1", "ms2"], traceXic: ["file", "rt", "y"], traceMs1: ["file", "rt", "mz", "y"], traceMs2: ["file", "prec", "mz", "y"], counts: ["righe", "tenute", "scartate"],
    report: ["crate", "tabella_sha256", "campioni_sha256", "righe", "file", "celle", "vista", "qc"], cells: ["rilevate", "mirate", "riempite", "assenti"], reportView: ["parametri", "conteggi"], rows: ["hojas"], sheet: ["nombre", "filas"],
  };
  const hex = (/** @type {ArrayBuffer} */ b) => [...new Uint8Array(b)].map(x => x.toString(16).padStart(2, "0")).join("");
  /** @type {any} */ let mod = null;

  /** the .wasm of the table, verified against the SHA-256 of wasm-feat/manifest.json */
  async function engine(/** @type {string} */ base) {
    if (mod) return mod;
    const u = (/** @type {string} */ p) => new URL("wasm-feat/" + p, base).href, man = await (await fetch(u("manifest.json"))).json(), t = man.table;
    if (!t) throw Object.assign(new Error("error.featFailed"), { key: "error.featFailed" });
    const bytes = await (await fetch(u(t.wasm))).arrayBuffer();
    if (t.sha256 && hex(await crypto.subtle.digest("SHA-256", bytes)) !== t.sha256) throw Object.assign(new Error("error.featWasmHash"), { key: "error.featWasmHash" });
    const m = await import(u(t.js));
    await m.default({ module_or_path: bytes });
    mod = m;
    return m;
  }
  /** an error of the engine is a JSON text `{error_key, params}`: becomes an Error with `key` and `params` */
  const fail = (/** @type {any} */ e) => {
    let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ }
    return Object.assign(new Error(j ? j.error_key : String(e && e.message || e)), j ? { key: j.error_key, params: j.params } : {});
  };

  /** the campioni/1 of the engine with the roles and times the user confirmed (campioni/2, same order of the files): the types of campioni/1 have no infusion, which counts as a sample without a time */
  function withRoles(/** @type {string} */ proposed, /** @type {any[]} */ cs) {
    const doc = JSON.parse(proposed);
    if (doc.campioni.length !== cs.length) return proposed;
    doc.campioni.forEach((/** @type {any} */ s, /** @type {number} */ i) => {
      const c = cs[i], inf = c.tipo === "infusion";
      s.tipo = inf ? "sample" : c.tipo; s.tempo_min = inf ? null : c.tempo_min;
    });
    return JSON.stringify(doc);
  }

  /** the traces of a row from the .mzidx of the series, read by the pool (OPFS in the page, the index folder in the app); `a.pool` gives it */
  async function tracesVia(/** @type {any} */ a, /** @type {Uint8Array} */ bytes, /** @type {string} */ samples, /** @type {number} */ id, /** @type {number} */ ppm) {
    const pool = await a.pool();
    return pool.rowTraces({ table: bytes, samples, sha: await G.UNT.sha256Hex(bytes), id, ppm, paths: a.names.map((/** @type {string} */ n) => "mzlab/idx/" + n + ".mzidx") });
  }

  /** candidate compositions of a row (`formula_id/1`) from its MS1 spectrum, computed by the pool (the `.wasm` `untargeted` in the page, the native route in the app) */
  async function compositionVia(/** @type {any} */ a, /** @type {number[]} */ mz, /** @type {number[]} */ y, /** @type {number} */ featureMz, /** @type {string} */ params) {
    const pool = await a.pool();
    return JSON.parse(await pool.formulaId({ mz, y, featureMz, params }));
  }

  /** a session on one table: `a` = { base, bytes (.mzfeat), names (stems of the files), inputs ([{name, sha256}] of the table), pol, pool() (for the traces) } */
  async function open(/** @type {any} */ a) {
    if (window.MZLAB_DESKTOP) return openNative(a);
    const m = await engine(a.base), bytes = /** @type {Uint8Array} */ (a.bytes), size = bytes.length;
    const read = (/** @type {any} */ _, /** @type {number} */ off, /** @type {number} */ len) => bytes.slice(off, off + len);
    const sha = await G.UNT.sha256Hex(bytes);
    /** @type {(fn: () => string) => any} */ const run = fn => { try { return fn(); } catch (e) { throw fail(e); } };
    const proposed = run(() => m.table_samples(JSON.stringify({ files: a.names.map((/** @type {string} */ n, /** @type {number} */ i) => ({ name: n + ".mzML", sha256: (a.inputs[i] || {}).sha256 || "" })) })));
    const samples = a.campioni ? withRoles(proposed, a.campioni) : proposed;
    return {
      samples, sha, size, bytes,
      qc: (/** @type {string} */ spec) => JSON.parse(run(() => m.table_qc(read, size, samples, spec))),
      view: (/** @type {string} */ spec) => JSON.parse(run(() => m.table_view(read, size, samples, spec))),
      rows: (/** @type {string} */ spec) => JSON.parse(run(() => m.export_table(read, size, samples, spec, sha, a.pol, "xlsx_rows"))).hojas[0].filas,
      report: (/** @type {string} */ spec) => JSON.parse(run(() => m.table_report(read, size, samples, spec, sha))),
      row: (/** @type {number} */ id) => JSON.parse(run(() => m.table_row(read, size, samples, id))),
      traces: async (/** @type {number} */ id, /** @type {number} */ ppm) => JSON.parse(await tracesVia(a, bytes, samples, id, ppm)),
      composition: (/** @type {number[]} */ mz, /** @type {number[]} */ y, /** @type {number} */ featureMz, /** @type {string} */ params) => compositionVia(a, mz, y, featureMz, params),
      file: (/** @type {string} */ spec, /** @type {string} */ kind) => run(() => m.export_table(read, size, samples, spec, sha, a.pol, kind)),
    };
  }

  /** the same session in the app: every call is the native route `feat/table` (the table the last step of the Pipeline holds in the app, named by its SHA-256), so the methods answer with a Promise */
  async function openNative(/** @type {any} */ a) {
    const bytes = /** @type {Uint8Array} */ (a.bytes), size = bytes.length, sha = await G.UNT.sha256Hex(bytes);
    const url = URL.createObjectURL(new Blob([window.QTOOLS.ctx.files["feat-desktop-pool.js"]], { type: "text/javascript" }));
    const pool = (await import(url)).createDesktopPool();
    /** @type {(kind: string, p: Record<string, any>) => Promise<string>} */ const T = (kind, p) => pool.table(kind, p);       // its errors carry `key` and `params`
    const proposed = await T("samples", { files: JSON.stringify({ files: a.names.map((/** @type {string} */ n, /** @type {number} */ i) => ({ name: n + ".mzML", sha256: (a.inputs[i] || {}).sha256 || "" })) }) });
    const samples = a.campioni ? withRoles(proposed, a.campioni) : proposed;
    /** @type {(kind: string, spec: string, more?: Record<string, any>) => Promise<string>} */ const V = (kind, spec, more) => T(kind, { samples, spec, sha256: sha, polarity: a.pol, ...more });
    return {
      samples, sha, size, bytes,
      qc: async (/** @type {string} */ spec) => JSON.parse(await V("qc", spec)),
      view: async (/** @type {string} */ spec) => JSON.parse(await V("view", spec)),
      rows: async (/** @type {string} */ spec) => JSON.parse(await V("export", spec, { export_kind: "xlsx_rows" })).hojas[0].filas,
      report: async (/** @type {string} */ spec) => JSON.parse(await V("report", spec)),
      row: async (/** @type {number} */ id) => JSON.parse(await V("row", "", { id })),
      traces: async (/** @type {number} */ id, /** @type {number} */ ppm) => JSON.parse(await tracesVia(a, bytes, samples, id, ppm)),
      composition: (/** @type {number[]} */ mz, /** @type {number[]} */ y, /** @type {number} */ featureMz, /** @type {string} */ params) => compositionVia(a, mz, y, featureMz, params),
      file: (/** @type {string} */ spec, /** @type {string} */ kind) => V("export", spec, { export_kind: kind }),
    };
  }

  // ------------------------------------------------------------ step 5: quality control
  const KINDS = [["rilevati", "D", "detected"], ["mirati", "T", "targeted"], ["riempiti", "F", "filled"], ["assenti", "A", "absent"]];
  const fmt = (/** @type {any} */ v) => (v == null ? "" : Math.abs(v) >= 1e4 || (v !== 0 && Math.abs(v) < 0.01) ? Number(v).toExponential(2) : String(+Number(v).toPrecision(4)));
  /** the reason of each rule of the filters, in words (the number of rows is the engine's) */
  const WHY = /** @type {Record<string, string>} */ ({ min_rilevati: "detected in too few files", min_area: "area below the minimum", min_altezza: "height below the minimum", bianco: "not above the blank", mz: "m/z out of range", rt: "RT out of range", ms2: "no MS2" });
  const W = 360, ROW = 22, M = { l: 120, r: 8, t: 6, b: 6 };

  /** stacked bars of the cells of each file (D, T, F, A) */
  function matrixSvg(/** @type {any[]} */ files) {
    const top = Math.max(1, ...files.map(f => KINDS.reduce((s, k) => s + f[k[0]], 0))), H = M.t + M.b + files.length * ROW;
    let s = `<svg viewBox="0 0 ${W} ${H}" role="img">`;
    files.forEach((f, i) => {
      let x = M.l;
      s += `<text x="${M.l - 4}" y="${M.t + i * ROW + 14}" text-anchor="end">${esc(String(f.file).slice(0, 18))}</text>`;
      KINDS.forEach((k, j) => { const w = (W - M.l - M.r) * f[k[0]] / top; s += `<rect class="qcb qcb${j}" x="${x.toFixed(1)}" y="${M.t + i * ROW + 3}" width="${w.toFixed(1)}" height="${ROW - 6}"/>`; x += w; });
    });
    return s + "</svg>";
  }
  const qcCss = `#pipe svg.qcl{display:inline;width:12px;vertical-align:-1px}#pipe .qcb{stroke:var(--panel)}#pipe .qcb0{fill:var(--pv-d)}#pipe .qcb1{fill:var(--pv-t)}#pipe .qcb2{fill:var(--pv-f)}#pipe .qcb3{fill:var(--pv-a)}#pipe .qc table{border-collapse:collapse;font-size:12px;margin:6px 0}#pipe .qc td,#pipe .qc th{padding:2px 8px;text-align:left}#pipe .qc .n{text-align:right}`;

  /** renders the step into `box`; `o` = { session, chart (PIPE.chart: box, svg, title, desc, table) } */
  async function render(/** @type {HTMLElement} */ box, /** @type {any} */ o) {
    const q = await o.session.qc(""), files = q.file;
    box.innerHTML = `<details open class="qc"><summary>Quality control</summary><p class="mut">Numbers of the engine for the whole table; no threshold is applied. D detected, T targeted, F filled, A absent.</p>
      <div class="charts"><div class="ch" id="qc-m"></div></div>
      <p class="mut" aria-hidden="true">${KINDS.map((k, j) => `<svg class="qcl" width="12" height="12"><rect class="qcb qcb${j}" width="12" height="12"/></svg> ${k[1]} ${k[2]}`).join(" &nbsp; ")}</p>
      <table><caption class="mut">Cells and area per file</caption><tr><th>File</th>${KINDS.map(k => `<th class="n" title="${k[2]}">${k[1]}</th>`).join("")}<th class="n">Total area</th></tr>
      ${files.map((/** @type {any} */ f) => `<tr><td>${esc(f.file)}</td>${KINDS.map(k => `<td class="n">${f[k[0]]}</td>`).join("")}<td class="n">${fmt(f.area_totale)}</td></tr>`).join("")}</table>
      ${q.gruppi.length ? `<table><caption class="mut">Agreement of the replicates: median CV (fraction) of the rows with a peak in every replicate</caption><tr><th>Type</th><th>Group</th><th class="n">Time (min)</th><th class="n">Files</th><th class="n">Rows</th><th class="n">CV</th></tr>
      ${q.gruppi.map((/** @type {any} */ g) => `<tr><td>${esc(g.tipo)}</td><td>${esc(g.gruppo)}</td><td class="n">${fmt(g.tempo_min)}</td><td class="n">${g.file}</td><td class="n">${g.righe}</td><td class="n">${fmt(g.cv_mediano)}</td></tr>`).join("")}</table>` : `<p class="mut">No condition has two or more replicates: no agreement to report.</p>`}
      ${q.bianchi.file ? `<p class="mut">Rows with a peak in a blank: ${q.bianchi.righe_con_picco.toLocaleString("en")} of ${q.bianchi.righe.toLocaleString("en")} (${q.bianchi.file} blank file${q.bianchi.file > 1 ? "s" : ""}).</p>` : `<p class="mut">No blank file in the series.</p>`}
      ${q.m1_su_m ? `<p class="mut">M+1/M of ${q.m1_su_m.righe.toLocaleString("en")} rows: median ${fmt(q.m1_su_m.mediana)} (from ${fmt(q.m1_su_m.min)} to ${fmt(q.m1_su_m.max)}).</p>` : ""}
      <p class="mut">Stability of the internal standard (CV of its area): ${q.standard_cv == null ? "no standard chosen" : fmt(q.standard_cv)}</p></details>`;
    o.chart(box.querySelector("#qc-m"), matrixSvg(files), "Cells per file", `Stacked bars of the cells of each of the ${files.length} files: detected, targeted, filled and absent.`,
      { head: ["File", ...KINDS.map(k => k[2])], rows: files.map((/** @type {any} */ f) => [f.file, ...KINDS.map(k => String(f[k[0]]))]) });
    return q;
  }

  G.PIPEQC = { open, render, engine, CONTRACT, fmt, WHY, css: qcCss };
})();
