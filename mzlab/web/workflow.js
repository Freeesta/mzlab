"use strict";
// Reproducibility (.mzworkflow) and MGF / MSP spectral export.
//
// .mzworkflow (JSON, format "mzworkflow/1"):
//   - SHA-256 of each loaded file
//   - program version and commit
//   - interface language
//   - all parameters: ppm, decimals, peak detection, corrections, filters, library thresholds, contaminant exclusions
//   - operations sequence with UTC timestamp
// MGF and MSP exports:
//   - from spectrum context menu (MS2)
//   - from «Identifica tutte le MS2» (HR)
// In LR, only spectra with precursor, charge, RT, file name (no candidate annotations or names: principle 1).
const WORKFLOW = (() => {
  const FORMAT = "mzworkflow/1";

  // Operation log (saved in notebook NB.ops with UTC timestamp)
  function logOp(op, details = {}) {
    try {
      if (typeof NB === "undefined") return;
      NB.ops = NB.ops || [];
      NB.ops.push({ op, t: new Date().toISOString(), ...details });
      if (NB.ops.length > 500) NB.ops.shift();
      if (typeof nbSave === "function") nbSave();
    } catch (_) {}
  }

  // ---------------------------------------------------------------- .mzworkflow creation & export
  function buildWorkflow() {
    const files = (typeof E !== "undefined" && E.files ? E.files : [])
      .filter(f => !f.gone)
      .map(f => ({
        nome: f.file || "",
        sha256: f.sha256 || "",
        dimensione: f.size || 0,
        tipo: f.type || "sample",
        etichetta: f.label || "",
        tempo: f.time ?? null,
      }));

    const soglieLib = window.LIVELLI ? LIVELLI.exportState() : null;       // profile and thresholds of the confidence levels (livelli.js)

    const parameters = {
      ppm: typeof UIP !== "undefined" && UIP.hrPpm != null ? UIP.hrPpm : 5,
      dec: typeof UIP !== "undefined" && UIP.hrDec != null ? UIP.hrDec : 4,
      tema: typeof UIP !== "undefined" && UIP.theme ? UIP.theme : "auto",
      tavolozza: typeof UIP !== "undefined" && UIP.pal ? UIP.pal : "time",
      fusione_isotopi: typeof UIP !== "undefined" ? UIP.merge !== false : true,
      mostra_tutti: typeof UIP !== "undefined" ? !!UIP.tog : false,
      picchi: typeof PK !== "undefined" ? { ...PK } : {},
      soglie_libreria: soglieLib,
      contaminanti_esclusi: window.LISTE ? LISTE.exclExport() : null,
    };

    const panels = (typeof E !== "undefined" && E.panels ? E.panels : []).map(p => ({
      type: p.type,
      tab: p.tab,
      title: p.title,
      kind: p.kind,
      smooth: p.smooth,
      level: p.level,
      prec: p.prec,
      filt: p.filt,
      snip: p.snip,
      snipw: p.snipw,
    }));

    const ops = (typeof NB !== "undefined" && NB.ops && NB.ops.length)
      ? NB.ops.slice()
      : [{ op: "session_snapshot", t: new Date().toISOString() }];

    return {
      formato: FORMAT,
      programma: {
        nome: window.APP_NAME || "",
        versione: window.MZLAB_VERSION || "0.1.0",
        commit: window.MZLAB_COMMIT || "dev",
      },
      data_creazione: new Date().toISOString(),
      lingua: (typeof I18N !== "undefined" && I18N.lang) || "it",
      file: files,
      parametri: parameters,
      pannelli: panels,
      operazioni: ops,
    };
  }

  function exportWorkflow() {
    if (typeof E === "undefined" || !E.files || !E.files.filter(f => !f.gone).length) {
      if (typeof info === "function") info(I18N.t("err.workflow.noFiles"));
      else alert(I18N.t("err.workflow.noFiles"));
      return;
    }
    const wf = buildWorkflow();
    const text = JSON.stringify(wf, null, 2);
    const first = E.files.find(f => !f.gone);
    const base = first ? (first.label || first.file).replace(/[^\w.-]+/g, "_") : "sessione";
    const filename = `${base}_${new Date().toISOString().slice(0, 10)}.mzworkflow`;
    dl(filename, text, "application/json");
    logOp("export_workflow", { filename });
  }

  // ---------------------------------------------------------------- .mzworkflow open & apply
  function openWorkflow() {
    let inp = document.getElementById("wf-open-input");
    if (!inp) {
      inp = document.createElement("input");
      inp.type = "file";
      inp.id = "wf-open-input";
      inp.accept = ".mzworkflow,.json";
      inp.style.display = "none";
      document.body.appendChild(inp);
    }
    inp.value = "";
    inp.onchange = async () => {
      const file = inp.files && inp.files[0];
      if (!file) return;
      try {
        const text = await file.text();
        const wf = JSON.parse(text);
        await applyWorkflow(wf, file.name);
      } catch (err) {
        if (typeof info === "function") info(I18N.t("workflow.err.invalidFormat") + " " + (err && err.message || ""));
        else alert(I18N.t("workflow.err.invalidFormat"));
      }
    };
    inp.click();
  }

  async function applyWorkflow(wf, wfFileName = "") {
    if (!wf || wf.formato !== FORMAT) {
      const msg = I18N.t("workflow.err.invalidFormat");
      if (typeof info === "function") info(msg); else alert(msg);
      return false;
    }

    const neededFiles = wf.file || [];
    const openFiles = (typeof E !== "undefined" && E.files ? E.files : []).filter(f => !f.gone);
    const openByName = new Map();
    openFiles.forEach(f => {
      openByName.set(f.file, f);
      if (f.label) openByName.set(f.label, f);
    });

    // Check for missing files
    const missing = neededFiles.filter(nf => !openByName.has(nf.nome));
    if (missing.length && !openFiles.length) {
      // Prompt user to load files first
      const msg = I18N.t("workflow.needFiles") + "\n" + missing.map(m => "• " + m.nome).join("\n");
      if (typeof info === "function") info(msg); else alert(msg);
    }

    // Check SHA-256 for open matching files
    for (const nf of neededFiles) {
      const of = openByName.get(nf.nome);
      if (of && of.sha256 && nf.sha256) {
        const act = of.sha256.trim().toLowerCase();
        const exp = nf.sha256.trim().toLowerCase();
        if (act !== exp) {
          const warnHtml = I18N.t("workflow.warn.shaMismatch", {
            file: of.file,
            expected: exp.slice(0, 10),
            actual: act.slice(0, 10),
          });
          if (typeof yesno === "function") {
            const proceed = await yesno(warnHtml);
            if (!proceed) return false;
          }
        }
      }
    }

    // Apply parameters
    const p = wf.parametri || {};
    if (p.picchi && typeof PK !== "undefined") {
      Object.assign(PK, p.picchi);
      if (typeof pkSave === "function") pkSave();
    }
    if (p.soglie_libreria && window.LIVELLI) LIVELLI.importState(p.soglie_libreria);
    if (typeof UIP !== "undefined") {
      if (p.ppm != null) UIP.hrPpm = p.ppm;
      if (p.dec != null) UIP.hrDec = p.dec;
      if (p.tema) UIP.theme = p.tema;
      if (p.tavolozza) UIP.pal = p.tavolozza;
      if (typeof p.fusione_isotopi === "boolean") UIP.merge = p.fusione_isotopi;
      if (typeof p.mostra_tutti === "boolean") UIP.tog = p.mostra_tutti;
      if (typeof uipSave === "function") uipSave();
      if (typeof uipApply === "function") uipApply();
    }
    if (p.contaminanti_esclusi && window.LISTE && typeof LISTE.exclImport === "function") {
      LISTE.exclImport(p.contaminanti_esclusi);
    }
    if (wf.operazioni && typeof NB !== "undefined") {
      NB.ops = wf.operazioni.slice();
    }

    logOp("workflow_applied", { workflow: wfFileName });
    if (typeof nearMsg === "function") nearMsg(I18N.t("workflow.loaded.success"));
    else if (typeof info === "function") info(I18N.t("workflow.loaded.success"));
    return true;
  }

  // ---------------------------------------------------------------- MGF & MSP formatting
  function formatMgf(spectra, isLr = false) {
    const blocks = [];
    for (const s of spectra) {
      const lines = ["BEGIN IONS"];
      const sid = s.sid != null ? s.sid : (s.scan != null ? s.scan : "");
      const rt = floatVal(s.rt, 0.0);
      const fn = s.file || "";
      let title = `scan=${sid} rt=${rt.toFixed(4)} file=${fn}`;
      if (!isLr && s.title_extra) title += " " + s.title_extra;
      lines.push(`TITLE=${title}`);
      lines.push(`RTINSECONDS=${(rt * 60).toFixed(2)}`);
      if (s.prec != null) lines.push(`PEPMASS=${s.prec}`);
      const chg = Math.abs(parseInt(s.charge || 1, 10) || 1);
      const pol = parseInt(s.polarity || 1, 10);
      const sign = pol >= 0 ? "+" : "-";
      lines.push(`CHARGE=${chg}${sign}`);
      const peaks = s.peaks || [];
      for (const pk of peaks) {
        const mz = floatVal(pk[0], 0.0), inten = floatVal(pk[1], 0.0);
        lines.push(`${mz.toFixed(5)} ${inten.toFixed(1)}`);
      }
      lines.push("END IONS");
      blocks.push(lines.join("\n"));
    }
    return blocks.join("\n\n") + (blocks.length ? "\n" : "");
  }

  function formatMsp(spectra, isLr = false) {
    const blocks = [];
    for (const s of spectra) {
      const lines = [];
      const sid = s.sid != null ? s.sid : (s.scan != null ? s.scan : "");
      const fn = s.file || "";
      let name = `${fn}_scan_${sid}`;
      if (!isLr && s.name_extra) name += `_${s.name_extra}`;
      lines.push(`NAME: ${name}`);
      if (s.prec != null) lines.push(`PRECURSORMZ: ${s.prec}`);
      const pol = parseInt(s.polarity || 1, 10);
      lines.push(`PRECURSORTYPE: ${pol >= 0 ? "[M+H]+" : "[M-H]-"}`);
      lines.push(`IONMODE: ${pol >= 0 ? "Positive" : "Negative"}`);
      const rt = floatVal(s.rt, 0.0);
      lines.push(`RETENTIONTIME: ${rt.toFixed(4)}`);
      const peaks = s.peaks || [];
      lines.push(`Num Peaks: ${peaks.length}`);
      for (const pk of peaks) {
        const mz = floatVal(pk[0], 0.0), inten = floatVal(pk[1], 0.0);
        lines.push(`${mz.toFixed(5)} ${inten.toFixed(1)}`);
      }
      blocks.push(lines.join("\n"));
    }
    return blocks.join("\n\n") + (blocks.length ? "\n" : "");
  }

  function floatVal(v, def = 0.0) {
    const n = parseFloat(v);
    return isNaN(n) ? def : n;
  }

  // ---------------------------------------------------------------- Spectrum panel export (single MS2)
  function extractSpecData(p) {
    if (!p || !p._a || !p._a.data || !p._a.data.length) return null;
    const item = p._a.data[0];
    const d = item.d;
    if (!d || !d.mz) return null;
    const f = item.f || (typeof E !== "undefined" && E.files ? E.files[p.k] : null);
    const fn = f ? (f.label || f.file || "") : "";
    const isHr = f && typeof HR !== "undefined" ? HR.isHr(f, 2) : false;
    const isLr = !isHr;

    const peaks = [];
    for (let i = 0; i < d.mz.length; i++) {
      if (d.y[i] > 0) peaks.push([d.mz[i], d.y[i]]);
    }
    const sid = d.sid != null ? d.sid : (p.sid != null ? p.sid : (p.si != null ? p.si : 0));
    const rt = d.rt != null ? d.rt : (p.r0 != null ? (p.r0 + (p.r1 ?? p.r0)) / 2 : 0.0);
    const prec = d.prec != null ? d.prec : (p.prec != null ? p.prec : null);
    const pol = f && f.polarity === "negative" ? -1 : 1;

    return {
      spec: { sid, rt, prec, charge: 1, polarity: pol, file: fn, peaks },
      isLr,
      fn,
      sid,
    };
  }

  function exportSpectrumMgf(p) {
    const data = extractSpecData(p);
    if (!data) return;
    const text = formatMgf([data.spec], data.isLr);
    const safeName = (data.fn || "spettro").replace(/[^\w.-]+/g, "_");
    dl(`${safeName}_scan_${data.sid}.mgf`, text, "text/plain");
    logOp("export_mgf_spectrum", { file: data.fn, sid: data.sid });
  }

  function exportSpectrumMsp(p) {
    const data = extractSpecData(p);
    if (!data) return;
    const text = formatMsp([data.spec], data.isLr);
    const safeName = (data.fn || "spettro").replace(/[^\w.-]+/g, "_");
    dl(`${safeName}_scan_${data.sid}.msp`, text, "text/plain");
    logOp("export_msp_spectrum", { file: data.fn, sid: data.sid });
  }

  function exportQueriesMgf(queries, f) {
    if (!queries || !queries.length) return;
    const isHr = f && typeof HR !== "undefined" ? HR.isHr(f, 2) : true;
    const isLr = !isHr;
    const fn = f ? (f.label || f.file || "file") : "file";
    const spectra = queries.map(q => ({
      sid: q.key,
      rt: q.rt,
      prec: q.prec,
      charge: 1,
      polarity: q.pol || (f && f.polarity === "negative" ? -1 : 1),
      file: fn,
      peaks: q.peaks || [],
    }));
    const text = formatMgf(spectra, isLr);
    const safeName = fn.replace(/[^\w.-]+/g, "_");
    dl(`${safeName}_all_ms2.mgf`, text, "text/plain");
    logOp("export_all_ms2_mgf", { file: fn, n: spectra.length });
  }

  function exportQueriesMsp(queries, f) {
    if (!queries || !queries.length) return;
    const isHr = f && typeof HR !== "undefined" ? HR.isHr(f, 2) : true;
    const isLr = !isHr;
    const fn = f ? (f.label || f.file || "file") : "file";
    const spectra = queries.map(q => ({
      sid: q.key,
      rt: q.rt,
      prec: q.prec,
      charge: 1,
      polarity: q.pol || (f && f.polarity === "negative" ? -1 : 1),
      file: fn,
      peaks: q.peaks || [],
    }));
    const text = formatMsp(spectra, isLr);
    const safeName = fn.replace(/[^\w.-]+/g, "_");
    dl(`${safeName}_all_ms2.msp`, text, "text/plain");
    logOp("export_all_ms2_msp", { file: fn, n: spectra.length });
  }

  return {
    FORMAT,
    logOp,
    buildWorkflow,
    export: exportWorkflow,
    open: openWorkflow,
    applyWorkflow,
    formatMgf,
    formatMsp,
    exportSpectrumMgf,
    exportSpectrumMsp,
    exportQueriesMgf,
    exportQueriesMsp,
  };
})();
window.WORKFLOW = WORKFLOW;
