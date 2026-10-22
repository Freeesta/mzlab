// @ts-check
// Flow (administrator mode): what needs attention, under the canvas. «Issues» lists every need of the plan with the step it belongs to, the one-click remedy when there is one and a way to show the step;
// «?» beside a step opens the entry of the Theory that explains it (links only). No DOM state of its own: the shell draws it again with the canvas.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;

  /** the entry of the glossary of the Theory that explains each step (file and anchor of `mzlab/web/teoria/`); a step without an entry has no «?» */
  F.THEORY = {
    "spectra.centroid": ["21-glossario.html", "g-fwhm"],
    "index.mzidx": ["21-glossario.html", "g-full-scan-q1-ms"],
    "features.detect": ["21-glossario.html", "g-xic-eic"],
    "features.align": ["21-glossario.html", "g-rt-tr"],
    "features.gapfill": ["21-glossario.html", "g-xic-eic"],
    "formula.ms1": ["21-glossario.html", "g-accuratezza-di-massa-errore-in-ppm"],
    "spectra.match": ["21-glossario.html", "g-ms2-product-ion-scan"],
    "annot.level": ["21-glossario.html", "g-livelli-di-confidenza"],
    "tp.generate": ["21-glossario.html", "g-tp"],
  };

  /** the link «?» of a step to the Theory, or null when it has none @param {string} stepId */
  F.theoryLink = stepId => {
    const t = F.THEORY[stepId.split("@")[0]];
    if (!t) return null;
    return F.el("a", { class: "fl-b flx-q", href: `teoria/${t[0]}#${t[1]}`, target: "_blank", rel: "noopener", "aria-label": `What is this? Opens the Theory for ${F.nameOf(stepId.split("@")[0])[0]}`, title: "What is this? Opens the Theory" }, "?");
  };

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
    return box;
  };
})();
