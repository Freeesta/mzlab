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
  /** the step an instance means @param {any} reg @param {any} inst */
  const stepOfInstance = (reg, inst) => reg.byId.get(inst.step.split("@")[0]);
  F.stepOfInstance = stepOfInstance;
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
})();
