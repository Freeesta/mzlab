// @ts-check
// Flow (administrator mode): the step/1 descriptors as a registry, the data types of the ports and the rules of fit between them. Plain JS with no DOM:
// the same file runs in the page and under node (tests/test_flow_grafico.py). Descriptors: docs/schemi/steps-1.json, steps-attributio-1.json, steps-tabui-1.json.
/** @typedef {{name: string, type: string, many: boolean, optional: boolean, title: string, accepts: string[], requires: any}} FlowPort */
(() => {
  "use strict";
  const g = /** @type {any} */ (globalThis);
  /** @type {any} */
  const F = g.FLOW = g.FLOW || {};

  // the 19 data types of the contract; family decides the shape of the port mark, the letter is the second cue after colour (Okabe-Ito, FLOW_GRAFICA.md 3.1)
  /** @type {Record<string, {family: string, letter: string}>} */
  F.TYPES = {
    raw: { family: "file", letter: "R" }, mzml: { family: "file", letter: "M" },
    mzidx: { family: "trace", letter: "I" }, xic: { family: "trace", letter: "X" },
    peaks: { family: "peak", letter: "P" }, spectra: { family: "peak", letter: "S" }, rtmap: { family: "peak", letter: "T" },
    table: { family: "table", letter: "B" }, annot: { family: "table", letter: "A" }, tp: { family: "table", letter: "V" },
    library: { family: "table", letter: "L" }, candidates: { family: "table", letter: "K" }, formulas: { family: "table", letter: "F" },
    structure: { family: "chem", letter: "C" }, transform: { family: "chem", letter: "H" },
    samples: { family: "end", letter: "G" }, qc: { family: "end", letter: "Q" }, report: { family: "end", letter: "O" }, comparison: { family: "end", letter: "D" },
  };
  /** the types a user can declare as an input file of the plan */
  F.INPUT_TYPES = ["raw", "mzml", "mzidx", "spectra", "library", "structure", "samples", "table"];
  // the family of a step is the first word of its id; the palette groups by package, then by family
  F.PACKAGES = [["core", "Core"], ["attributio", "Attributio"], ["tabui", "Tabui"]];

  /** @param {Record<string, any> | undefined} o @returns {FlowPort[]} */
  F.ports = o => Object.entries(o || {}).map(([name, p]) => ({
    name, type: p.type, many: !!p.many, optional: !!p.optional, title: p.title || name, accepts: p.accepts || [], requires: p.requires || {},
  }));

  /** @param {any[][]} lists step/1 arrays (any package order) @returns {{steps: any[], byId: Map<string, any>}} */
  F.registry = lists => {
    const steps = lists.flat();
    return { steps, byId: new Map(steps.map(s => [s.id, s])) };
  };
  /** the presets of a step (an object by id in the descriptor) as a list @param {any} step @returns {any[]} */
  F.presetList = step => Object.entries(step.presets || {}).map(([id, p]) => ({ id, ...(/** @type {any} */ (p)) }));
  /** the family of a step: «features» for «features.detect» */
  F.family = (/** @type {string} */ id) => id.split(".")[0];

  /** the shortest chain of conversion steps from one type to another (empty = same type), null when none; mirrors flow::conversion_path
   * @param {{steps: any[]}} reg @param {string} from @param {string} to */
  F.conversionPath = (reg, from, to) => {
    /** @type {[string, any[]][]} */ const seen = [[from, []]];
    for (let at = 0; at < seen.length; at++) {
      const [ty, path] = seen[at];
      if (ty === to) return path;
      for (const s of reg.steps) if (s.converts && s.converts.from === ty && !seen.some(x => x[0] === s.converts.to)) seen.push([s.converts.to, [...path, s]]);
    }
    return null;
  };
  /** does a datum of type `ty` fit the port directly, through `accepts`, or only through a registered conversion («adapt»); else «no»
   * @param {{steps: any[]}} reg @param {string} ty @param {FlowPort} port @returns {"ok" | "adapt" | "no"} */
  F.fit = (reg, ty, port) => ty === port.type || port.accepts.includes(ty) ? "ok" : F.conversionPath(reg, ty, port.type) ? "adapt" : "no";

  /** the algorithm and preset an instance means (flow::canon::effective); `error` names what is unknown
   * @param {any} inst {algorithm, preset} @param {any} step @returns {{algorithm: any, preset: any, error?: {key: string, name: string, value: string}}} */
  F.effective = (inst, step) => {
    const presets = F.presetList(step), algs = step.algorithms || [];
    const presetId = inst.preset ?? step.default_preset ?? null;
    const preset = presetId == null ? null : presets.find((/** @type {any} */ p) => p.id === presetId);
    if (presetId != null && !preset) return { algorithm: null, preset: null, error: { key: "err.flow.preset.unknown", name: "preset", value: presetId } };
    const algId = inst.algorithm ?? (preset && preset.algorithm) ?? step.default_algorithm ?? null;
    const algorithm = algId == null ? null : algs.find((/** @type {any} */ a) => a.id === algId);
    if (algId != null && !algorithm) return { algorithm: null, preset: null, error: { key: "err.flow.algorithm.unknown", name: "algorithm", value: algId } };
    const keep = preset && (inst.preset != null || !preset.algorithm || (algorithm && preset.algorithm === algorithm.id));
    return { algorithm, preset: keep ? preset : null };
  };
  /** the parameters an instance can be given: those of the step and those of its algorithm, as [name, schema]
   * @param {any} step @param {any} algorithm @returns {[string, any][]} */
  F.declared = (step, algorithm) => [...Object.entries(step.params.properties || {}), ...Object.entries(algorithm && algorithm.params ? algorithm.params.properties || {} : {})];
})();
