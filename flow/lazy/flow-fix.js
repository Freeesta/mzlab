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
  /** the filter of the rows reads the table of the alignment, not the registry of the filters of the TP chain (the last table the add-ons made) @param {any} plan @param {any} reg */
  const wireReliable = (plan, reg) => {
    const rel = F.findStep(plan, "table.filter"), gap = F.findStep(plan, "features.gapfill"), rstep = rel && reg.byId.get("table.filter");
    const tport = rstep && F.ports(rstep.in).find((/** @type {FlowPort} */ x) => x.name === "table");
    if (rel && gap && tport) F.setPort(rel, tport, [`${gap.id}.table`]);
  };
  /** a plan for a template: the base, then its add-ons @param {any} reg @param {any} tpl */
  F.templatePlan = (reg, tpl) => {
    // a template with its own steps (the MSn infusion) does not start from the feature-table base
    const S = { plan: tpl.steps ? F.newPlan() : F.basePlan(reg), reg };
    for (const id of tpl.steps || []) F.addStep(S.plan, reg, id);
    for (const a of tpl.addons) F.addAddon(S, F.ADDONS.find((/** @type {any} */ x) => x.id === a));
    // the hypotheses read the polarity and the resolution of the files unless the plan writes them: the index goes in
    const gen = F.findStep(S.plan, "tp.generate"), idx = F.findStep(S.plan, "index.mzidx"), gstep = gen && reg.byId.get("tp.generate");
    const iport = gstep && F.ports(gstep.in).find((/** @type {FlowPort} */ x) => x.name === "index");
    if (gen && idx && iport && !gen.in.index) F.setPort(gen, iport, [`${idx.id}.index`]);
    // the network reads the rows of the link (a filter keeps a view, not a table) and the same sample list, for the time course
    const net = F.findStep(S.plan, "tp.network"), link = F.findStep(S.plan, "tp.link"), nstep = net && reg.byId.get("tp.network");
    if (net && link && nstep) for (const [name, ref] of [["table", link.id + ".tp"], ["samples", link.in.samples]]) {
      const port = F.ports(nstep.in).find((/** @type {FlowPort} */ x) => x.name === name);
      if (port && ref) F.setPort(net, port, [].concat(ref));
    }
    wireReliable(S.plan, reg);
    // the settings a template starts with, by step id: {"table.filter": {blank_ratio: 3}}
    for (const [stepId, values] of Object.entries(tpl.params || {})) {
      const inst = F.findStep(S.plan, stepId), step = inst && reg.byId.get(stepId);
      if (inst && step) for (const [name, v] of Object.entries(values)) F.setParam(inst, step, name, v);
    }
    return S.plan;
  };
  /** has the plan anything beyond the base? (a plan without the base chain has its own steps) @param {any} plan */
  F.hasAddons = plan => !F.findStep(plan, "index.mzidx") || plan.steps.some((/** @type {any} */ s) => !F.BASE.some((/** @type {any} */ g) => g.steps.includes(s.step.split("@")[0])));

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
      if (port) {
        const what = F.INPUT_NAMES[port.type] ? F.INPUT_NAMES[port.type].toLowerCase() : port.title.toLowerCase(), prod = F.INPUT_TYPES.includes(port.type) ? null : producerOf(S, port);
        return { text: prod ? `Needs ${what}: it comes from “${F.nameOf(prod.id)[0]}”, which this plan does not have yet.` : `Needs ${what}.`, fix: fixFor(S, inst, port) };
      }
    }
    return { text: F.message(p), fix: null };
  };
  /** the first step the plan lacks that makes what the port reads @param {any} S @param {FlowPort} port */
  const producerOf = (S, port) => S.reg.steps.find((/** @type {any} */ s) => s.id !== "meta.compare" && !F.findStep(S.plan, s.id) && F.ports(s.out).some((/** @type {FlowPort} */ o) => F.fit(S.reg, o.type, port) === "ok"));
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
    const prod = producerOf(S, port);
    if (!prod) return null;
    return { label: `Add “${F.nameOf(prod.id)[0]}”`, run() {
      const made = F.addStep(S.plan, S.reg, prod.id), o = F.ports(prod.out).find((/** @type {FlowPort} */ x) => F.fit(S.reg, x.type, port) === "ok");
      F.setPort(inst, port, [`${made.id}.${o.name}`]);
    } };
  };

  /** the Data files in profile (the first scan of the mixed ones too), by name: what the centroid step is for */
  const profileNames = () => {
    const have = new Set(F.dataFiles()), all = typeof ST !== "undefined" && Array.isArray(ST.files) ? ST.files : [];
    return all.filter((/** @type {any} */ f) => have.has(f.name) && (f.spectrum_mode === "profile" || f.spectrum_mode === "mixed")).map((/** @type {any} */ f) => f.name);
  };
  /** puts «Centroid spectra» between the files and the index: it takes what the index took, the index takes its output @param {any} S */
  const addCentroid = (/** @type {any} */ S) => {
    const idx = F.findStep(S.plan, "index.mzidx");
    if (!idx || F.findStep(S.plan, "spectra.centroid")) return null;
    const cen = F.addStep(S.plan, S.reg, "spectra.centroid"), took = idx.in.source;
    cen.in.source = [].concat(took || []);
    idx.in.source = Array.isArray(took) ? [cen.id + ".mzml"] : cen.id + ".mzml";
    S.plan.steps.splice(S.plan.steps.indexOf(cen), 1);
    S.plan.steps.splice(S.plan.steps.indexOf(idx), 0, cen);
    return cen;
  };
  /** the need of a plan that reads profile files with no centroid step and no choice written in it: the two remedies are the user's to pick (the check never inserts a step) @param {any} S @returns {{instance: string, text: string, fixes: {label: string, run: () => void}[]}[]} */
  const profileIssues = (/** @type {any} */ S) => {
    const idx = F.findStep(S.plan, "index.mzidx"), names = profileNames();
    if (!idx || F.findStep(S.plan, "spectra.centroid") || S.plan.profile === "as_is" || !names.length) return [];
    return [{ instance: idx.id, text: `${names.length} profile file${names.length > 1 ? "s" : ""} (${names.slice(0, 3).join(", ")}${names.length > 3 ? "…" : ""}): peaks are found on centroids, so their detection waits for your choice.`, fixes: [
      { label: "Centroid the profile files", run: () => { F.addCentroid(S); } },
      { label: "Run on profile anyway", run: () => { S.plan.profile = "as_is"; } },
    ] }];
  };

  Object.assign(F, { profileNames, addCentroid, profileIssues });

  /** the nodes of the canvas. Simple folds the base into two nodes and hides the raw-file inputs; Advanced shows every step and every input
   * @param {any} S state {plan, reg, problems} @param {boolean} simple @returns {{nodes: any[], edges: [string, string][]}} */
  F.canvasModel = (S, simple) => {
    const plan = S.plan, nodes = /** @type {any[]} */ ([]), owner = /** @type {Map<string, string>} */ (new Map()), edges = /** @type {Set<string>} */ (new Set());
    const baseSteps = new Set(F.BASE.flatMap((/** @type {any} */ g) => g.steps));
    // the base group of a step the plan lacks (the MSn template has no feature table) is not drawn; without the import group the raw files stay visible
    const present = (/** @type {any} */ g) => g.steps.some((/** @type {string} */ s) => F.findStep(plan, s));
    const lone = (/** @type {any} */ f) => !(simple && (f.type === "raw" || f.type === "mzml") && present(F.BASE[0]));
    for (const f of plan.inputs.filter(lone)) { nodes.push({ key: "file:" + f.name, kind: "input", title: F.INPUT_NAMES[f.type] || f.type, why: "Your data files.", cat: "input", ids: [], file: f }); owner.set("file:" + f.name, "file:" + f.name); }
    const groups = simple ? [...F.BASE, ...F.GROUPS] : [];
    for (const g of groups) {
      const insts = g.steps.map((/** @type {string} */ s) => F.findStep(plan, s)).filter(Boolean);
      if (!insts.length) continue;
      const base = F.BASE.includes(g);
      nodes.push({ key: g.key, kind: "group", title: g.title, why: g.why, cat: g.cat || (g.key === "import" ? "input" : "process"), ids: insts.map((/** @type {any} */ i) => i.id), locked: base });
      for (const i of insts) owner.set(i.id, g.key);
    }
    for (const inst of plan.steps) {
      if (owner.has(inst.id)) continue;
      const id = inst.step.split("@")[0], [name, cat, why] = F.nameOf(id);
      nodes.push({ key: inst.id, kind: "step", title: name, why, cat, ids: [inst.id], locked: baseSteps.has(id) });
      owner.set(inst.id, inst.id);
    }
    if (simple && present(F.BASE[0])) for (const f of plan.inputs.filter((/** @type {any} */ x) => x.type === "raw" || x.type === "mzml")) owner.set("file:" + f.name, "import");
    for (const inst of plan.steps) for (const [, v] of Object.entries(inst.in || {})) for (const r of [].concat(/** @type {any} */ (v))) {
      const src = r.startsWith("file:") ? owner.get(r) : owner.get(r.split(".")[0]), dst = owner.get(inst.id);
      if (src && dst && src !== dst) edges.add(src + "\u0000" + dst);
    }
    // a step that reads the infusion open in Data has no `file:` in the plan: the raw files still lead to it
    for (const inst of plan.steps) if (F.INFUSION_STEPS.includes(inst.step.split("@")[0]) && !(inst.in && inst.in.source)) for (const f of plan.inputs.filter((/** @type {any} */ x) => x.type === "raw" || x.type === "mzml")) {
      const src = owner.get("file:" + f.name), dst = owner.get(inst.id);
      if (src && dst && src !== dst) edges.add(src + "\u0000" + dst);
    }
    // the view cards: beside a step that has a view, joined to it
    for (const inst of plan.steps) {
      const view = F.STEP_VIEWS[inst.step.split("@")[0]], from = owner.get(inst.id);
      if (!view || !from) continue;
      const key = "view:" + inst.id;
      nodes.push({ key, kind: "view", title: view.title, why: view.why, cat: "output", ids: [], of: inst.id, result: view.result, locked: true });
      edges.add(from + "\u0000" + key);
    }
    const keys = new Set(nodes.map(n => n.key));
    return { nodes, edges: [...edges].map(e => /** @type {[string, string]} */ (e.split("\u0000"))).filter(([a, b]) => keys.has(a) && keys.has(b)) };
  };
  /** left-to-right layout: the column is the longest path from the left, the row the order inside the column @param {any[]} nodes @param {[string, string][]} edges */
  F.layout = (nodes, edges) => {
    // few cards: a tighter grid
    const small = nodes.length <= 5, depth = new Map(nodes.map(n => [n.key, 0])), W = small ? 250 : 280, H = small ? 88 : 96;
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
