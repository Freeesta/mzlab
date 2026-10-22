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
    // a template with its own steps (the MSn infusion) does not start from the feature-table base
    const S = { plan: tpl.steps ? F.newPlan() : F.basePlan(reg), reg };
    for (const id of tpl.steps || []) F.addStep(S.plan, reg, id);
    for (const a of tpl.addons) F.addAddon(S, F.ADDONS.find((/** @type {any} */ x) => x.id === a));
    // the settings a template starts with, by step id: {"table.filter": {blank_ratio: 3}}
    for (const [stepId, values] of Object.entries(tpl.params || {})) {
      const inst = F.findStep(S.plan, stepId), step = inst && reg.byId.get(stepId);
      if (inst && step) for (const [name, v] of Object.entries(values)) F.setParam(inst, step, name, v);
    }
    return S.plan;
  };
  /** has the plan anything beyond the base? (a plan without the base chain has its own steps) @param {any} plan */
  F.hasAddons = plan => !F.findStep(plan, F.BASE[0].steps[0]) || plan.steps.length > F.BASE.reduce((n, g) => n + g.steps.length, 0);

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
      // a sample list is filled from the files open in Data in the same click when there are any
      const data = port.type === "samples" && F.samplesFromData ? F.samplesFromData() : null;
      return { label: data ? "Add sample list from Data" : `Add ${F.INPUT_NAMES[port.type].toLowerCase()}`, run() {
        const input = F.addInput(S.plan, port.type); F.setPort(inst, port, ["file:" + input.name]);
        if (data) void F.samplesSet(S, input, data).then(() => { if (S.redraw) S.redraw(); });
      } };
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
    // the base group of a step the plan lacks (the MSn template has no feature table) is not drawn; without the import group the raw files stay visible
    const present = (/** @type {any} */ g) => g.steps.some((/** @type {string} */ s) => F.findStep(plan, s));
    const lone = (/** @type {any} */ f) => !(simple && f.type === "raw" && present(F.BASE[0]));
    for (const f of plan.inputs.filter(lone)) { nodes.push({ key: "file:" + f.name, kind: "input", title: F.INPUT_NAMES[f.type] || f.type, why: "Your data files.", cat: "input", ids: [], file: f }); owner.set("file:" + f.name, "file:" + f.name); }
    const groups = simple ? F.BASE : [];
    for (const g of groups) {
      const insts = g.steps.map((/** @type {string} */ s) => F.findStep(plan, s)).filter(Boolean);
      if (!insts.length) continue;
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
  /** the names of the files open in the Data tab @returns {string[]} */
  F.dataFiles = () => {
    const open = typeof E !== "undefined" && Array.isArray(E.files) ? E.files.filter((/** @type {any} */ f) => f && !f.gone && f.file) : [];
    return [...new Set(open.map((/** @type {any} */ f) => String(f.file).split("#")[0]))];
  };
  /** the state of a card: «warn» with the reason when data or settings are missing, «ok» when it can run from the start @param {any} S @param {any} node @returns {{ok: boolean, word: string}} */
  F.nodeState = (S, node) => {
    if (F.nodeProblems(S, node).length) return { ok: false, word: "Needs input" };
    if (node.key === "import" && !F.dataFiles().length) return { ok: false, word: "No data" };
    if (node.kind === "input" && node.file.type !== "samples" && !F.dataFiles().length) return { ok: false, word: "Add a file" };
    return { ok: true, word: "Runs" };
  };
  /** the first problem of a node, plain @param {any} S @param {any} node */
  F.nodeProblems = (S, node) => S.problems.filter((/** @type {any} */ p) => node.ids.includes(p.instance)).map((/** @type {any} */ p) => F.friendly(S, p));
  F.lastOf = last;
})();
