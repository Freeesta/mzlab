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

  /** the state of the Fragmentation graph tab of Data (one instance, shared with the tab): the graph and the choices of the step in this plan, so its ion panel can be drawn here; `again` is called after a «Use» @param {any} g */
  const dataState = async g => {
    const st = await app("infusione-grafo-stato.js"), to = target();
    st.G.j = g; st.G.ro = null; st.G.scelte = to ? String((to.inst.params || {}).scelte || "").split(",").map(x => { const [mz, f] = x.split("="); return { mz: Number(mz), formula: (f || "").trim() }; }).filter(c => c.mz && c.formula) : [];
    return st.G;
  };

  /** the panel of the selected ion, the one of the Fragmentation graph tab; «Use» writes `scelte` of the step @param {HTMLElement} box @param {any} G0 @param {any} g @param {string | null} id */
  const ionPanel = async (box, G0, g, id) => {
    const node = await app("infusione-grafo-nodo.js"), n = id && g.nodi.find((/** @type {any} */ x) => x.id === id);
    G0.j = g; G0.sel = id;
    if (!n) { box.replaceChildren(); return; }
    const to = target(), redo = () => {
      if (!to) return;
      F.setParam(to.inst, to.step, "scelte", G0.scelte.map((/** @type {any} */ c) => `${c.mz}=${c.formula}`).join(",") || null);
      to.S.refresh(null); box.append(F.el("p", { class: "fl-note", role: "status", text: "Applied at the next Run." }));
    };
    node.nodePanel(box, n, { redo, again: () => ionPanel(box, G0, g, id) });
  };

  /** opens the Fragmentation graph tab of Data on the infusion and the settings of the step @param {any} G0 */
  const openInData = async G0 => {
    const to = target(); if (!to) throw new Error("The plan has no fragmentation graph.");
    const p = to.inst.params || {}, text = (/** @type {string} */ n) => (typeof p[n] === "string" ? p[n].trim() : "");
    await G.carica("m:infusione-grafo.js");
    const k = F.webMsn.infusion(to.S);
    Object.assign(G0, { mol: null, text: text("formula"), adduct: text("adduct"), smiles: text("smiles"), j: null, sel: null, k: -1, ro: null });
    for (const n of ["ppm_formula", "mz_basso", "ppm_basso", "prove_min"]) if (typeof p[n] === "number") G0.o[n] = p[n];
    G0.o.attivazione = text("attivazione"); G0.o.isotopi = p.isotopi === true;
    STATO.file(k, "flow"); setView("data"); BARRA.show("gra");
  };

  /** the graph of `g` under `host`, with the candidates of the clicked ion and the downloads @param {HTMLElement} host @param {any} g @param {string} label */
  F.msnGraphView = async (host, g, label) => {
    const { el } = F, wrap = el("div", { class: "ig-wrap" }), side = el("div", { class: "fl-meta", style: "display:block" }), bar = el("div", { class: "flx-seg" }), msg = el("p", { class: "fl-note", role: "status" });
    host.append(bar, wrap, side, msg);
    if (!(g.nodi || []).length || !g.nodi.every((/** @type {any} */ n) => n.pos)) { wrap.replaceWith(el("p", { class: "fl-note", text: "The graph has no ions to draw." })); return; }
    try {
      const [svg, css, G0] = await Promise.all([app("infusione-grafo-svg.js"), app("infusione-grafo-css.js"), dataState(g)]);
      css.addStyle();
      /** @type {string | null} */ let sel = null; /** @type {"tree" | "radial"} */ let mode = "tree";
      const draw = () => {
        const s = svg.drawGraph(wrap, g, { onSelect: (/** @type {string | null} */ id) => { sel = id; draw(); void ionPanel(side, G0, g, id); }, onOpen: () => {} }, { t, sel, mode });
        const on = sel && s.querySelector(`.ig-node[data-id="${CSS.escape(sel)}"]`); if (on) /** @type {HTMLElement} */ (on).focus({ preventScroll: true });
      };
      for (const [name, m] of /** @type {[string, "tree" | "radial"][]} */ ([["Tree", "tree"], ["Radial", "radial"]])) bar.append(el("button", { type: "button", "aria-pressed": String(m === mode), text: name, onclick: () => {
        mode = m; for (const b of bar.querySelectorAll("button[aria-pressed]")) b.setAttribute("aria-pressed", String(b.textContent === name)); draw();
      } }));
      bar.append(el("button", { type: "button", text: "Open in Data", onclick: () => { msg.textContent = ""; openInData(G0).catch(e => { msg.textContent = F.message ? F.message(e) : String(e && e.message || e); }); } }));
      draw();
      const save = (/** @type {string} */ name, /** @type {string} */ type, /** @type {string} */ body) => { const a = el("a", { href: URL.createObjectURL(new Blob([body], { type })), download: name }); a.click(); setTimeout(() => URL.revokeObjectURL(a.href), 1000); };
      host.append(el("div", { class: "fl-meta" },
        F.btn("export", `Download ${label}`, () => save(label, "application/json", JSON.stringify(g, null, 1)), "JSON"),
        F.btn("export", "Download the graph as an SVG image", () => { const s = wrap.querySelector("svg"); if (s) app("msngrafo-immagine.js").then(m => save(label.replace(/\.json$/i, "") + ".svg", "image/svg+xml", m.svgAutonomo(s))); }, "SVG"),
        F.btn("export", "Download the graph as a PNG image", () => { const s = wrap.querySelector("svg"); if (s) app("msngrafo-immagine.js").then(async m => { const b = await m.pngDa(m.svgAutonomo(s), 2); const a = el("a", { href: URL.createObjectURL(b), download: label.replace(/\.json$/i, "") + ".png" }); a.click(); setTimeout(() => URL.revokeObjectURL(a.href), 1000); }); }, "PNG")));
    } catch (_) { wrap.replaceWith(el("p", { class: "fl-note", text: "The graph could not be drawn here: the tables show the same data." })); }
  };
})();
