"use strict";
// Open data: files for MassBank (record, text format 2.x) and SIRIUS 6 (.ms) from ONE high-resolution MS2 spectrum. Nothing is sent anywhere: the files are downloaded and the user
// decides if and where to upload them (NORMAN 2023 guidance, FAIR data). Never in low resolution (Principle 1: no record with annotations there).
// The compound fields (name, SMILES, InChIKey) are typed by the user (SMILES may come from «Disegno»); instrument, ionisation, collision energy, precursor and peaks come from the mzML.
// The pure functions (record, check, siriusMs, adductMass) are tested with node; the dialog is at the bottom. The untargeted tables of mzFinder use `siriusMs` through TP_Mine/js/30_untarget_fair.js.
const FAIR = (() => {
  const LICENSES = ["CC BY", "CC0", "CC BY-SA", "CC BY-NC", "CC BY-NC-SA"];            // the licences MassBank accepts for contributed records
  // m/z shift of the precursor (M + shift); electron mass included
  const ADDUCTS = { "[M+H]+": 1.007276, "[M+Na]+": 22.989218, "[M+NH4]+": 18.033823, "[M+K]+": 38.963158, "[M-H2O+H]+": -17.00274, "[M-H]-": -1.007276, "[M+Cl]-": 34.969402, "[M+HCOO]-": 44.998201, "[M+CH3COO]-": 59.013851 };
  const adductMass = a => (a in ADDUCTS ? ADDUCTS[a] : null);
  const adductSign = a => (a.endsWith("-") ? -1 : 1);
  const RX = { ik: /^[A-Z]{14}-[A-Z]{10}-[A-Z]$/, contrib: /^[A-Za-z0-9_]{1,32}$/, id: /^[A-Za-z0-9_]{1,64}$/ };
  const one = s => String(s == null ? "" : s).replace(/[\r\n]+/g, " ").trim();
  const num = (v, d) => (Number.isFinite(+v) ? (+v).toFixed(d) : "");

  /** instrument type string of MassBank (AC$INSTRUMENT_TYPE) from the analyzers named in the mzML; the user can change it */
  function instrumentType(analyzers) {
    const a = (analyzers || []).join(" ").toLowerCase(), has = x => a.includes(x);
    const q = has("quadrupole") || has("q-") || a === "q";
    if (has("orbitrap") || has("fourier")) return q ? "LC-ESI-QFT" : "LC-ESI-ITFT";
    if (has("time-of-flight") || has("tof")) return "LC-ESI-QTOF";
    if (has("ion trap")) return "LC-ESI-IT";
    return "LC-ESI-";
  }

  /** What is missing or doubtful before a record can be written: errors block the download, warnings do not. Each entry is {key, params} for I18N (`fair.err.*`). */
  function check(spec, c, o) {
    const errors = [], warnings = [], e = (key, params) => errors.push({ key, params: params || {} });
    if (spec.lr) { e("lr"); return { errors, warnings }; }
    const n = (spec.mz || []).filter((m, i) => spec.y[i] > 0).length;
    if (!n) e("peaks");
    if (!(spec.prec > 0)) e("prec");
    if (!one(c.name)) e("name");
    if (!one(c.smiles)) e("smiles");
    else if (!(c.mass > 0) || !one(c.formula)) e("smilesBad");
    if (!RX.ik.test(one(c.inchikey))) e("inchikey");
    if (!one(o.authors)) e("authors");
    if (!LICENSES.includes(o.license)) e("license");
    if (!RX.contrib.test(one(o.contributor))) e("contributor");
    if (!RX.id.test(one(o.id))) e("id");
    if (!one(o.instrument)) e("instrument");
    if (!/^LC-ESI-\w+$/.test(one(o.itype))) e("itype");
    const sh = adductMass(o.precType);
    if (sh == null) e("precType");
    else {
      if (spec.polarity != null && adductSign(o.precType) !== (spec.polarity < 0 ? -1 : 1)) e("polarity");
      if (c.mass > 0 && spec.prec > 0) { const ppm = (c.mass + sh - spec.prec) / spec.prec * 1e6; if (Math.abs(ppm) > 10) warnings.push({ key: "precMismatch", params: { ppm: ppm.toFixed(1) } }); }
    }
    if (spec.ce == null) warnings.push({ key: "noCe", params: {} });
    return { errors, warnings };
  }

  /** Record in the text format of MassBank 2.x. SPLASH is not written (MassBank adds it when the record is curated). Throws if check() finds errors. */
  function record(spec, c, o) {
    const ck = check(spec, c, o); if (ck.errors.length) throw Object.assign(new Error("fair: " + ck.errors.map(x => x.key).join(",")), { errors: ck.errors });
    const pk = []; for (let i = 0; i < spec.mz.length; i++) if (spec.y[i] > 0) pk.push([spec.mz[i], spec.y[i]]);
    pk.sort((a, b) => a[0] - b[0]);
    const top = Math.max(...pk.map(p => p[1]));
    const pos = adductSign(o.precType) > 0, d = o.date instanceof Date ? o.date : new Date();
    const date = `${d.getFullYear()}.${String(d.getMonth() + 1).padStart(2, "0")}.${String(d.getDate()).padStart(2, "0")}`;
    const ce = spec.ce == null ? null : (spec.nce ? `${spec.ce} % (nominal)` : `${spec.ce} eV`);
    const L = [`ACCESSION: MSBNK-${one(o.contributor)}-${one(o.id)}`,
      `RECORD_TITLE: ${one(c.name)}; ${one(o.itype)}; MS2${ce ? `; CE: ${ce}` : ""}; ${o.precType}`,
      `DATE: ${date}`, `AUTHORS: ${one(o.authors)}`, `LICENSE: ${o.license}`];
    L.push(`COMMENT: ${one(o.comment) || `Exported from ${one(spec.file) || "an mzML file"}${spec.sid != null ? `, scan ${spec.sid}` : ""} with mzLab`}`);
    L.push(`CH$NAME: ${one(c.name)}`, `CH$COMPOUND_CLASS: ${one(c.cls) || "N/A"}`, `CH$FORMULA: ${one(c.formula)}`, `CH$EXACT_MASS: ${num(c.mass, 6)}`, `CH$SMILES: ${one(c.smiles)}`,
      `CH$IUPAC: ${one(c.inchi) || "N/A"}`, `CH$LINK: INCHIKEY ${one(c.inchikey)}`);
    L.push(`AC$INSTRUMENT: ${one(o.instrument)}`, `AC$INSTRUMENT_TYPE: ${one(o.itype)}`, "AC$MASS_SPECTROMETRY: MS_TYPE MS2", `AC$MASS_SPECTROMETRY: ION_MODE ${pos ? "POSITIVE" : "NEGATIVE"}`);
    if (ce) L.push(`AC$MASS_SPECTROMETRY: COLLISION_ENERGY ${ce}`);
    if (spec.act) L.push(`AC$MASS_SPECTROMETRY: FRAGMENTATION_MODE ${String(spec.act).toUpperCase()}`);
    L.push("AC$MASS_SPECTROMETRY: IONIZATION ESI");
    if (spec.rt != null) L.push(`AC$CHROMATOGRAPHY: RETENTION_TIME ${num(spec.rt, 3)} min`);
    L.push(`MS$FOCUSED_ION: PRECURSOR_M/Z ${num(spec.prec, 4)}`, `MS$FOCUSED_ION: PRECURSOR_TYPE ${o.precType}`, `PK$NUM_PEAK: ${pk.length}`, "PK$PEAK: m/z int. rel.int.");
    for (const [m, y] of pk) L.push(`  ${num(m, 4)} ${num(y, 1)} ${Math.max(1, Math.round(999 * y / top))}`);
    L.push("//");
    return L.join("\n") + "\n";
  }

  /** SIRIUS 6 .ms: one compound per item {name, mass (parent m/z), ion, formula?, rt (min)?, ms1?:{mz,it}, ms2:[{mz,it,ce?}]}. Formula is optional; items without MS2 are skipped. */
  function siriusMs(items) {
    const L = ["# mzLab export for SIRIUS 6 (.ms). Spectra only; nothing here is an identification."];
    for (const it of items) {
      const ms2 = (it.ms2 || []).filter(s => s.mz && s.mz.length); if (!ms2.length || !(it.mass > 0)) continue;
      L.push("", `>compound ${one(it.name).replace(/\s+/g, "_") || "unknown"}`, `>parentmass ${num(it.mass, 5)}`, `>ionization ${it.ion || (it.pol < 0 ? "[M-H]-" : "[M+H]+")}`);
      if (one(it.formula)) L.push(`>formula ${one(it.formula)}`);
      if (it.rt != null) L.push(`>rt ${num(it.rt * 60, 1)}s`);
      if (it.ms1 && it.ms1.mz && it.ms1.mz.length) { L.push(">ms1"); for (let i = 0; i < it.ms1.mz.length; i++) if (it.ms1.it[i] > 0) L.push(`${num(it.ms1.mz[i], 5)} ${num(it.ms1.it[i], 1)}`); }
      for (const s of ms2) { L.push(s.ce != null ? `>collision ${s.ce}` : ">ms2"); for (let i = 0; i < s.mz.length; i++) if (s.it[i] > 0) L.push(`${num(s.mz[i], 5)} ${num(s.it[i], 1)}`); }
    }
    return L.join("\n") + "\n";
  }

  // ------------------------------------------------------------------ dialog (browser only)
  const T = (k, p) => I18N.t(k, p);
  const MSG = { lr: "fair.err.lr", peaks: "fair.err.peaks", prec: "fair.err.prec", name: "fair.err.name", smiles: "fair.err.smiles", smilesBad: "fair.err.smilesBad", inchikey: "fair.err.inchikey", authors: "fair.err.authors", license: "fair.err.license", contributor: "fair.err.contributor", id: "fair.err.id", instrument: "fair.err.instrument", itype: "fair.err.itype", precType: "fair.err.precType", polarity: "fair.err.polarity", precMismatch: "fair.warn.precMismatch", noCe: "fair.warn.noCe" };   // literal keys, so that tools/controlla_i18n.py sees them
  const EHt = s => EH(s == null ? "" : s);
  const mem = { get: () => { try { return JSON.parse(localStorage.getItem("qqq.fair") || "{}"); } catch (_) { return {}; } }, set: v => { try { localStorage.setItem("qqq.fair", JSON.stringify(v)); } catch (_) { /* storage blocked */ } } };

  async function ocl() {
    if (window.OCL) return window.OCL;
    try { const mod = await import("./vendor/openchemlib.js"); window.OCL = mod.default || mod; return window.OCL; } catch (_) { return null; }
  }
  /** formula and exact mass from a SMILES (OpenChemLib); null if it cannot be read */
  async function fromSmiles(smi) {
    const O = await ocl(); if (!O || !O.Molecule || !one(smi)) return null;
    try { const mf = O.Molecule.fromSmiles(one(smi)).getMolecularFormula(); return { formula: mf.formula, mass: mf.absoluteWeight }; } catch (_) { return null; }
  }

  /** the data of the MS2 spectrum of a panel: from workflow.js plus what the scan says about activation and collision energy; null in low resolution */
  function specOf(p) {
    const x = window.WORKFLOW && WORKFLOW.extractSpecData(p); if (!x || x.isLr) return null;
    const d0 = p._a.data[0].d, d = { ...d0, ...(d0.scan || {}) }, s = x.spec;
    return { mz: s.peaks.map(a => a[0]), y: s.peaks.map(a => a[1]), prec: s.prec, rt: s.rt, sid: x.sid, file: x.fn, polarity: s.polarity, ce: d.ce != null ? d.ce : null, nce: !!d.nce, act: d.act || "", parent: d.parent, k: (p._a.data[0].f || {}).k };
  }
  const available = p => { try { return !!specOf(p); } catch (_) { return false; } };         // the entries of the menu exist only for high-resolution MS2 spectra
  const safe = s => String(s || "spectrum").replace(/\.[^.]+$/, "").replace(/[^\w.-]+/g, "_").slice(0, 60);

  /** MS1 peaks (parent scan of the MS2, from the file) around the precursor for SIRIUS: the isotope pattern; {} if the file does not say which scan it was */
  async function ms1Of(spec) {
    if (spec.parent == null || spec.k == null || !window.J) return null;
    try { const r = await J(`api/scan?k=${spec.k}&sid=${spec.parent}` + (window.HR ? HR.q() : "")); const lo = spec.prec - 1.5, hi = spec.prec + 7, mz = [], it = [];
      r.mz.forEach((m, i) => { if (m >= lo && m <= hi && r.y[i] > 0) { mz.push(m); it.push(r.y[i]); } }); return mz.length ? { mz, it } : null; } catch (_) { return null; }
  }

  async function openDialog(p, kind) {
    const spec = specOf(p); if (!spec) return;
    const mem0 = mem.get(); let info = {};
    try { if (spec.k != null && window.J) info = await J(`api/fileinfo?k=${spec.k}`); } catch (_) { /* optional */ }
    const mb = kind === "massbank";
    let d = document.getElementById("fairdlg"); if (!d) { d = document.createElement("dialog"); d.id = "fairdlg"; document.body.appendChild(d); }
    const f = (id, label, val, extra = "") => `<label class="fr"><span>${T(label)}</span><input id="fa-${id}" value="${EHt(val == null ? "" : val)}" ${extra}></label>`;
    const sel = (id, label, opts, val) => `<label class="fr"><span>${T(label)}</span><select id="fa-${id}">${opts.map(v => `<option${v === val ? " selected" : ""}>${EHt(v)}</option>`).join("")}</select></label>`;
    const pol = spec.polarity < 0 ? "-" : "+";
    d.innerHTML = `<div class="top" style="display:flex;align-items:center"><h3 style="margin:0;flex:1">${T(mb ? "fair.mb.title" : "fair.si.title")}</h3><button class="x" id="fa-x" aria-label="${T("common.close")}">&times;</button></div>
      <div class="sm muted" style="margin:4px 0 8px">${T(mb ? "fair.mb.note" : "fair.si.note")}</div>
      <div class="sm muted">${EHt(spec.file)} · scan ${EHt(spec.sid)} · m/z ${num(spec.prec, 4)} · ${spec.ce != null ? "CE " + EHt(spec.ce) + (spec.nce ? " %" : " eV") : T("fair.noCe")}${spec.act ? " · " + EHt(spec.act) : ""}</div>
      <fieldset class="fa"><legend>${T("fair.compound")}</legend>${f("name", "fair.name", mem0.lastName || "")}
        <label class="fr"><span>SMILES</span><input id="fa-smiles" value="" autocomplete="off"><button type="button" id="fa-draw" ${window.TPDraw && TPDraw.ready && TPDraw.ready() ? "" : "hidden"}>${T("fair.fromDraw")}</button></label>
        ${mb ? f("ik", "fair.inchikey", "", 'placeholder="XXXXXXXXXXXXXX-YYYYYYYYYY-Z" spellcheck="false"') + f("inchi", "fair.inchi", "") : f("formula", "fair.formulaOpt", "")}
        <div class="sm muted" id="fa-calc" role="status"></div></fieldset>
      <fieldset class="fa"><legend>${T("fair.acq")}</legend>${sel("prec", "fair.precType", Object.keys(ADDUCTS), pol === "+" ? "[M+H]+" : "[M-H]-")}
        ${mb ? f("instr", "fair.instrument", info.instrument || "") + f("itype", "fair.itype", instrumentType(info.analyzers)) : ""}</fieldset>
      ${mb ? `<fieldset class="fa"><legend>${T("fair.record")}</legend>${f("contrib", "fair.contributor", mem0.contributor || "")}${f("id", "fair.recordId", "MZLAB" + String(spec.sid).replace(/\W+/g, "").padStart(5, "0"))}
        ${f("authors", "fair.authors", mem0.authors || "")}${sel("lic", "fair.license", LICENSES, mem0.license || "CC BY")}</fieldset>` : ""}
      <div id="fa-msg" class="sm" role="alert" style="margin:8px 0"></div>
      <div class="bar"><button type="button" id="fa-go" class="pri">${T("fair.download")}</button></div>`;
    const $ = id => d.querySelector("#fa-" + id), val = id => ($(id) ? $(id).value.trim() : "");
    $("x").onclick = () => d.close();
    if ($("draw")) $("draw").onclick = async () => { try { $("smiles").value = (await TPDraw.smiles()) || ""; $("smiles").dispatchEvent(new Event("change")); } catch (_) { /* drawing not ready */ } };
    let calc = null;
    $("smiles").onchange = async () => { calc = await fromSmiles(val("smiles")); $("calc").textContent = calc ? `${calc.formula} · ${calc.mass.toFixed(6)}` : (val("smiles") ? T("fair.err.smilesBad") : ""); if (calc && $("formula") && !val("formula")) $("formula").value = calc.formula; };
    $("go").onclick = async () => {
      const m = $("msg"); calc = await fromSmiles(val("smiles"));
      if (mb) {
        const c = { name: val("name"), smiles: val("smiles"), inchikey: val("ik").toUpperCase(), inchi: val("inchi"), formula: calc && calc.formula, mass: calc && calc.mass };
        const o = { contributor: val("contrib"), id: val("id"), authors: val("authors"), license: val("lic"), instrument: val("instr"), itype: val("itype"), precType: val("prec") };
        const ck = check(spec, c, o);
        m.innerHTML = ck.errors.concat(ck.warnings.map(w => ({ ...w, warn: true }))).map(x => `<div class="${x.warn ? "muted" : "err"}">${EHt(T(MSG[x.key], x.params))}</div>`).join("");
        if (ck.errors.length) return;
        mem.set({ ...mem.get(), contributor: o.contributor, authors: o.authors, license: o.license, lastName: c.name });
        dl(`${safe(spec.file)}_scan_${spec.sid}_massbank.txt`, record(spec, c, o), "text/plain;charset=utf-8");
        if (window.WORKFLOW) WORKFLOW.logOp("export_massbank_record", { file: spec.file, sid: spec.sid });
      } else {
        if (!(spec.prec > 0)) { m.textContent = T("fair.err.prec"); return; }
        const ms1 = await ms1Of(spec), ion = val("prec");
        dl(`${safe(spec.file)}_scan_${spec.sid}.ms`, siriusMs([{ name: val("name") || `${safe(spec.file)}_${spec.sid}`, mass: spec.prec, ion, formula: val("formula"), rt: spec.rt, ms1, ms2: [{ mz: spec.mz, it: spec.y, ce: spec.ce }] }]), "text/plain;charset=utf-8");
        if (window.WORKFLOW) WORKFLOW.logOp("export_sirius_ms", { file: spec.file, sid: spec.sid });
      }
      d.close();
    };
    d.showModal();
  }

  const massbank = p => openDialog(p, "massbank"), sirius = p => openDialog(p, "sirius");
  return { LICENSES, ADDUCTS, adductMass, instrumentType, check, record, siriusMs, massbank, sirius, specOf, available };
})();
if (typeof window !== "undefined") window.FAIR = FAIR;
if (typeof module !== "undefined") module.exports = FAIR;
