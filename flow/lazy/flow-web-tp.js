// @ts-check
// Flow (administrator mode): the steps of the transformation products that run in this browser. `tp.generate` calls the `hr` .wasm of the site in the page (the registry `ipotesi/1` of a parent
// is a few milliseconds of work) with the rules of the campaign (regole_tp_2.json) and, for a SMILES, the structure the `grafo` .wasm gives: the same text as the native `flow run`. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  const STEPS = ["tp.generate", "tp.link", "tp.filter", "tp.position", "tp.network", "tp.review", "tp.dig_lr"];
  /** @type {Promise<any> | null} */ let loaded = null;
  const fail = (/** @type {string} */ key, /** @type {any} */ params = {}) => Object.assign(new Error(key), { key, params });

  /** a wasm-bindgen module of the manifest `wasm-feat/manifest.json`, checked against its SHA-256 @param {string} base @param {any} m */
  const open = async (base, m) => {
    const u = (/** @type {string} */ p) => new URL("wasm-feat/" + p, base).href, bytes = await (await fetch(u(m.wasm))).arrayBuffer();
    const hex = [...new Uint8Array(await crypto.subtle.digest("SHA-256", bytes))].map(x => x.toString(16).padStart(2, "0")).join("");
    if (m.sha256 && hex !== m.sha256) throw fail("error.featWasmHash");
    const mod = await import(/* webpackIgnore: true */ u(m.js)); await mod.default({ module_or_path: bytes });
    return mod;
  };

  /** the `proofs` and `grafo` modules and the rules, loaded once */
  const engine = async () => {
    if (loaded) return loaded;
    loaded = (async () => {
      const c = await F.tpCtx("regole_tp_2.json"), man = await (await fetch(new URL("wasm-feat/manifest.json", c.base).href)).json();
      const rules = c.files["regole_tp_2.json"];
      if (!rules || !man.proofs) throw fail("error.featFailed");
      return { hr: await open(c.base, man.proofs), grafo: man.grafo ? await open(c.base, man.grafo) : null, rules: typeof rules === "string" ? new TextEncoder().encode(rules) : rules };
    })().catch(e => { loaded = null; throw e; });
    return loaded;
  };

  /** an error of a .wasm ({error_key, params} as text) as the page's {key, params} @param {any} e */
  const asError = e => { let j = null; try { j = JSON.parse(String(e)); } catch (_) { /* not JSON */ } return j && j.error_key ? fail(j.error_key, j.params || {}) : e; };

  /** the parent the plan holds, as the .wasm reads it: its text and the structure text of its SMILES @param {any} S @param {any} inst */
  const parentOf = async (S, inst) => {
    const ref = (F.refs(inst, "parent")[0] || "").replace(/^file:/, ""), text = F.parentText(S, ref);
    if (!text) throw fail("err.flow.tp.parent", { input: ref });
    const e = await engine(), smiles = JSON.parse(e.hr.tp_parent(text)).smiles;
    return { ...e, text, structure: smiles ? (e.grafo ? e.grafo.analyse(smiles) : (() => { throw fail("error.featFailed"); })()) : "" };
  };

  /** the indexes of the series of the live session `x`, in the order of its items (the centroids of a profile file when the plan centroids) @param {any} x @returns {string[]} */
  const indexPaths = x => x.items.map((/** @type {any} */ it) => x.source(it)).filter(Boolean).map((/** @type {any} */ s) => s.path);

  /** the parameters of a tp.generate the plan leaves unwritten that the indexes decide (polarity, unit mass: `series_estimates` of the proofs .wasm, the rule of `flow run`)
   * @param {any} pool @param {any} inst @param {string[]} paths @returns {Promise<{ name: string, value: any, proof: any }[]>} */
  const estimates = async (pool, inst, paths) => (paths.length ? JSON.parse(await pool.seriesEstimates({ written: Object.keys(inst.params || {}), paths })) : []);

  /** tp.generate: the registry of the hypotheses of the parent the plan holds, under the parameters of the step and the ones the files decide (`est`)
   * @param {any} S @param {any} inst @param {{ name: string, value: any }[]} [est] @returns {Promise<{ name: string, text: string }>} */
  const generate = async (S, inst, est = []) => {
    try {
      const { hr, rules, text, structure } = await parentOf(S, inst), decided = Object.fromEntries(est.map(e => [e.name, e.value]));
      return { name: inst.id + ".ipotesi.json", text: hr.tp_generate(rules, text, structure, JSON.stringify({ ...F.defaults(inst, F.stepOf(S.reg, inst)), ...decided, ...(inst.params || {}) })) };
    } catch (e) { throw asError(e); }
  };

  /** the hypotheses of the tp.generate `gen` of the live session `x` as its job made them @param {any} x @param {any} pool @param {any} gen */
  const generateIn = async (x, pool, gen) => generate(x.S, gen, await estimates(pool, gen, indexPaths(x)));

  /** the inputs of tp.link@2 that are text: the parent with its structure, the samples and the hypotheses (made again by the tp.generate step they come from; a file of hypotheses stays with the computer, null)
   * @param {any} x the live session @param {any} pool @param {any} inst */
  const linkInputs = async (x, pool, inst) => {
    const S = x.S, sref = (F.refs(inst, "samples")[0] || "").replace(/^file:/, ""), samples = S.samplesJson && S.samplesJson[sref];
    if (!samples) throw fail("err.flow.input.missing", { input: sref });
    const href = F.refs(inst, "hypotheses")[0] || "", gen = href.startsWith("file:") ? null : x.inst(href.split(".")[0]);
    if (!gen || String(gen.step).split("@")[0] !== "tp.generate") return null;
    try { return { ...await parentOf(S, inst), samples, hypotheses: (await generateIn(x, pool, gen)).text }; } catch (e) { throw asError(e); }
  };

  /** the inputs of tp.dig_lr that are text: the parent with its structure and the samples (the table, the peaks and the indexes come from the files of the series) @param {any} S @param {any} inst */
  const digInputs = async (S, inst) => {
    const sref = (F.refs(inst, "samples")[0] || "").replace(/^file:/, ""), samples = S.samplesJson && S.samplesJson[sref];
    if (!samples) throw fail("err.flow.input.missing", { input: sref });
    try { return { ...await parentOf(S, inst), samples }; } catch (e) { throw asError(e); }
  };

  const NAMES = /** @type {Record<string, string>} */ ({ "tp.filter": ".filtri.json", "tp.position": ".sites.json", "tp.network": ".rete.json", "tp.review": ".review.json" });
  const step = (/** @type {any} */ i) => String(i && i.step).split("@")[0];

  /** the `.tp.mzfeat` of the tp.link job an input points to (null when it is not a tp.link of this plan or not made yet) @param {any} c the context of the executor @param {any} inst @param {string} port */
  const tpFile = async (c, inst, port) => {
    const up = c.x.inst(String(F.refs(inst, port)[0] || "").split(".")[0]), key = up && step(up) === "tp.link" && c.x.keys.get(c.x.jk({ instance: up.id, item: null }));
    return key && await c.p.fs.exists(c.dir + key + ".tp.mzfeat") ? c.p.fs.read(c.dir + key + ".tp.mzfeat") : null;
  };

  /** the text a tp.filter, tp.position, tp.network or tp.review job asks of the pool (the same Rust as `flow run`): its name and text, or null when an input stays with the computer
   * @param {{ x: any, p: any, dir: string }} c @param {any} inst */
  const post = async (c, inst) => {
    const S = c.x.S, kind = step(inst), tp = await tpFile(c, inst, "tp");
    if (!tp) return null;
    const params = JSON.stringify({ ...F.defaults(inst, F.stepOf(S.reg, inst)), ...(inst.params || {}) }), m = /** @type {any} */ ({ kind: kind.slice(3), tp, params });
    try {
      if (kind === "tp.filter") {
        const href = F.refs(inst, "hypotheses")[0] || "", gen = href.startsWith("file:") ? null : c.x.inst(href.split(".")[0]);
        if (!gen || step(gen) !== "tp.generate") return null;
        m.hypotheses = (await generateIn(c.x, c.p, gen)).text;
      } else if (kind === "tp.review") {
        const dref = (F.refs(inst, "decisions")[0] || "").replace(/^file:/, "");
        m.decisions = S.samplesJson && S.samplesJson[dref];
        if (!m.decisions) throw fail("err.flow.input.missing", { input: dref });
      } else if (kind === "tp.position") {
        const { hr, text, structure } = await parentOf(S, inst), pf = JSON.parse(hr.tp_parent(text));
        m.smiles = pf.smiles; m.formula = pf.formula || (structure && JSON.parse(structure).formula) || "";
      } else {
        const same = F.refs(inst, "table")[0] === F.refs(inst, "tp")[0], sref = (F.refs(inst, "samples")[0] || "").replace(/^file:/, "");
        m.table = same ? tp : await tpFile(c, inst, "table");
        if (!m.table) return null;
        const pr = JSON.parse(params); m.ppm = pr.ppm ?? 10; m.minFragments = pr.min_fragments ?? 0;
        m.samples = (sref && S.samplesJson && S.samplesJson[sref]) || "";
        const rules = G.QTOOLS.ctx.files["regole_tp_2.json"];
        m.rules = typeof rules === "string" ? new TextEncoder().encode(rules) : rules;
      }
      return { name: inst.id + NAMES[kind], text: await c.p.tpPost(m) };
    } catch (e) { throw asError(e); }
  };

  F.webTp = { STEPS, generate, estimates, indexPaths, linkInputs, digInputs, post };
})();
