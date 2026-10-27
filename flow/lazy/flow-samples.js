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

  /** the plan to run in the app and what the page tells about it: the files open in the Data tab become its scan inputs (the first placeholder input of a scan type is replaced by one input per file,
   * wired where it was; a plan that already names a Data file keeps it), `warns` lists the profile files met by a step whose chosen algorithm needs centroids. Never changes S.plan
   * @param {any} S @returns {{plan: any, names: string[], warns: string[]}} */
  F.runPlanFromData = S => {
    const plan = JSON.parse(JSON.stringify(S.plan)), warns = /** @type {string[]} */ ([]);
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
