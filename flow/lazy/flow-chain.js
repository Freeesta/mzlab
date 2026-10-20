// @ts-check
// Flow (administrator mode): the chain of steps as a list of cards. Each card shows the inputs with a choice of source (only what fits, a conversion marked ⇄),
// the form of the step, its outputs and its own problems. Reordering and removing are buttons, so everything works without dragging (WCAG 2.5.7).
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;

  /** how a source is named in a list: the conversion it needs, when it needs one @param {any} reg @param {any} c candidate @param {FlowPort} port */
  const optionText = (reg, c, port) => {
    if (c.fit === "ok") return `${c.label} (${c.type})`;
    const path = (F.conversionPath(reg, c.type, port.type) || []).map((/** @type {any} */ s) => s.id).join(", ");
    return `⇄ ${c.label} (${c.type}, needs ${path})`;
  };

  /** the row of one input port @param {any} S state {plan, reg} @param {any} inst @param {FlowPort} port @param {(focus?: string) => void} done */
  const portRow = (S, inst, port, done) => {
    const { el, mark } = F, cands = F.candidates(S.plan, S.reg, inst.id, port), refs = F.refs(inst, port.name), id = `fl-in-${inst.id}-${port.name}`;
    const name = el("span", { class: "fl-pn" }, mark(port.type), el("span", { text: port.title + (port.optional ? " (optional)" : " *") }));
    const known = (/** @type {string} */ r) => cands.some((/** @type {any} */ c) => c.ref === r);
    const bad = refs.filter((/** @type {string} */ r) => !known(r));
    const choose = (/** @type {string} */ aria, /** @type {string[]} */ skip, /** @type {string} */ value) => el("select", {
      id, "data-fk": id, "aria-label": `${aria}: ${port.title}`, onchange: (/** @type {any} */ e) => {
        const v = e.target.value;
        F.setPort(inst, port, port.many ? (v ? [...refs, v] : refs) : (v ? [v] : []));
        done();
      },
    }, el("option", { value: "", text: port.many ? "Add a source…" : "not connected" }),
    ...cands.filter((/** @type {any} */ c) => !skip.includes(c.ref)).map((/** @type {any} */ c) => el("option", { value: c.ref, selected: c.ref === value, text: optionText(S.reg, c, port) })),
    ...(!port.many && bad.length ? [el("option", { value: bad[0], selected: true, text: `${bad[0]} (does not fit)` })] : []));
    if (!port.many) return [name, choose("Source", [], refs[0] || "")];
    const chips = refs.map((/** @type {string} */ r) => el("span", { class: "fl-chip" }, el("span", { text: r.replace(/^file:/, "") }),
      F.btn("x", `Disconnect ${r} from ${port.title}`, () => { F.setPort(inst, port, refs.filter((/** @type {string} */ x) => x !== r)); done(); })));
    return [name, el("span", { class: "fl-meta" }, ...chips, choose("Add a source", refs, ""))];
  };

  /** the card of an instance @param {any} S state {plan, reg, problems} @param {any} inst @param {number} k position @param {(focus?: string) => void} done */
  const card = (S, inst, k, done) => {
    const { el, btn } = F, step = F.stepOf ? F.stepOf(S.reg, inst) : S.reg.byId.get(inst.step.split("@")[0]), mine = S.problems.filter((/** @type {any} */ p) => p.instance === inst.id);
    const li = el("li", { class: "fl-card " + (mine.length ? "bad" : "ok"), id: `fl-card-${inst.id}`, "data-id": inst.id });
    if (!step) {
      li.append(el("div", { class: "fl-hd" }, el("span", { class: "fl-ti", text: inst.id }), el("span", { class: "fl-pk", text: inst.step }), el("span", { class: "fl-sp" }),
        btn("x", `Remove ${inst.id}`, () => { F.removeStep(S.plan, inst.id); done(); })));
    } else {
      li.append(el("div", { class: "fl-hd" }, el("span", { class: "fl-pk", text: String(k + 1) }), el("span", { class: "fl-ti", text: step.title }), el("span", { class: "fl-pk", text: `${inst.id} · ${step.id}@${step.version}` }),
        el("span", { class: "fl-sp" }),
        ...(F.compareButton ? [F.compareButton(S, inst, step, done)] : []),
        btn("up", `Move ${inst.id} up`, () => { F.moveStep(S.plan, inst.id, -1); done(`fl-card-${inst.id}`); }),
        btn("down", `Move ${inst.id} down`, () => { F.moveStep(S.plan, inst.id, 1); done(`fl-card-${inst.id}`); }),
        btn("x", `Remove ${inst.id}`, () => { F.removeStep(S.plan, inst.id); done(); })));
      if (step.doc) li.append(el("div", { class: "fl-note", text: step.doc }));
      const ports = el("div", { class: "fl-ports" });
      for (const p of F.ports(step.in)) ports.append(...portRow(S, inst, p, done));
      if (ports.children.length) li.append(ports);
      if (F.compareBody && inst.step.startsWith("meta.compare@")) li.append(F.compareBody(S, inst, done));
      else li.append(F.form(inst, step, done));
      li.append(el("div", { class: "fl-out" }, el("span", { text: "Gives:" }), ...F.ports(step.out).map((/** @type {FlowPort} */ p) => el("span", { class: "fl-meta" }, F.mark(p.type), p.title))));
    }
    if (mine.length) li.append(el("ul", { class: "fl-probs", "aria-label": `Problems of ${inst.id}` }, ...mine.map((/** @type {any} */ p) => el("li", {}, F.icon("warn", 14), el("span", { text: F.message(p) })))));
    return li;
  };

  /** the chain @param {any} S state {plan, reg, problems} @param {(focus?: string) => void} done called after every change (the page validates and draws again) */
  F.chain = (S, done) => {
    const { el } = F, ol = el("ol", { class: "fl-chain", "aria-label": "Chain of steps" });
    S.plan.steps.forEach((/** @type {any} */ inst, /** @type {number} */ k) => ol.append(card(S, inst, k, done)));
    return S.plan.steps.length ? ol : el("div", { class: "fl-empty" }, "No steps yet. Add a step from the list: it is connected to what fits.");
  };
})();
