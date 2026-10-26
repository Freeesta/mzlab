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

  /** the files open in the Data tab, as the Import data card shows them: «No file loaded» with a way to Data, or the count and the first names @param {any} el */
  const dataState = el => {
    const names = F.dataFiles();
    if (!names.length) return el("p", { class: "fl-note fl-data" }, el("span", { text: "No file loaded. " }), F.btn("import", "Go to the Data tab to open files", () => { const b = document.querySelector('#nav [data-v="data"]'); if (b) /** @type {HTMLElement} */ (b).click(); }, "Open Data"));
    return el("p", { class: "fl-note fl-data", text: `${names.length} file${names.length > 1 ? "s" : ""} in Data: ${names.slice(0, 3).join(", ")}${names.length > 3 ? ", …" : ""}` });
  };

  /** @param {any} S state {plan, reg, problems} @param {any | null} node @param {{simple: boolean, done: (focus?: string) => void, select: (key: string) => void, remove: (node: any) => void, model: any}} c */
  F.inspector = (S, node, c) => {
    const { el, icon } = F;
    if (!node) return el("div", { class: "fl-insp-empty", text: "Select a step to see what it does and change its settings." });
    const cat = F.CATS[node.cat], probs = F.nodeProblems(S, node);
    const box = el("div", { class: "fl-insp", style: `--cc:${cat.color}` },
      el("div", { class: "fl-insp-hd" }, el("span", { class: "fln-ic" }, icon(cat.icon, 20)), el("h3", { text: node.title }), el("span", { class: "fl-pk", text: cat.word }), (node.ids || []).length && F.theoryLink ? F.theoryLink(((F.instanceOf(S.plan, node.ids[0]) || {}).step) || "") : null),
      el("p", { class: "fl-insp-why", text: node.why }));
    if (probs.length) {
      const ul = el("ul", { class: "fl-needs", "aria-label": "What this step needs" });
      probs.forEach((/** @type {any} */ p) => ul.append(el("li", {}, icon("warn", 14), el("span", { text: p.text }),
        p.fix ? el("button", { type: "button", class: "fl-b fl-fix", onclick: () => { p.fix.run(); c.done(); } }, icon("plus", 14), el("span", { text: p.fix.label })) : null)));
      box.append(ul);
    }
    if (node.key === "import") box.append(dataState(el), ...(F.importBox ? [F.importBox(S)] : []));
    if (node.kind === "input") { box.append(node.file && node.file.type === "samples" && F.samplesEditor ? F.samplesEditor(S, node.file) : node.file && node.file.type === "structure" && F.parentEditor ? F.parentEditor(S, node.file) : F.addFilesBox ? el("div", {}, dataState(el), F.addFilesBox(S)) : dataState(el));
      if (!c.simple) box.append(F.btn("x", `Remove ${node.title}`, () => c.remove(node), "Remove")); return box; }
    for (const id of node.ids) {
      const inst = F.instanceOf(S.plan, id), step = inst && (F.stepOf ? F.stepOf(S.reg, inst) : S.reg.byId.get(inst.step.split("@")[0]));
      if (!inst || !step) continue;
      if (node.ids.length > 1) box.append(el("h4", { text: F.nameOf(step.id)[0] }), el("p", { class: "fl-note", text: F.nameOf(step.id)[2] }));
      if (!c.simple) { const src = sources(S, inst, c.done); if (src) box.append(src); }
      box.append(F.compareBody && inst.step.startsWith("meta.compare@") ? F.compareBody(S, inst, c.done) : F.form(inst, step, c.done));
      if (!c.simple && F.compareButton) box.append(F.compareButton(S, inst, step, c.done));
    }
    const filesTable = F.liveCard ? F.liveCard.files(node) : null;
    if (filesTable) { box.append(filesTable); F.liveCard.paint(); }
    const results = F.resultsCard ? F.resultsCard(S, node) : null;
    if (results) box.append(results);
    const near = c.model.edges.filter((/** @type {string[]} */ e) => e.includes(node.key)), byKey = new Map(c.model.nodes.map((/** @type {any} */ n) => [n.key, n]));
    const chips = (/** @type {string} */ label, /** @type {string[]} */ keys) => keys.length ? el("div", { class: "fl-meta" }, el("span", { class: "fl-note", text: label }),
      ...keys.map(k => el("button", { type: "button", class: "fl-chip fl-link", onclick: () => c.select(k) }, el("span", { text: /** @type {any} */ (byKey.get(k)).title })))) : null;
    for (const row of [chips("From", near.filter((/** @type {string[]} */ e) => e[1] === node.key).map((/** @type {string[]} */ e) => e[0])), chips("To", near.filter((/** @type {string[]} */ e) => e[0] === node.key).map((/** @type {string[]} */ e) => e[1]))]) if (row) box.append(row);
    if (!node.locked) box.append(F.btn("x", `Remove ${node.title}`, () => c.remove(node), "Remove step"));
    return box;
  };
})();
