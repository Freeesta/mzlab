// @ts-check
// Flow (administrator mode): the palette of steps, built from the descriptors: packages (Core, Attributio, Tabui), then families (first word of the id).
// One click on a step adds it at the end of the chain and connects what fits. A step that nothing in the plan can feed yet is dimmed, not hidden.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;

  /** the required inputs of a step that nothing in the plan could feed (a file-like port is always feedable: the plan gets the input) @param {any} plan @param {any} reg @param {any} step @returns {string[]} */
  F.unmet = (plan, reg, step) => F.ports(step.in).filter((/** @type {FlowPort} */ p) => !p.optional && !F.INPUT_TYPES.includes(p.type) && !F.candidates(plan, reg, "", p).length).map((/** @type {FlowPort} */ p) => p.name);

  /** the markup of the palette; `onAdd(stepId)` is called by a click. Returns {root, update(plan)} where update dims the steps that cannot be fed
   * @param {any} reg @param {(stepId: string) => void} onAdd */
  F.palette = (reg, onAdd) => {
    const { el, mark } = F;
    const search = el("input", { type: "search", placeholder: "Filter steps", "aria-label": "Filter steps", id: "fl-filter" });
    const list = el("div", { id: "fl-pal-list" });
    /** @type {Map<string, any>} */ const items = new Map();
    for (const [pkg, title] of F.PACKAGES) {
      const steps = reg.steps.filter((/** @type {any} */ s) => s.package === pkg);
      if (!steps.length) continue;
      const box = el("details", { open: pkg === "core", "data-pkg": pkg }, el("summary", { text: `${title} (${steps.length})` }));
      let family = "";
      for (const s of steps) {
        const fam = F.family(s.id);
        if (fam !== family) { box.append(el("h4", { text: fam })); family = fam; }
        const btn = el("button", { type: "button", class: "fl-it", "data-step": s.id, "aria-label": `Add ${s.title}`, title: s.doc, onclick: () => onAdd(s.id) },
          F.icon("plus", 14), el("span", { class: "fl-ti" }, s.title, el("small", { text: " " + s.id })),
          ...F.ports(s.out).slice(0, 2).map((/** @type {FlowPort} */ p) => mark(p.type)));
        items.set(s.id, btn); box.append(btn);
      }
      list.append(box);
    }
    const filter = () => {
      const q = search.value.trim().toLowerCase();
      for (const [id, b] of items) {
        const s = reg.byId.get(id);
        b.hidden = !!q && !(`${s.id} ${s.title} ${s.doc} ${s.package}`.toLowerCase().includes(q));
      }
      list.querySelectorAll("details").forEach((/** @type {HTMLDetailsElement} */ d) => { d.open = q ? [...d.querySelectorAll(".fl-it")].some(b => !b.hidden) : d.dataset.pkg === "core"; });
    };
    search.addEventListener("input", filter);
    const root = el("aside", { class: "fl-pal", "aria-label": "Steps" }, el("h3", { text: "Steps" }), search, list);
    return {
      root,
      update(/** @type {any} */ plan) {
        for (const [id, b] of items) {
          const miss = F.unmet(plan, reg, reg.byId.get(id));
          b.classList.toggle("fl-dim", miss.length > 0);
          b.title = reg.byId.get(id).doc + (miss.length ? `\nNothing in the plan can feed yet: ${miss.join(", ")}.` : "");
        }
      },
    };
  };
})();
