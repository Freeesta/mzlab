// @ts-check
// Flow (administrator mode): what needs attention, under the canvas. «Issues» lists every need of the plan with the step it belongs to, the one-click remedy when there is one and a way to show the step.
// No DOM state of its own: the shell draws it again with the canvas.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;

  /** the list of needs @param {any} S @param {(instance: string) => any} nodeOf @param {(node: any) => void} show selects the node @param {() => void} done redraws after a remedy */
  F.issuesPanel = (S, nodeOf, show, done) => {
    const { el, icon } = F, box = el("section", { class: "flx-issues", "aria-label": "Issues" });
    const list = S.problems || [], extra = F.profileIssues ? F.profileIssues(S) : [];
    box.append(el("h3", { text: list.length + extra.length ? `Issues (${list.length + extra.length})` : "Issues" }));
    // nothing to fix: the box is not shown (the cards and the status in the bar already say so)
    if (!list.length && !extra.length) { box.hidden = true; return box; }
    const ul = el("ul", { class: "flx-needs", "aria-label": "Things to fix" });
    for (const p of extra) {
      const n = nodeOf(p.instance);
      ul.append(el("li", {}, icon("warn", 14), el("span", { text: `${n ? n.title : p.instance}: ${p.text}` }),
        ...p.fixes.map((/** @type {any} */ f) => el("button", { type: "button", class: "fl-b", onclick: () => { f.run(); done(); } }, icon("plus", 14), el("span", { text: f.label }))),
        n ? el("button", { type: "button", class: "fl-b", onclick: () => show(n) }, el("span", { text: "Show" })) : null));
    }
    for (const p of list) {
      const f = F.friendly(S, p), n = nodeOf(p.instance);
      ul.append(el("li", {}, icon("warn", 14), el("span", { text: `${n ? n.title : p.instance}: ${f.text}` }),
        f.fix ? el("button", { type: "button", class: "fl-b", onclick: () => { f.fix.run(); done(); } }, icon("plus", 14), el("span", { text: f.fix.label })) : null,
        n ? el("button", { type: "button", class: "fl-b", onclick: () => show(n) }, el("span", { text: "Show" })) : null));
    }
    box.append(ul);
    const det = F.findStep(S.plan, "features.detect");
    if (det && det.preset === "lr") box.append(el("p", { class: "fl-note", text: F.UNIT_NOTE }));
    return box;
  };
})();
