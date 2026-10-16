// @ts-check
// TPMINE-PRIVATE  Step «Table» of the Feature table (window.PIPET): normalisation and filters as a view of the table (the engine counts «N of M rows» and the rows), and the rows of the view
// in a TPTAB (`#ft-t`): few columns, the first two fixed, one row open at a time with TPRIGA (head, Overview, Samples, Identity, Quality). The open row asks the engine for `session.row(id)`
// (`riga/1`) and `session.traces(id, ppm)` (`riga_tracce/1`, XIC and spectra from the `.mzidx` of the series, not from the file open in Data). Nothing is computed here: the page builds the
// parameters of the view, formats and draws. Texts in English (mzFinder convention); the helps are keys of the catalogue of Tabui.
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  /** the ratio over the blank of the chip «Not in blank» (the same number as the field «Blank ratio») */
  const BLANK_CHIP = 3, DETECTED_CHIP = 2;
  /** the states of the sheet of the engine, as TPTAB names them */
  const STATE = /** @type {Record<string, string>} */ ({ DETECTED: "detected", FILLED: "filled", TARGETED: "targeted", UNKNOWN: "absent" });
  const css = `#pipe .ft-bar{display:flex;gap:6px 10px;align-items:center;flex-wrap:wrap;margin:6px 0}#pipe .ft-bar label,#pipe .ft-more label{display:flex;flex-direction:column;gap:2px;font-size:12px}#pipe .ft-bar select{min-height:28px;box-sizing:border-box}
#pipe .ft-more{display:flex;gap:8px 10px;align-items:flex-end;flex-wrap:wrap;margin:0 0 6px}#pipe .ft-more[hidden]{display:none}#pipe .ft-more input{width:92px;min-height:28px;box-sizing:border-box}
#pipe .ft-bar .tpt-chip:disabled{opacity:.55;cursor:not-allowed}#pipe #ft-host .tpt-n{display:none}#pipe .ft-n{text-align:right}`;

  const num = (/** @type {HTMLInputElement} */ el) => (el.value.trim() === "" ? null : +el.value);
  /** the parameters of the view from the form and the chips (the text `normalizza`/`filtra` is the language of the engine) */
  function specOf(/** @type {(s: string) => HTMLInputElement} */ q, /** @type {Set<string>} */ chips = new Set()) {
    const filtra = [], rule = (/** @type {any} */ o) => filtra.push(o);
    const det = Math.max(num(q("#ft-det")) || 0, chips.has("det") ? DETECTED_CHIP : 0), area = num(q("#ft-area")), blank = num(q("#ft-blank")) || (chips.has("blank") ? BLANK_CHIP : null);
    const mz = [num(q("#ft-mz0")), num(q("#ft-mz1"))], rt = [num(q("#ft-rt0")), num(q("#ft-rt1"))];
    if (det) rule({ regola: "min_rilevati", valore: det });
    if (area) rule({ regola: "min_area", valore: area });
    if (blank) rule({ regola: "bianco", valore: blank });
    if (mz[0] != null || mz[1] != null) rule({ regola: "mz", da: mz[0] ?? 0, a: mz[1] ?? 1e9 });
    if (rt[0] != null || rt[1] != null) rule({ regola: "rt", da: rt[0] ?? 0, a: rt[1] ?? 1e9 });
    if (chips.has("ms2")) rule({ regola: "ms2" });
    return JSON.stringify({ normalizza: { metodo: q("#ft-norm").value }, filtra });
  }
  const html = () => `<details open><summary>Table</summary><div class="tp"><div class="ft-bar">
    <button type="button" class="tpt-chip" data-fc="blank" aria-pressed="false" data-tt="ft.chip.noblank">Not in blank</button><button type="button" class="tpt-chip" data-fc="det" aria-pressed="false" data-tt="ft.chip.det2">Detected in ≥ ${DETECTED_CHIP} files</button>
    <button type="button" class="tpt-chip" data-fc="ms2" aria-pressed="false" data-tt="ft.chip.ms2">With MS2</button><button type="button" class="tpt-chip" id="ft-moreb" aria-expanded="false" aria-controls="ft-more" data-tt="ft.more.filters">More filters</button>
    <label data-tt="ft.norm">Normalisation<select id="ft-norm"><option value="nessuna">none</option><option value="somma">sum of the areas</option><option value="mediana">median of the areas</option></select></label></div>
    <div class="ft-more" id="ft-more" hidden><label>Detected in at least (files)<input id="ft-det" type="number" min="0" step="1"></label><label>Area at least<input id="ft-area" type="number" min="0" step="any"></label>
    <label>Blank ratio (area over blank)<input id="ft-blank" type="number" min="0" step="any"></label><label><i>m/z</i> from<input id="ft-mz0" type="number" step="any"></label><label>to<input id="ft-mz1" type="number" step="any"></label>
    <label>RT from (min)<input id="ft-rt0" type="number" step="any"></label><label>to<input id="ft-rt1" type="number" step="any"></label></div>
    <div class="mut" id="ft-n" role="status"></div><div class="mut" id="ft-why"></div><div id="ft-host"></div></div></details>`;

  const f2 = (/** @type {any} */ v, /** @type {number} */ d) => (typeof v === "number" && Number.isFinite(v) ? v.toFixed(d) : "");
  const sci = (/** @type {number} */ v) => (v > 0 ? v.toExponential(1) : "");
  /** @typedef {{ id: number, mz: number, rt: number, lo: number | null, hi: number | null, charge: number | null, areas: number[], states: string[] }} FtRow */
  /** a row of the sheet of the engine (`xlsx_rows`: id, mz, rt, rt_range min and max, charge, then area and state per file) @returns {FtRow} */
  function rowOf(/** @type {any[]} */ r, /** @type {number} */ n) {
    const areas = [], states = [];
    for (let j = 0; j < n; j++) { areas.push(+r[6 + 2 * j] || 0); states.push(STATE[r[7 + 2 * j]] || "absent"); }
    return { id: r[0], mz: r[1], rt: r[2], lo: r[3] ?? null, hi: r[4] ?? null, charge: r[5] ?? null, areas, states };
  }

  /** the columns of the table and the two readers they share (identity text, level) */
  function columns(/** @type {any} */ o, /** @type {number} */ n, /** @type {(s: any) => string} */ esc) {
    const TT = G.TPTAB;
    // the order of the timed samples for the trend: t0 and the treated ones, by minutes
    const timed = o.cs ? o.cs.map((/** @type {any} */ c, /** @type {number} */ i) => ({ i, t: c.tipo === "t0" ? 0 : c.tipo === "sample" ? c.tempo_min : null })).filter((/** @type {any} */ x) => x.t != null).sort((/** @type {any} */ a, /** @type {any} */ b) => a.t - b.t) : [];
    const levelOf = (/** @type {FtRow} */ r) => (o.levels ? o.levels.get(r.id) : null);
    const best = (/** @type {FtRow} */ r) => Math.max(0, ...r.areas);
    const identText = (/** @type {FtRow} */ r) => { const x = (o.identsOf(r.id) || [])[0]; return x && x.formula ? x.formula : `m/z ${r.mz.toFixed(4)}`; };
    /** @type {any[]} */ const cols = [
      { key: "level", label: "Lvl", help: "ft.col.level", sticky: true, width: 60, sort: (/** @type {FtRow} */ r) => String(levelOf(r) || "z"), cell: (/** @type {FtRow} */ r) => (levelOf(r) ? TT.level(levelOf(r)) : ""), text: (/** @type {FtRow} */ r) => String(levelOf(r) || "") },
      { key: "identity", label: "Identity", help: "ft.col.identity", sticky: true, width: 200, sort: identText, cell: (/** @type {FtRow} */ r) => esc(identText(r)), text: identText },
      { key: "mz", label: "<i>m/z</i>", help: "ft.col.mz", num: true, sort: (/** @type {FtRow} */ r) => r.mz, cell: (/** @type {FtRow} */ r) => r.mz.toFixed(4), text: (/** @type {FtRow} */ r) => r.mz.toFixed(4) },
      { key: "rt", label: "RT", help: "ft.col.rt", num: true, sort: (/** @type {FtRow} */ r) => r.rt, cell: (/** @type {FtRow} */ r) => r.rt.toFixed(2), text: (/** @type {FtRow} */ r) => r.rt.toFixed(2) },
      { key: "trend", label: "Trend", help: "ft.col.trend", cell: (/** @type {FtRow} */ r) => (timed.length > 1 ? TT.spark(timed.map((/** @type {any} */ x) => r.areas[x.i]), "area") : ""), text: () => "" },
      { key: "cells", label: "Cells", help: "ft.col.cells", cell: (/** @type {FtRow} */ r) => TT.cells(r.states), text: (/** @type {FtRow} */ r) => r.states.join(" ") },
      { key: "max", label: "Max area", help: "ft.col.maxarea", num: true, sort: best, cell: (/** @type {FtRow} */ r) => sci(best(r)), text: (/** @type {FtRow} */ r) => String(best(r)) },
      { key: "id", label: "ID", help: "ft.col.id", num: true, off: true, sort: (/** @type {FtRow} */ r) => r.id, cell: (/** @type {FtRow} */ r) => String(r.id), text: (/** @type {FtRow} */ r) => String(r.id) },
      { key: "charge", label: "Charge", help: "ft.col.charge", num: true, off: true, sort: (/** @type {FtRow} */ r) => r.charge, cell: (/** @type {FtRow} */ r) => (r.charge == null ? "" : String(r.charge)), text: (/** @type {FtRow} */ r) => (r.charge == null ? "" : String(r.charge)) },
      { key: "detn", label: "Detected in", help: "ft.col.detn", num: true, off: true, sort: (/** @type {FtRow} */ r) => r.states.filter(s => s === "detected").length,
        cell: (/** @type {FtRow} */ r) => `${r.states.filter(s => s === "detected").length} of ${n}`, text: (/** @type {FtRow} */ r) => `${r.states.filter(s => s === "detected").length} of ${n}` },
      { key: "edges", label: "Peak edges", help: "ft.peak.borders", num: true, off: true, sort: (/** @type {FtRow} */ r) => r.lo, cell: (/** @type {FtRow} */ r) => (r.lo == null || r.hi == null ? "" : `${r.lo.toFixed(2)}–${r.hi.toFixed(2)}`), text: (/** @type {FtRow} */ r) => (r.lo == null || r.hi == null ? "" : `${r.lo.toFixed(2)}-${r.hi.toFixed(2)}`) },
    ];

    return { cols, identText, levelOf };
  }

  /** the decisions of the chemist for this table (TPDEC, 45_tabui_decisioni.js): the series of the run, the key of a row, its proposed level; null without TPDEC @param {any} o @param {(r: FtRow) => any} levelOf */
  function decisions(o, levelOf) {
    if (!G.TPDEC) return null;
    return { series: () => ({ files: o.names, formula: o.formula || "" }), key: (/** @type {FtRow} */ r) => G.TPDEC.rowKey(r.mz, r.rt, true), level: levelOf };
  }
  /** the open row: `riga/1` and `riga_tracce/1` once per row (cache) and the model TpRiga built from them */
  function rowModels(/** @type {any} */ o, /** @type {any} */ cols, /** @type {(r: FtRow) => string} */ identText, /** @type {(r: FtRow) => any} */ levelOf, /** @type {(t: string) => void} */ warn) {
    /** @type {Map<number, { row?: any, rowErr?: string, tr?: any, trErr?: string, rowP?: Promise<void>, trP?: Promise<void> }>} */ const cache = new Map();
    const entry = (/** @type {number} */ id) => { let e = cache.get(id); if (!e) cache.set(id, e = {}); return e; };
    const msg = (/** @type {any} */ e) => (o.say ? o.say(e) : String((e && e.message) || e));
    const loadRow = (/** @type {number} */ id) => { const e = entry(id); return e.rowP || (e.rowP = Promise.resolve(o.session.row(id)).then((/** @type {any} */ v) => { e.row = v; }, (/** @type {any} */ x) => { e.rowErr = msg(x); })); };
    const loadTr = (/** @type {number} */ id) => { const e = entry(id); return e.trP || (e.trP = Promise.resolve(o.session.traces(id, o.ppm())).then((/** @type {any} */ v) => { e.tr = v; }, (/** @type {any} */ x) => { e.trErr = msg(x); })); };
    /** the cells of the row (typed and timed) by file @param {FtRow} r @param {any} e */
    const sampleOf = (r, e, j) => { const c = e.row && e.row.cells && e.row.cells[j]; return { file: o.names[j], type: c ? c.type : (o.cs && o.cs[j] ? o.cs[j].tipo : undefined), time: c ? c.time : (o.cs && o.cs[j] ? o.cs[j].tempo_min : null), area: c ? c.area : r.areas[j] || null, state: c ? c.state : r.states[j],
      apexRt: c ? c.apex_rt : null, lo: null, hi: null }; };
    const dec = decisions(o, levelOf);
    const model = (/** @type {FtRow} */ r) => {
      const e = entry(r.id), w = e.row, t = e.tr, idents = o.identsOf(r.id) || [], ii = w && w.ident;
      const missing = /** @type {Record<string, string>} */ ({});
      if (!w) { const why = e.rowErr || "reading the row"; missing.quality = missing.identity = why; }
      if (!t) missing.xic = missing.spectrum = e.trErr || "reading the .mzidx files";
      const samples = r.areas.map((_, j) => sampleOf(r, e, j));
      const best = r.areas.reduce((b, a, j) => (a > (r.areas[b] || 0) ? j : b), 0);
      const sources = ii && ii.sources && ii.sources.length ? "Found by " + ii.sources.join(", ") : "";
      const m = {
        title: identText(r), level: levelOf(r) || undefined, words: sources, missing, samples, decide: dec ? { series: dec.series(), key: dec.key(r), level: dec.level(r) } : undefined,
        actions: [{ key: "data", label: "Show in Data", help: "ft.showdata", run: () => showInData(o.names[best], r.mz) }],
        xic: t ? t.xic.map((/** @type {any} */ x, /** @type {number} */ j) => ({ file: x.file, group: samples[j].type, time: samples[j].time, rt: x.rt, y: x.y })) : [],
        apexRt: r.rt, lo: w ? w.rt_lo : r.lo, hi: w ? w.rt_hi : r.hi,
        spectrum: t && t.ms1 && t.ms1.mz.length ? { kind: "ms1", mz: t.ms1.mz, y: t.ms1.y, iso: isoOf(t.ms1, r.mz) } : null,
        spectrum2: t && t.ms2 && t.ms2.mz.length ? { kind: "ms2", mz: t.ms2.mz, y: t.ms2.y } : null,
        line: w ? lineOf(w) : "",
        identity: w ? { neutral: ii ? ii.neutral : null, adduct: ii && ii.adduct ? ii.adduct : "", candidates: idents.map((/** @type {any} */ x) => ({ formula: x.formula, source: "hypothesis", level: x.level, proofs: (x.proofs || []).map((/** @type {any} */ p) => ({ ok: p.segno !== "contro", text: `${p.tipo}: ${p.punti} points` })) })) } : null,
        quality: w ? { shape: w.quality ? w.quality.shape : null, snr: w.quality ? w.quality.snr : null, fwhm: w.quality ? w.quality.fwhm : null, flags: w.quality ? w.quality.flags : [], isoM1: w.iso_m1,
          blank: presence(w, ["blank", "dark"]), t0: presence(w, ["t0"]) } : null,
      };
      if (t && !m.spectrum) missing.spectrum = "no MS1 scan at the apex";
      if (t && !t.ms2) missing.ms2 = "no MS2";
      return m;
    };
    /** the M+1 peak of the MS1 spectrum (the peak nearest to m/z + 1.00336 within 0.02) next to the peak of the row: what was observed, nothing expected */
    function isoOf(/** @type {{ mz: number[], y: number[] }} */ s, /** @type {number} */ mz) {
      const near = (/** @type {number} */ v) => s.mz.reduce((b, x, i) => (Math.abs(x - v) < Math.abs(s.mz[b] - v) ? i : b), 0);
      const a = near(mz), b = near(mz + 1.00336);
      return Math.abs(s.mz[a] - mz) < 0.02 && Math.abs(s.mz[b] - (mz + 1.00336)) < 0.02 && a !== b ? { obs: { mz: [s.mz[a], s.mz[b]], y: [s.y[a], s.y[b]] } } : undefined;
    }
    /** present in a group of controls: a flag read from the states of the cells (the ratios belong to the engine and are in the report) @param {any} w @param {string[]} types */
    function presence(w, types) {
      const cs = (w.cells || []).filter((/** @type {any} */ c) => types.includes(c.type));
      return cs.length ? { present: cs.some((/** @type {any} */ c) => c.state !== "absent") } : null;
    }
    function lineOf(/** @type {any} */ w) {
      const p = [`RT ${f2(w.rt, 2)} min${w.rt_lo != null && w.rt_hi != null ? ` (${f2(w.rt_lo, 2)}-${f2(w.rt_hi, 2)})` : ""}`];
      if (w.quality && w.quality.fwhm != null) p.push(`FWHM ${f2(w.quality.fwhm, 2)} min`);
      if (w.charge != null) p.push(`charge ${w.charge}`);
      if (w.ms2) p.push(`MS2 from ${w.ms2.file}${w.ms2.purity != null ? ` (purity ${f2(w.ms2.purity, 2)})` : ""}`);
      return p.join(" · ");
    }
    /** opens the XIC of the row in Data, with the file of the highest area when it is open there (the page of Data does the drawing) */
    function showInData(/** @type {string} */ name, /** @type {number} */ mz) {
      const w = G;
      try {
        const d = document.getElementById("unt"); if (d && /** @type {any} */ (d).open && /** @type {any} */ (d).close) /** @type {any} */ (d).close();
        if (typeof w.openXic === "function") {
          w.openXic(null, { mz, diretto: true });
          const base = (/** @type {string} */ s) => String(s || "").replace(/\.(mzml|mzidx|raw|d)$/i, "").toLowerCase(), sel = /** @type {HTMLSelectElement | null} */ (document.getElementById("xic-file"));
          if (sel) { const op = [...sel.options].find(x => base(x.textContent || "") === base(name)); if (op) { sel.value = op.value; sel.dispatchEvent(new Event("change", { bubbles: true })); } }
        }
      } catch (/** @type {any} */ e) { warn(String(e.message || e)); }
    }

    return { model, loadRow, loadTr };
  }

  /** mounts the step in `box`. o = { session (PIPEQC.open), names, gate, say, cs (campioni/2 or null), ppm(), levels (Map id → level, or null), identsOf(id) → [identity of evid/1] } */
  function mount(/** @type {HTMLElement} */ box, /** @type {any} */ o) {
    box.innerHTML = html();
    const q = (/** @type {string} */ s) => /** @type {any} */ (box.querySelector(s)), chips = /** @type {Set<string>} */ (new Set()), n = o.names.length;
    const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"]/g, c => /** @type {any} */ ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
    const hasBlank = !!(o.cs && o.cs.some((/** @type {any} */ c) => c.tipo === "blank" || c.tipo === "dark"));
    if (!hasBlank) { const b = q('[data-fc="blank"]'); b.disabled = true; b.setAttribute("aria-disabled", "true"); }
    const TT = G.TPTAB, { cols, identText, levelOf } = columns(o, n, esc);
    const { model, loadRow, loadTr } = rowModels(o, cols, identText, levelOf, t => { q("#ft-why").textContent = t; });
    const hook = G.TPRIGA.hook(model);
    /** @type {any} */ let T = null;
    const draw = (/** @type {FtRow} */ r, /** @type {() => void} */ again) => { loadRow(r.id).then(() => { if (T && T.openId === r.id) again(); }); };
    T = TT.mount(q("#ft-host"), {
      id: "ft-t", cols, rowId: (/** @type {FtRow} */ r) => r.id, name: (/** @type {FtRow} */ r) => identText(r), esc, label: "Feature table", columns: true, decide: decisions(o, levelOf) || undefined, exportName: "feature_table_view", virtual: 300, height: 520,
      tabs: hook.tabs,
      head: (/** @type {FtRow} */ r, /** @type {HTMLElement} */ box) => { hook.head(r, box); draw(r, () => hook.head(r, box)); },
      onOpen: (/** @type {FtRow} */ r, /** @type {HTMLElement} */ p, /** @type {string} */ tab) => {
        hook.onOpen(r, p, tab);
        const again = () => { if (T.openId === r.id && T.tab === tab && p.isConnected) hook.onOpen(r, p, tab); };
        loadRow(r.id).then(again); if (tab === "overview") loadTr(r.id).then(again);
      },
    });
    q("#ft-host").insertAdjacentHTML("beforeend", TT.legend(["detected", "filled", "targeted", "absent"]));

    let timer = 0, seq = 0;
    const spec = () => specOf(q, chips);
    const update = async () => {
      const mine = ++seq;
      try {
        const s = spec(), v = await o.session.view(s), c = v.conteggi, rej = (await o.session.qc(s)).scartate, rows = (await o.session.rows(s)).slice(1);
        if (mine !== seq) return;
        T.setRows(rows.map((/** @type {any[]} */ r) => rowOf(r, n)));
        q("#ft-n").textContent = `${c.tenute.toLocaleString("en")} of ${c.righe.toLocaleString("en")} rows`;
        q("#ft-why").textContent = rej.length ? "Rejected by each filter, in order: " + rej.map((/** @type {any} */ r) => `${G.PIPEQC.WHY[r.regola] || r.regola} (${r.righe.toLocaleString("en")})`).join(", ") : "";
      } catch (/** @type {any} */ e) { if (mine !== seq) return; q("#ft-n").textContent = ""; q("#ft-why").textContent = o.say ? o.say(e) : String(e.message || e); }
    };
    box.addEventListener("input", e => { if (!(/** @type {HTMLElement} */ (e.target)).closest("#ft-host")) { clearTimeout(timer); timer = window.setTimeout(update, 250); } });
    box.addEventListener("change", e => { const t = /** @type {HTMLElement} */ (e.target); if (t.closest("#ft-more") || t.id === "ft-norm") { clearTimeout(timer); timer = window.setTimeout(update, 0); } });
    box.querySelectorAll("[data-fc]").forEach(b => b.addEventListener("click", () => {
      const k = /** @type {string} */ (/** @type {HTMLElement} */ (b).dataset.fc), on = !chips.has(k); if (on) chips.add(k); else chips.delete(k);
      b.setAttribute("aria-pressed", String(on)); clearTimeout(timer); timer = window.setTimeout(update, 0);
    }));
    q("#ft-moreb").addEventListener("click", () => { const m = q("#ft-more"), open = m.hidden; m.hidden = !open; q("#ft-moreb").setAttribute("aria-expanded", String(open)); });
    update();
    return { update, spec, table: T };
  }

  G.PIPET = { mount, specOf, css };
})();
