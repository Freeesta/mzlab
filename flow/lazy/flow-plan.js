// @ts-check
// Flow (administrator mode): the plan as the page edits it (a plain mzflow/2 object): adding, wiring and removing steps, the canonical export, the import, and
// the goal check (can the chain reach the identification). No DOM. Rules of fit are those of flow-registry.js; the validity of the whole plan is flow-check.js.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  // v1 decision (FLOW_GRAFICA.md 13, easy to change): the goal of a chain is the step that grades an identification
  F.GOAL_STEPS = ["annot.level"];

  F.newPlan = () => ({ format: "mzflow/2", program: { name: "mzLab", version: String(/** @type {any} */ (globalThis).MZLAB_VERSION || "") }, inputs: [], steps: [] });
  /** the instance of a plan with that id @param {any} plan @param {string} id */
  F.instanceOf = (plan, id) => plan.steps.find((/** @type {any} */ s) => s.id === id);
  /** `det`, `det2`... for a step id: short, unique in the plan @param {any} plan @param {string} stepId */
  F.freshId = (plan, stepId) => {
    const parts = stepId.split("."), last = parts[parts.length - 1];
    const base = (F.TYPES[last] ? parts[0] : last).replace(/[^a-z0-9]/gi, "").slice(0, 8) || "step";
    let id = base;
    for (let n = 2; F.instanceOf(plan, id) || plan.inputs.some((/** @type {any} */ f) => f.name === id); n++) id = base + n;
    return id;
  };
  /** the ids of the instances that read, directly or not, from `id` @param {any} plan @param {string} id */
  const downstream = (plan, id) => {
    const out = new Set([id]);
    for (let grew = true; grew;) {
      grew = false;
      for (const s of plan.steps) if (!out.has(s.id) && Object.values(s.in || {}).flat().some((/** @type {any} */ r) => out.has(String(r).split(".")[0]) && !String(r).startsWith("file:"))) { out.add(s.id); grew = true; }
    }
    return out;
  };
  /** what can feed a port: the input files and the outputs of the other instances (not those that read from this one), each with how it fits
   * @param {any} plan @param {any} reg @param {string} id the instance @param {FlowPort} port @returns {{ref: string, label: string, type: string, many: boolean, fit: string}[]} */
  F.candidates = (plan, reg, id, port) => {
    const out = [], after = downstream(plan, id);
    for (const f of plan.inputs) if (f.type && F.TYPES[f.type]) out.push({ ref: "file:" + f.name, label: f.name, type: f.type, many: false, fit: F.fit(reg, f.type, port) });
    for (const s of plan.steps) {
      const step = reg.byId.get(s.step.split("@")[0]);
      if (!step || after.has(s.id)) continue;
      for (const p of F.ports(step.out)) out.push({ ref: s.id + "." + p.name, label: `${s.id}.${p.name}`, type: p.type, many: p.many, fit: F.fit(reg, p.type, port) });
    }
    return out.filter(c => c.fit !== "no");
  };
  const stepOfInstance = (/** @type {any} */ reg, /** @type {any} */ inst) => reg.byId.get(inst.step.split("@")[0]);
  /** wires each required input to the latest output that fits (a direct fit before a conversion); optional ports stay empty @param {any} plan @param {any} reg @param {any} inst */
  F.autoWire = (plan, reg, inst) => {
    const step = stepOfInstance(reg, inst);
    inst.in = inst.in || {};
    for (const port of F.ports(step.in).filter((/** @type {FlowPort} */ p) => !p.optional)) {
      const c = F.candidates(plan, reg, inst.id, port).filter((/** @type {any} */ x) => !(x.many && !port.many));
      const best = c.filter((/** @type {any} */ x) => x.fit === "ok").pop() || c.pop();
      // nothing in the plan can feed a file-like port: the plan gets the input it needs, so a first step is one click
      const made = !best && F.INPUT_TYPES.includes(port.type) ? "file:" + F.addInput(plan, port.type).name : null;
      if (best || made) { const ref = best ? best.ref : /** @type {string} */ (made); inst.in[port.name] = port.many ? [ref] : ref; }
    }
  };
  /** adds a step at the end, wired; returns its instance @param {any} plan @param {any} reg @param {string} stepId */
  F.addStep = (plan, reg, stepId) => {
    const step = reg.byId.get(stepId), inst = { id: F.freshId(plan, stepId), step: `${stepId}@${step.version}`, in: {} };
    plan.steps.push(inst);
    F.autoWire(plan, reg, inst);
    return inst;
  };
  /** removes an instance and every reference to it @param {any} plan @param {string} id */
  F.removeStep = (plan, id) => {
    plan.steps = plan.steps.filter((/** @type {any} */ s) => s.id !== id);
    for (const s of plan.steps) for (const [port, v] of Object.entries(s.in || {})) {
      const keep = [].concat(/** @type {any} */ (v)).filter((/** @type {any} */ r) => r.startsWith("file:") || r.split(".")[0] !== id);
      if (keep.length) s.in[port] = Array.isArray(v) ? keep : keep[0]; else delete s.in[port];
    }
  };
  /** moves an instance one place up (-1) or down (+1) in the list @param {any} plan @param {string} id @param {number} by */
  F.moveStep = (plan, id, by) => {
    const k = plan.steps.findIndex((/** @type {any} */ s) => s.id === id), j = k + by;
    if (k < 0 || j < 0 || j >= plan.steps.length) return false;
    [plan.steps[k], plan.steps[j]] = [plan.steps[j], plan.steps[k]];
    return true;
  };
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
    for (const s of plan.steps) { const st = stepOfInstance(reg, s); if (st) for (const p of F.ports(st.out)) avail.add(p.type); }
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
