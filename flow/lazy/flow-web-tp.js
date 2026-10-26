// @ts-check
// Flow (administrator mode): the steps of the transformation products that run in this browser. `tp.generate` calls the `hr` .wasm of the site in the page (the registry `ipotesi/1` of a parent
// is a few milliseconds of work) with the rules of the campaign (regole_tp_2.json) and, for a SMILES, the structure the `grafo` .wasm gives: the same text as the native `flow run`. English only.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  const STEPS = ["tp.generate", "tp.link"];
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
      const c = G.QTOOLS.ctx, man = await (await fetch(new URL("wasm-feat/manifest.json", c.base).href)).json();
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

  /** tp.generate: the registry of the hypotheses of the parent the plan holds, under the parameters of the step @param {any} S @param {any} inst @returns {Promise<{ name: string, text: string }>} */
  const generate = async (S, inst) => {
    try {
      const { hr, rules, text, structure } = await parentOf(S, inst);
      return { name: inst.id + ".ipotesi.json", text: hr.tp_generate(rules, text, structure, JSON.stringify({ ...F.defaults(inst, F.stepOf(S.reg, inst)), ...(inst.params || {}) })) };
    } catch (e) { throw asError(e); }
  };

  /** the inputs of tp.link@2 that are text: the parent with its structure, the samples and the hypotheses (made again by the tp.generate step they come from; a file of hypotheses stays with the computer, null)
   * @param {any} S @param {any} inst @param {(id: string) => any} instOf */
  const linkInputs = async (S, inst, instOf) => {
    const sref = (F.refs(inst, "samples")[0] || "").replace(/^file:/, ""), samples = S.samplesJson && S.samplesJson[sref];
    if (!samples) throw fail("err.flow.input.missing", { input: sref });
    const href = F.refs(inst, "hypotheses")[0] || "", gen = href.startsWith("file:") ? null : instOf(href.split(".")[0]);
    if (!gen || String(gen.step).split("@")[0] !== "tp.generate") return null;
    try { return { ...await parentOf(S, inst), samples, hypotheses: (await generate(S, gen)).text }; } catch (e) { throw asError(e); }
  };

  F.webTp = { STEPS, generate, linkInputs };
})();
