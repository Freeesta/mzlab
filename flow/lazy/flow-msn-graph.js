// @ts-check
// Flow (administrator mode): the fragmentation graph msngrafo/1 of spectra.msn_graph drawn as in the Fragmentation graph tab (same SVG, loaded from the app's modules), in English.
// Tree view, no filters, no drawings (the tab has them). Clicking an ion shows its candidate formulas; «Use» writes the choice in the parameter `scelte` of the step, and the next Run applies it. Used by flow-views-msn.js.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  /** the app module `name` (loaded once, the same instance as the app's) @param {string} name */
  const app = name => import(/* webpackIgnore: true */ new URL(name, G.QTOOLS && G.QTOOLS.ctx ? G.QTOOLS.ctx.base : document.baseURI).href);
  const EN = /** @type {Record<string, string>} */ ({
    "msngrafo.more": "+{n} more", "msngrafo.amb": "Several candidates: pick one with «Use»", "msngrafo.graph.aria": "Fragmentation graph: columns by MS level, arrows for the losses",
    "msngrafo.v.graph.radial": "Fragmentation graph, radial view: the precursor in the centre, one ring per step of the best path, arrows for the losses",
    "msngrafo.v.aria.levels": "levels {levels}", "msngrafo.v.aria.rel": "{rel} % of the base peak", "msngrafo.v.aria.formula": "formula {f}", "msngrafo.v.aria.cands": "{n} candidate formulas",
    "msngrafo.v.aria.common": "reached from {n} precursors", "msngrafo.v.aria.collapsed": "{n} ions closed below", "msngrafo.v.common": "Reached from {n} different precursors",
    "msngrafo.v.compat": "compatible with {f}", "msngrafo.v.hidden": "{n} closed", "msngrafo.v.hyp": "hypothesis",
    "msngrafo.prove.perdita": "known loss", "msngrafo.prove.isotopi": "isotopes", "msngrafo.prove.msn": "MSn", "msngrafo.rule.weak": "weak",
  });
  /** @param {string} key @param {Record<string, any>} [p] */
  const t = (key, p) => (EN[key] || key).replace(/\{(\w+)\}/g, (_, k) => String(p && p[k] !== undefined ? p[k] : ""));

  /** the pairs «MZ=ION» of `scelte` with the choice for `mz` set (or removed when `formula` is empty) @param {string} text @param {string} mz @param {string} formula */
  F.msnChoices = (text, mz, formula) => {
    const keep = (text || "").split(",").map(s => s.trim()).filter(s => s && Number(s.split("=")[0]) !== Number(mz));
    if (formula) keep.push(`${mz}=${formula}`);
    return keep.join(",");
  };

  /** the step instance of spectra.msn_graph in the plan on the page, with its step: null when the page has none @returns {{inst: any, step: any, S: any} | null} */
  const target = () => {
    const S = F.current, inst = S && S.plan && F.findStep(S.plan, "spectra.msn_graph"), step = inst && (F.stepOf ? F.stepOf(S.reg, inst) : S.reg.byId.get("spectra.msn_graph"));
    return inst && step ? { inst, step, S } : null;
  };

  /** the candidates of the selected ion, each with «Use» @param {HTMLElement} box @param {any} n @param {() => void} again */
  const candidates = (box, n, again) => {
    const { el } = F, f = n.formula, cs = (f && f.candidati) || [];
    box.replaceChildren(el("h4", { text: `MS${n.livello} · m/z ${Number(n.mz).toFixed(4)}` }));
    if (!cs.length) { box.append(el("p", { class: "fl-note", text: "No candidate formula for this ion." })); return; }
    const to = target(), rows = cs.map((/** @type {any} */ c) => [c.formula, String(Math.round(c.ppm * 100) / 100), f.scelta === c.formula ? "yes" : ""]);
    F.drawTable(box, { caption: "Candidate formulas of the ion", head: ["Candidate", "Error (ppm)", "Chosen"], rows, label: "Candidate formulas of the ion" });
    if (!to) return;
    const row = el("div", { class: "fl-meta" });
    for (const c of cs) row.append(F.btn("ok", `Use ${c.formula} for this ion (a candidate, not an identification)`, () => {
      F.setParam(to.inst, to.step, "scelte", F.msnChoices(String((to.inst.params || {}).scelte || ""), String(n.mz), c.formula) || null);
      to.S.refresh(null); again();
      box.append(el("p", { class: "fl-note", role: "status", text: `${c.formula} will be used for m/z ${Number(n.mz).toFixed(4)} at the next Run.` }));
    }, `Use ${c.formula}`));
    box.append(row);
  };

  /** the graph of `g` under `host`, with the candidates of the clicked ion and the downloads @param {HTMLElement} host @param {any} g @param {string} label */
  F.msnGraphView = async (host, g, label) => {
    const { el } = F, wrap = el("div", { class: "ig-wrap" }), side = el("div", { class: "fl-meta", style: "display:block" });
    host.append(wrap, side);
    if (!(g.nodi || []).length || !g.nodi.every((/** @type {any} */ n) => n.pos)) { wrap.replaceWith(el("p", { class: "fl-note", text: "The graph has no ions to draw." })); return; }
    try {
      const [svg, css] = await Promise.all([app("infusione-grafo-svg.js"), app("infusione-grafo-css.js")]);
      css.addStyle();
      /** @type {string | null} */ let sel = null;
      const draw = () => {
        const s = svg.drawGraph(wrap, g, { onSelect: (/** @type {string | null} */ id) => { sel = id; draw(); const n = id && g.nodi.find((/** @type {any} */ x) => x.id === id); if (n) candidates(side, n, () => {}); else side.replaceChildren(); }, onOpen: () => {} }, { t, sel });
        const on = sel && s.querySelector(`.ig-node[data-id="${CSS.escape(sel)}"]`); if (on) /** @type {HTMLElement} */ (on).focus({ preventScroll: true });
      };
      draw();
      const save = (/** @type {string} */ name, /** @type {string} */ type, /** @type {string} */ body) => { const a = el("a", { href: URL.createObjectURL(new Blob([body], { type })), download: name }); a.click(); setTimeout(() => URL.revokeObjectURL(a.href), 1000); };
      host.append(el("div", { class: "fl-meta" },
        F.btn("export", `Download ${label}`, () => save(label, "application/json", JSON.stringify(g, null, 1)), "JSON"),
        F.btn("export", "Download the graph as an SVG image", () => { const s = wrap.querySelector("svg"); if (s) app("msngrafo-immagine.js").then(m => save(label.replace(/\.json$/i, "") + ".svg", "image/svg+xml", m.svgAutonomo(s))); }, "SVG"),
        F.btn("export", "Download the graph as a PNG image", () => { const s = wrap.querySelector("svg"); if (s) app("msngrafo-immagine.js").then(async m => { const b = await m.pngDa(m.svgAutonomo(s), 2); const a = el("a", { href: URL.createObjectURL(b), download: label.replace(/\.json$/i, "") + ".png" }); a.click(); setTimeout(() => URL.revokeObjectURL(a.href), 1000); }); }, "PNG")));
    } catch (_) { wrap.replaceWith(el("p", { class: "fl-note", text: "The graph could not be drawn here: the tables show the same data." })); }
  };
})();
