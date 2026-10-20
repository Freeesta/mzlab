// @ts-check
// Flow (administrator mode): the comparison of alternatives (`meta.compare@1`). A step with two or more available algorithms gets a «Compare algorithms» button that adds a
// comparison next to it: the same inputs, one variant per algorithm. The Rust checker has no descriptor for `meta.compare` yet, so the page checks it here and leaves it out
// of the parity check (F.withoutCompare); the runner refuses a plan with a comparison until the executor exists.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const isCompare = (/** @type {any} */ inst) => inst.step.startsWith("meta.compare@");
  const TOLERANCES = [["mz_tolerance_ppm", "m/z tolerance (ppm)", 5], ["mz_tolerance_da", "m/z tolerance (unit resolution, Da)", 0.3], ["rt_tolerance_min", "RT tolerance (min)", 0.2]];

  /** the step an instance means; a comparison takes the inputs of the step it wraps @param {any} reg @param {any} inst */
  F.stepOf = (reg, inst) => {
    if (!isCompare(inst)) return reg.byId.get(inst.step.split("@")[0]);
    const wrapped = reg.byId.get(String((inst.params || {}).step || "").split("@")[0]);
    return { id: "meta.compare", version: 1, package: "core", title: "Compare algorithms", in: wrapped ? wrapped.in : {}, params: {}, algorithms: [], presets: {},
      doc: "Runs the wrapped step once per variant on the same inputs and measures how the outputs differ.", out: { comparison: { type: "comparison", title: "Differences between variants" } } };
  };

  /** the available algorithms of a step @param {any} step */
  const available = step => (step.algorithms || []).filter((/** @type {any} */ a) => a.status === "available");

  /** @param {any} S @param {any} inst @param {any} step @param {(focus?: string) => void} done */
  F.compareButton = (S, inst, step, done) => {
    const algs = available(step);
    if (isCompare(inst) || algs.length < 2) return F.el("span");
    return F.btn("compare", `Compare algorithms of ${inst.id}`, () => {
      const cur = F.effective(inst, step).algorithm, first = cur ? cur.id : algs[0].id, other = algs.find((/** @type {any} */ a) => a.id !== first);
      const id = F.freshId(S.plan, "compare");
      S.plan.steps.splice(S.plan.steps.indexOf(inst) + 1, 0, { id, step: "meta.compare@1", in: JSON.parse(JSON.stringify(inst.in || {})),
        params: { step: `${step.id}@${step.version}`, variants: [{ label: "A", algorithm: first }, { label: "B", algorithm: other.id }] } });
      done(`fl-card-${id}`);
    });
  };

  /** the editor of the variants and tolerances of a comparison @param {any} S @param {any} inst @param {(focus?: string) => void} done */
  F.compareBody = (S, inst, done) => {
    const { el } = F, params = /** @type {any} */ (inst.params = inst.params || {}), real = S.reg.byId.get(String(params.step || "").split("@")[0]);
    const algs = real ? available(real) : [], variants = params.variants = params.variants || [];
    const rows = variants.map((/** @type {any} */ v, /** @type {number} */ k) => {
      const id = `fl-v-${inst.id}-${k}`;
      return el("div", { class: "fl-var" },
        el("input", { id: `${id}-label`, "data-fk": `${id}-label`, value: v.label || "", "aria-label": `Name of variant ${k + 1}`, onchange: (/** @type {any} */ e) => { v.label = e.target.value; done(`${id}-label`); } }),
        el("select", { id: `${id}-alg`, "data-fk": `${id}-alg`, "aria-label": `Algorithm of variant ${k + 1}`, onchange: (/** @type {any} */ e) => { v.algorithm = e.target.value; done(`${id}-alg`); } },
          ...algs.map((/** @type {any} */ a) => el("option", { value: a.id, text: a.id, selected: a.id === v.algorithm }))),
        F.btn("x", `Remove variant ${k + 1} of ${inst.id}`, () => { variants.splice(k, 1); done(); }));
    });
    const tol = TOLERANCES.map(([name, title, def]) => el("label", { class: "fl-tol" }, el("span", { text: title }),
      el("input", { type: "number", step: "any", min: 0, id: `fl-p-${inst.id}-${name}`, "data-fk": `fl-p-${inst.id}-${name}`, value: params[name] ?? def,
        onchange: (/** @type {any} */ e) => { const t = e.target.value; if (t === "" || +t === def) delete params[name]; else params[name] = +t; done(e.target.id); } })));
    const add = F.btn("plus", `Add a variant to ${inst.id}`, () => { const used = variants.map((/** @type {any} */ v) => v.algorithm); const a = algs.find((/** @type {any} */ x) => !used.includes(x.id)) || algs[0];
      variants.push({ label: String.fromCharCode(65 + variants.length), algorithm: a ? a.id : "" }); done(); }, "Variant");
    return el("div", { class: "fl-form" }, el("p", { class: "fl-note", text: `Compares ${params.step || "a step"} on the same inputs; the plan is not run until the comparison step exists in the runner.` }),
      ...rows, add, el("div", { class: "fl-tols" }, ...tol));
  };

  /** the plan without the comparisons: what the Rust checker can read @param {any} plan */
  F.withoutCompare = plan => ({ ...plan, steps: plan.steps.filter((/** @type {any} */ s) => !isCompare(s)) });

  /** the problems of the comparisons, in the shape of the checker's @param {any} plan @param {any} reg */
  F.validateExtra = (plan, reg) => {
    const out = /** @type {any[]} */ ([]);
    for (const inst of plan.steps.filter(isCompare)) {
      const p = /** @type {any} */ (inst.params || {}), [sid, ver] = String(p.step || "").split("@"), step = reg.byId.get(sid), add = (/** @type {string} */ key, /** @type {any} */ params) => out.push({ instance: inst.id, port: "", key, params });
      if (!step) { add("err.flow.step.unknown", { step: p.step || "" }); continue; }
      if (+ver !== step.version) add("err.flow.step.version", { step: sid, version: ver, available: step.version });
      const vs = p.variants || [];
      if (vs.length < 2) add("err.flow.compare.few", {});
      const seen = new Set();
      for (const v of vs) {
        const a = step.algorithms.find((/** @type {any} */ x) => x.id === v.algorithm);
        if (!a) add("err.flow.algorithm.unknown", { algorithm: v.algorithm }); else if (a.status !== "available") add("err.flow.algorithm.unavailable", { algorithm: v.algorithm });
        const key = JSON.stringify([v.algorithm, v.preset || "", v.params || {}]);
        if (seen.has(key)) add("err.flow.compare.sameVariant", {}); seen.add(key);
      }
    }
    return out;
  };
})();
