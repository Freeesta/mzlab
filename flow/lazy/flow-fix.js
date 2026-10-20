// @ts-check
// Flow (administrator mode): the plan model of the canvas. The base chain, add-on analyses, templates, the nodes shown in Simple and Advanced, friendly texts for the
// problems of a node and the one-click fixes that go with them. No DOM here.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const last = (/** @type {any} */ a) => a[a.length - 1];

  /** the plan with the base chain connected, default settings @param {any} reg */
  F.basePlan = reg => {
    const plan = F.newPlan();
    for (const g of F.BASE) for (const id of g.steps) F.addStep(plan, reg, id);
    return plan;
  };
  /** instance ids of the base: configurable, never removable @param {any} plan */
  F.baseIds = plan => new Set(F.BASE.flatMap((/** @type {any} */ g) => g.steps).map((/** @type {string} */ s) => F.instanceOf(plan, s.split(".").pop()) ? s.split(".").pop() : ""));
  /** @param {any} plan @param {string} stepId the instance of a step, if the plan has one */
  F.findStep = (plan, stepId) => plan.steps.find((/** @type {any} */ s) => s.step.split("@")[0] === stepId);

  /** adds an add-on: the steps it lacks, each wired to what fits; returns the last instance added @param {any} S @param {any} addon */
  F.addAddon = (S, addon) => {
    let made = null;
    for (const id of addon.steps) if (!F.findStep(S.plan, id)) made = F.addStep(S.plan, S.reg, id);
    return made;
  };
  /** a plan for a template: the base, then its add-ons @param {any} reg @param {any} tpl */
  F.templatePlan = (reg, tpl) => {
    const S = { plan: F.basePlan(reg), reg };
    for (const a of tpl.addons) F.addAddon(S, F.ADDONS.find((/** @type {any} */ x) => x.id === a));
    return S.plan;
  };
  /** has the plan anything beyond the base? @param {any} plan */
  F.hasAddons = plan => plan.steps.length > F.BASE.reduce((n, g) => n + g.steps.length, 0);

  /** steps that could take the output of `inst` as a required input: the proposals of «Add next step» @param {any} S @param {any} inst */
  F.nextSteps = (S, inst) => {
    const step = F.stepOf ? F.stepOf(S.reg, inst) : S.reg.byId.get(inst.step.split("@")[0]), outs = F.ports(step.out), res = [];
    for (const cand of S.reg.steps) {
      if (cand.id === "meta.compare") continue;
      for (const port of F.ports(cand.in)) {
        const o = outs.find((/** @type {FlowPort} */ p) => F.fit(S.reg, p.type, port) === "ok");
        if (o && !port.optional) { res.push({ step: cand, port, out: o }); break; }
      }
    }
    return res;
  };
  /** adds the step and connects the port to `inst`'s output @param {any} S @param {any} inst @param {any} next one of nextSteps */
  F.addNext = (S, inst, next) => {
    const made = F.addStep(S.plan, S.reg, next.step.id);
    F.setPort(made, next.port, next.port.many ? [`${inst.id}.${next.out.name}`] : [`${inst.id}.${next.out.name}`]);
    return made;
  };

  /** the friendly text of a problem of a node and, when there is one, the fix @param {any} S @param {any} p problem {instance, port, key, params} */
  F.friendly = (S, p) => {
    const inst = F.instanceOf(S.plan, p.instance), step = inst && (F.stepOf ? F.stepOf(S.reg, inst) : S.reg.byId.get(inst.step.split("@")[0]));
    if (p.key === "err.flow.port.missing" && step) {
      const port = F.ports(step.in).find((/** @type {FlowPort} */ x) => x.name === p.port);
      if (port) return { text: `Needs ${F.INPUT_NAMES[port.type] ? F.INPUT_NAMES[port.type].toLowerCase() : port.title.toLowerCase()}.`, fix: fixFor(S, inst, port) };
    }
    return { text: F.message(p), fix: null };
  };
  /** @param {any} S @param {any} inst @param {FlowPort} port */
  const fixFor = (S, inst, port) => {
    if (F.INPUT_TYPES.includes(port.type)) {
      return { label: `Add ${F.INPUT_NAMES[port.type].toLowerCase()}`, run() { F.setPort(inst, port, ["file:" + F.addInput(S.plan, port.type).name]); } };
    }
    const prod = S.reg.steps.find((/** @type {any} */ s) => s.id !== "meta.compare" && !F.findStep(S.plan, s.id) && F.ports(s.out).some((/** @type {FlowPort} */ o) => F.fit(S.reg, o.type, port) === "ok"));
    if (!prod) return null;
    return { label: `Add “${F.nameOf(prod.id)[0]}”`, run() {
      const made = F.addStep(S.plan, S.reg, prod.id), o = F.ports(prod.out).find((/** @type {FlowPort} */ x) => F.fit(S.reg, x.type, port) === "ok");
      F.setPort(inst, port, [`${made.id}.${o.name}`]);
    } };
  };

  /** the nodes of the canvas. Simple folds the base into two nodes and hides the raw-file inputs; Advanced shows every step and every input
   * @param {any} S state {plan, reg, problems} @param {boolean} simple @returns {{nodes: any[], edges: [string, string][]}} */
  F.canvasModel = (S, simple) => {
    const plan = S.plan, nodes = /** @type {any[]} */ ([]), owner = /** @type {Map<string, string>} */ (new Map()), edges = /** @type {Set<string>} */ (new Set());
    const baseSteps = new Set(F.BASE.flatMap((/** @type {any} */ g) => g.steps));
    const lone = (/** @type {any} */ f) => !(simple && f.type === "raw");
    for (const f of plan.inputs.filter(lone)) { nodes.push({ key: "file:" + f.name, kind: "input", title: F.INPUT_NAMES[f.type] || f.type, why: "Your files. Chosen when the plan is run.", cat: "input", ids: [], file: f }); owner.set("file:" + f.name, "file:" + f.name); }
    const groups = simple ? F.BASE : [];
    for (const g of groups) {
      const insts = g.steps.map((/** @type {string} */ s) => F.findStep(plan, s)).filter(Boolean);
      nodes.push({ key: g.key, kind: "group", title: g.title, why: g.why, cat: g.key === "import" ? "input" : "process", ids: insts.map((/** @type {any} */ i) => i.id), locked: true });
      for (const i of insts) owner.set(i.id, g.key);
    }
    for (const inst of plan.steps) {
      if (owner.has(inst.id)) continue;
      const id = inst.step.split("@")[0], [name, cat, why] = F.nameOf(id);
      nodes.push({ key: inst.id, kind: "step", title: name, why, cat, ids: [inst.id], locked: baseSteps.has(id) });
      owner.set(inst.id, inst.id);
    }
    if (simple) for (const f of plan.inputs.filter((/** @type {any} */ x) => x.type === "raw")) owner.set("file:" + f.name, "import");
    for (const inst of plan.steps) for (const [, v] of Object.entries(inst.in || {})) for (const r of [].concat(/** @type {any} */ (v))) {
      const src = r.startsWith("file:") ? owner.get(r) : owner.get(r.split(".")[0]), dst = owner.get(inst.id);
      if (src && dst && src !== dst) edges.add(src + "\u0000" + dst);
    }
    const keys = new Set(nodes.map(n => n.key));
    return { nodes, edges: [...edges].map(e => /** @type {[string, string]} */ (e.split("\u0000"))).filter(([a, b]) => keys.has(a) && keys.has(b)) };
  };
  /** left-to-right layout: the column is the longest path from the left, the row the order inside the column @param {any[]} nodes @param {[string, string][]} edges */
  F.layout = (nodes, edges) => {
    const depth = new Map(nodes.map(n => [n.key, 0])), W = 280, H = 96;
    for (let k = 0; k < nodes.length; k++) for (const [a, b] of edges) depth.set(b, Math.max(/** @type {number} */ (depth.get(b)), /** @type {number} */ (depth.get(a)) + 1));
    const rows = new Map();
    for (const n of nodes) { const d = /** @type {number} */ (depth.get(n.key)), r = rows.get(d) || 0; rows.set(d, r + 1); n.x = 40 + d * W; n.y = 40 + r * H; }
    return { width: 80 + (Math.max(0, ...nodes.map(n => /** @type {number} */ (depth.get(n.key)))) + 1) * W, height: 80 + Math.max(1, ...rows.values()) * H };
  };
  /** the first problem of a node, plain @param {any} S @param {any} node */
  F.nodeProblems = (S, node) => S.problems.filter((/** @type {any} */ p) => node.ids.includes(p.instance)).map((/** @type {any} */ p) => F.friendly(S, p));
  F.lastOf = last;
})();
