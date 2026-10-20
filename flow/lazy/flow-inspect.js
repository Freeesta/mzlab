// @ts-check
// Flow (administrator mode): the panel on the right of the canvas. For the selected card: what the step does in a plain sentence, what it needs (with a one-click fix),
// the settings (Simple: the form without «More»; Advanced: everything, the sources of every input and the comparison), and the neighbours as chips.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;

  /** @param {any} S @param {any} inst @param {(focus?: string) => void} done */
  const sources = (S, inst, done) => {
    const step = F.stepOf ? F.stepOf(S.reg, inst) : S.reg.byId.get(inst.step.split("@")[0]), ports = F.el("div", { class: "fl-ports" });
    for (const p of F.ports(step.in)) ports.append(...F.portRow(S, inst, p, done));
    return ports.children.length ? ports : null;
  };

  /** @param {any} S state {plan, reg, problems} @param {any | null} node @param {{simple: boolean, done: (focus?: string) => void, select: (key: string) => void, remove: (node: any) => void, model: any}} c */
  F.inspector = (S, node, c) => {
    const { el, icon } = F;
    if (!node) return el("div", { class: "fl-insp-empty", text: "Select a step to see what it does and change its settings." });
    const cat = F.CATS[node.cat], probs = F.nodeProblems(S, node);
    const box = el("div", { class: "fl-insp", style: `--cc:${cat.color}` },
      el("div", { class: "fl-insp-hd" }, el("span", { class: "fln-ic" }, icon(cat.icon, 20)), el("h3", { text: node.title }), el("span", { class: "fl-pk", text: cat.word })),
      el("p", { class: "fl-insp-why" }, el("strong", { text: "What this step does. " }), node.why));
    if (probs.length) {
      const ul = el("ul", { class: "fl-needs", "aria-label": "What this step needs" });
      probs.forEach((/** @type {any} */ p) => ul.append(el("li", {}, icon("warn", 14), el("span", { text: p.text }),
        p.fix ? el("button", { type: "button", class: "fl-b fl-fix", onclick: () => { p.fix.run(); c.done(); } }, icon("plus", 14), el("span", { text: p.fix.label })) : null)));
      box.append(ul);
    } else box.append(el("p", { class: "fl-ready" }, icon("ok", 14), " Ready."));
    if (node.kind === "input") { box.append(el("p", { class: "fl-note", text: "The file is chosen when the plan is run." }));
      if (!c.simple) box.append(F.btn("x", `Remove ${node.title}`, () => c.remove(node), "Remove")); return box; }
    for (const id of node.ids) {
      const inst = F.instanceOf(S.plan, id), step = inst && (F.stepOf ? F.stepOf(S.reg, inst) : S.reg.byId.get(inst.step.split("@")[0]));
      if (!inst || !step) continue;
      if (node.ids.length > 1) box.append(el("h4", { text: F.nameOf(step.id)[0] }), el("p", { class: "fl-note", text: F.nameOf(step.id)[2] }));
      if (!c.simple) { const src = sources(S, inst, c.done); if (src) box.append(src); }
      box.append(F.compareBody && inst.step.startsWith("meta.compare@") ? F.compareBody(S, inst, c.done) : F.form(inst, step, c.done));
      if (!c.simple && F.compareButton) box.append(F.compareButton(S, inst, step, c.done));
    }
    const near = c.model.edges.filter((/** @type {string[]} */ e) => e.includes(node.key)), byKey = new Map(c.model.nodes.map((/** @type {any} */ n) => [n.key, n]));
    const chips = (/** @type {string} */ label, /** @type {string[]} */ keys) => keys.length ? el("div", { class: "fl-meta" }, el("span", { class: "fl-note", text: label }),
      ...keys.map(k => el("button", { type: "button", class: "fl-chip fl-link", onclick: () => c.select(k) }, el("span", { text: /** @type {any} */ (byKey.get(k)).title })))) : null;
    box.append(chips("Uses:", near.filter((/** @type {string[]} */ e) => e[1] === node.key).map((/** @type {string[]} */ e) => e[0])), chips("Feeds into:", near.filter((/** @type {string[]} */ e) => e[0] === node.key).map((/** @type {string[]} */ e) => e[1])));
    if (node.locked) box.append(el("p", { class: "fl-note", text: "This is part of the base. You can change its settings but not remove it." }));
    else box.append(F.btn("x", `Remove ${node.title}`, () => c.remove(node), "Remove step"));
    return box;
  };
})();
