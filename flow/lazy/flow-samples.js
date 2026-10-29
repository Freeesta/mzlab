// @ts-check
// Flow (administrator mode): the sample list of a plan (input type `samples`, a campioni/3 document) kept in memory (S.samples[input name]) with its SHA-256 (the canonical form of the Rust of mzlab-core),
// which the plan records; the editor is the metadata grid (flow-meta-grid.js). Also the plan to run from the files open in Data.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  const TIMED = ["sample", "dark"];

  /** the document of the files open in the Data tab, or null when none is open */
  F.samplesFromData = () => {
    const files = typeof E !== "undefined" && Array.isArray(E.files) ? E.files : [];
    return files.length && G.CDOC ? G.CDOC.doc(files) : null;
  };

  /** what the files open in Data say of themselves: the polarity of all of them (±1, null for a mix or none) and the resolution of their MS1 scans ("hr", "lr", null for a mix or when unknown)
   * @returns {{polarity: number | null, resolution: string | null}} */
  F.fileNature = () => {
    const open = typeof E !== "undefined" && Array.isArray(E.files) ? E.files.filter((/** @type {any} */ f) => f && !f.gone && f.file && f.kind !== "infusion") : [];
    const one = (/** @type {any[]} */ a) => (a.length && a.every(x => x === a[0]) ? a[0] : null);
    const pols = open.map((/** @type {any} */ f) => (f.polarity === "negative" ? -1 : f.polarity === "positive" ? 1 : 0)).filter(Boolean);
    const res = open.map((/** @type {any} */ f) => (f.prof1 ? (f.prof1.hr ? "hr" : "lr") : null)).filter(Boolean);
    return { polarity: open.length && pols.length === open.length ? one(pols) : null, resolution: open.length && res.length === open.length ? one(res) : null };
  };

  /** the parameters of a step that the open files decide: `polarity` when they agree (and the step offers it), `nominal` for unit-resolution files @param {any} step @returns {Record<string, any>} */
  F.fromFiles = step => {
    const n = F.fileNature(), props = (step.params && step.params.properties) || {}, out = /** @type {Record<string, any>} */ ({});
    if (n.polarity != null && props.polarity && (props.polarity.enum || []).includes(n.polarity)) out.polarity = n.polarity;
    if (n.resolution === "lr" && props.nominal) out.nominal = true;
    return out;
  };

  /** a node made with files open starts with the preset that fits their resolution (written, so the form shows it and the user changes it) @param {any} inst @param {any} step */
  F.inheritPreset = (inst, step) => {
    const res = F.fileNature().resolution;
    if (!res || inst.preset != null) return;
    const fit = F.presetList(step).find((/** @type {any} */ p) => p.when && (p.when.resolution || []).includes(res));
    if (fit && fit.id !== step.default_preset) F.setPreset(inst, fit.id);
  };

  /** the instance of a plan to run: the preset of the files and the parameters they decide, written where the plan has not chosen and the descriptor default differs @param {any} inst @param {any} step */
  F.inheritNature = (inst, step) => {
    F.inheritPreset(inst, step);
    const plain = F.defaults(inst, step, true);
    for (const [name, v] of Object.entries(F.fromFiles(step))) if (!(inst.params && name in inst.params) && v !== plain[name]) { inst.params = inst.params || {}; inst.params[name] = v; }
  };

  /** the plan to run in the app and what the page tells about it: the files open in the Data tab become its scan inputs (the first placeholder input of a scan type is replaced by one input per file,
   * wired where it was; a plan that already names a Data file keeps it), `warns` lists the profile files met by a step whose chosen algorithm needs centroids. Never changes S.plan
   * @param {any} S @returns {{plan: any, names: string[], warns: string[]}} */
  F.runPlanFromData = S => {
    const plan = JSON.parse(JSON.stringify(S.plan)), warns = /** @type {string[]} */ ([]);
    for (const inst of plan.steps) { const step = F.stepOf(S.reg, inst); if (step) F.inheritNature(inst, step); }
    const open = typeof E !== "undefined" && Array.isArray(E.files) ? E.files.filter((/** @type {any} */ f) => f && !f.gone && f.file) : [];
    const held = [...new Set(open.map((/** @type {any} */ f) => String(f.file).split("#")[0]))];
    const scan = plan.inputs.filter((/** @type {any} */ i) => i.type === "raw" || i.type === "mzml");
    const holders = scan.filter((/** @type {any} */ i) => held.includes(i.name)), spare = scan.filter((/** @type {any} */ i) => !held.includes(i.name));
    /** @type {string[]} */ let names = holders.map((/** @type {any} */ i) => i.name);
    if (spare.length && held.length) {
      const first = spare[0], add = held.filter(n => !names.includes(n));
      for (const n of add) plan.inputs.push({ name: n, sha256: "", type: first.type });
      for (const s of plan.steps) for (const [port, v] of Object.entries(s.in || {})) {
        const refs = [].concat(/** @type {any} */ (v)), at = refs.indexOf("file:" + first.name);
        if (at < 0) continue;
        const many = (F.stepOf(S.reg, s) || { in: {} }).in[port]?.many, all = add.map(n => "file:" + n);
        const out = [...refs.slice(0, at), ...(many ? all : all.slice(0, 1)), ...refs.slice(at + 1)];
        if (!many && add.length > 1) warns.push(`${s.id}: input ${port} takes one file, so only ${add[0]} goes in.`);
        s.in[port] = many ? out : out[0];
      }
      names = names.concat(add);
      for (const i of spare) F.removeInput(plan, i.name);
    }
    const prof = (typeof ST !== "undefined" && Array.isArray(ST.files) ? ST.files : []).filter((/** @type {any} */ f) => names.includes(f.name) && f.spectrum_mode === "profile");
    if (prof.length) for (const s of plan.steps) {
      const step = F.stepOf(S.reg, s), alg = step && F.effective(s, step).algorithm, mode = alg && alg.applicability?.spectrum_mode;
      if (Array.isArray(mode) && !mode.includes("profile")) warns.push(`${prof.length} profile file${prof.length > 1 ? "s" : ""} (${prof.map((/** @type {any} */ f) => f.name).slice(0, 3).join(", ")}${prof.length > 3 ? "…" : ""}) reach ${s.id} with ${alg.id}, which needs centroided data. Add a spectra.centroid step before index.mzidx, or pick an algorithm that takes profile data.`);
    }
    return { plan, names, warns };
  };

  /** keeps `doc` as the content of the sample list `input` and its SHA-256 on the input; resolves when the hash is set @param {any} S @param {any} input @param {any} doc */
  F.samplesSet = async (S, input, doc) => {
    S.samples = S.samples || {}; S.samples[input.name] = doc;
    const c = G.CDOC ? await G.CDOC.canonical(doc) : null;
    if (c && /^[0-9a-f]{64}$/.test(c.sha256)) { input.sha256 = c.sha256; S.samplesJson = S.samplesJson || {}; S.samplesJson[input.name] = c.json; }
    else input.sha256 = "";
  };

  /** an empty entry of the table @param {number} i */
  const blank = F.sampleBlank = i => ({ composto: "", concentrazione: null, file: "", istd: false, note: "", ordine_iniezione: i + 1, replica: 1, serie: "", sha256: "", tempo: null, tempo_confermato: false, tempo_min: null, tipo: "sample", tipo_bianco: null, unita: null, unita_tempo: "min" });

  /** the editor of one sample list: the metadata grid of flow-meta-grid.js @param {any} S @param {any} input the input of the plan */
  F.samplesEditor = (S, input) => (F.metaGrid ? F.metaGrid(S, input) : F.el("p", { class: "fl-note", text: "Loading the sample table…" }));
})();
