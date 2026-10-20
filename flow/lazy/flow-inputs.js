// @ts-check
// Flow (administrator mode): the input files of a plan, their removal, the defaults of an instance and the check of what the goal still needs (continues flow-plan.js). No DOM.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  /** an input file of the plan; the name is made unique @param {any} plan @param {string} type */
  F.addInput = (plan, type) => {
    let name = "input1." + type, n = 1;
    while (plan.inputs.some((/** @type {any} */ f) => f.name === name)) name = `input${++n}.${type}`;
    plan.inputs.push({ name, sha256: "", type });
    return plan.inputs[plan.inputs.length - 1];
  };
  /** @param {any} plan @param {string} name */
  F.removeInput = (plan, name) => {
    plan.inputs = plan.inputs.filter((/** @type {any} */ f) => f.name !== name);
    for (const s of plan.steps) for (const [port, v] of Object.entries(s.in || {})) {
      const keep = [].concat(/** @type {any} */ (v)).filter((/** @type {any} */ r) => r !== "file:" + name);
      if (keep.length) s.in[port] = Array.isArray(v) ? keep : keep[0]; else delete s.in[port];
    }
  };
  /** the references of a port as a list @param {any} inst @param {string} port @returns {string[]} */
  F.refs = (inst, port) => [].concat(/** @type {any} */ ((inst.in || {})[port] ?? []));
  /** sets the references of a port (empty = unwired) @param {any} inst @param {FlowPort} port @param {string[]} refs */
  F.setPort = (inst, port, refs) => {
    inst.in = inst.in || {};
    if (!refs.length) delete inst.in[port.name]; else inst.in[port.name] = port.many ? refs : refs[0];
  };

  /** the values an instance resolves to without what the file says: step defaults, algorithm defaults, then the preset @param {any} inst @param {any} step */
  F.defaults = (inst, step) => {
    const e = F.effective(inst, step), out = /** @type {Record<string, any>} */ ({});
    for (const [name, p] of F.declared(step, e.algorithm)) if (p.default !== undefined) out[name] = p.default;
    if (e.preset) Object.assign(out, e.preset.params || {});
    return out;
  };
  /** sets a parameter; a value equal to the default (or null) removes it, so the file keeps only the differences @param {any} inst @param {any} step @param {string} name @param {any} value */
  F.setParam = (inst, step, name, value) => {
    const d = F.defaults(inst, step)[name];
    inst.params = inst.params || {};
    if (value == null || JSON.stringify(value) === JSON.stringify(d)) delete inst.params[name]; else inst.params[name] = value;
    if (!Object.keys(inst.params).length) delete inst.params;
  };
  /** switching the algorithm drops the parameters it does not declare @param {any} inst @param {any} step @param {string | null} algorithm */
  F.setAlgorithm = (inst, step, algorithm) => {
    if (algorithm == null) delete inst.algorithm; else inst.algorithm = algorithm;
    const known = new Set(F.declared(step, F.effective(inst, step).algorithm).map((/** @type {any} */ x) => x[0]));
    for (const k of Object.keys(inst.params || {})) if (!known.has(k)) F.setParam(inst, step, k, null);
  };
  /** @param {any} inst @param {string | null} preset */
  F.setPreset = (inst, preset) => { if (preset == null) delete inst.preset; else inst.preset = preset; };

  /** the plan in canonical form: the order of the run (ties in file order), keys in a fixed order, no empty fields. Positions and notes of the page are not in it
   * @param {any} plan @returns {any} */
  F.exportPlan = plan => {
    const order = F.planOrder(F.normalize(F.cleanRefs(plan))) || plan.steps.map((/** @type {any} */ _, /** @type {number} */ k) => k);
    const steps = order.map((/** @type {number} */ k) => {
      const s = plan.steps[k], o = /** @type {any} */ ({ id: s.id, step: s.step });
      if (s.label) o.label = s.label;
      o.in = s.in || {};
      for (const key of ["algorithm", "preset", "params"]) if (s[key] != null && !(key === "params" && !Object.keys(s.params).length)) o[key] = s[key];
      return o;
    });
    return { format: "mzflow/2", program: plan.program, inputs: plan.inputs.map((/** @type {any} */ f) => ({ name: f.name, sha256: f.sha256 || "", ...(f.type ? { type: f.type } : {}), ...(f.facets ? { facets: f.facets } : {}) })), steps };
  };
  /** a copy of the plan that the reader of flow-check.js can always read (references that are not text are dropped) @param {any} plan */
  F.cleanRefs = plan => ({ ...plan, steps: plan.steps.map((/** @type {any} */ s) => ({ ...s, in: Object.fromEntries(Object.entries(s.in || {}).filter(([, v]) => typeof v === "string" || Array.isArray(v))) })) });
  F.exportText = (/** @type {any} */ plan) => JSON.stringify(F.exportPlan(plan), null, 1) + "\n";

  /** reads a file; {plan} or {error: {key, params}} (the keys of the Rust reader). The page edits the object it gets @param {string} text */
  F.importText = text => {
    let v;
    try { v = JSON.parse(text); } catch (_) { return { error: { key: "err.flow.json", params: {} } }; }
    try { F.normalize(v); } catch (e) { const x = /** @type {any} */ (e); return { error: { key: x.key || "err.flow.json", params: x.params || {} } }; }
    return { plan: { format: "mzflow/2", program: v.program || F.newPlan().program, inputs: (v.inputs || []).map((/** @type {any} */ f) => ({ ...f, sha256: f.sha256 || "" })), steps: v.steps || [] } };
  };

  /** can the chain reach the identification (F.GOAL_STEPS)? `reached`: it has the step; `canReach`: every type it needs can be made from the plan's inputs and outputs;
   * `missing`: input types to add when not; `next`: steps to add now that lead there. @param {any} plan @param {any} reg */
  F.goal = (plan, reg) => {
    const avail = new Set(plan.inputs.map((/** @type {any} */ f) => f.type).filter(Boolean));
    for (const s of plan.steps) { const st = F.stepOfInstance(reg, s); if (st) for (const p of F.ports(st.out)) avail.add(p.type); }
    const runnable = (/** @type {any} */ st, /** @type {Set<string>} */ have) => F.ports(st.in).filter((/** @type {FlowPort} */ p) => !p.optional).every((/** @type {FlowPort} */ p) => [...have].some(t => F.fit(reg, /** @type {string} */ (t), p) !== "no"));
    const now = new Set(avail), closure = new Set(avail);
    for (let grew = true; grew;) {
      grew = false;
      for (const st of reg.steps) if (runnable(st, closure)) for (const p of F.ports(st.out)) if (!closure.has(p.type)) { closure.add(p.type); grew = true; }
    }
    const goals = F.GOAL_STEPS.map((/** @type {string} */ id) => reg.byId.get(id)).filter(Boolean);
    const reached = plan.steps.some((/** @type {any} */ s) => F.GOAL_STEPS.includes(s.step.split("@")[0]));
    const canReach = goals.some((/** @type {any} */ g) => runnable(g, closure));
    // the types the goal needs, and what makes them, back to the inputs; `missing` is the smallest set of input types that would let the chain reach it
    const need = new Set(), todo = goals.flatMap((/** @type {any} */ g) => F.ports(g.in).filter((/** @type {FlowPort} */ p) => !p.optional).map((/** @type {FlowPort} */ p) => p.type));
    while (todo.length) {
      const t = todo.pop();
      if (need.has(t)) continue;
      need.add(t);
      for (const st of reg.steps) if (F.ports(st.out).some((/** @type {FlowPort} */ p) => p.type === t)) todo.push(...F.ports(st.in).filter((/** @type {FlowPort} */ p) => !p.optional).map((/** @type {FlowPort} */ p) => p.type));
    }
    const reach = (/** @type {Set<string>} */ have) => {
      const c = new Set(have);
      for (let grew = true; grew;) {
        grew = false;
        for (const st of reg.steps) if (runnable(st, c)) for (const p of F.ports(st.out)) if (!c.has(p.type)) { c.add(p.type); grew = true; }
      }
      return goals.some((/** @type {any} */ g) => runnable(g, c));
    };
    const absent = F.INPUT_TYPES.filter((/** @type {string} */ t) => !avail.has(t));
    /** @type {string[]} */ let missing = [];
    for (let size = 1; size <= 3 && !canReach && !missing.length; size++) {
      const pick = (/** @type {string[]} */ from, /** @type {string[]} */ got) => got.length === size ? (reach(new Set([...avail, ...got])) ? got : null) : from.map((t, k) => pick(from.slice(k + 1), [...got, t])).find(Boolean) || null;
      missing = pick(absent, []) || [];
    }
    const have = (/** @type {string} */ id) => plan.steps.some((/** @type {any} */ s) => s.step.split("@")[0] === id);
    const next = reg.steps.filter((/** @type {any} */ st) => !have(st.id) && runnable(st, now) && F.ports(st.out).some((/** @type {FlowPort} */ p) => need.has(p.type) && !now.has(p.type))).map((/** @type {any} */ st) => st.id);
    return { reached, canReach, missing, next };
  };
})();
