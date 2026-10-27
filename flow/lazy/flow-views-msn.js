// @ts-check
// Flow (administrator mode): the result view of the step spectra.msn_graph, the fragmentation graph msngrafo/1 of an MSn infusion. Its tables are the modifications
// (neutral losses between ions, with the formulas of both ions) and the ions with their proposed sub-formulas, the same columns as the CSV of msn.table_export.
// flow-report.js hands a report JSON of this schema to F.openMsnGraph; F.webMsnAfterRun puts the texts an MSn run made in the Results and opens the graph.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const txt = (/** @type {any} */ v) => v === null || v === undefined ? "" : String(v);
  const fx = (/** @type {any} */ n, d = 4) => n === null || n === undefined || !Number.isFinite(Number(n)) ? "" : String(Math.round(Number(n) * 10 ** d) / 10 ** d);

  /** the chosen sub-formula of a node and its error in ppm @param {any} n @returns {[string, string]} */
  const chosen = n => {
    const f = n && n.formula, name = f && f.scelta, c = ((f && f.candidati) || []).find((/** @type {any} */ x) => x.formula === name);
    return [txt(name), c ? fx(c.ppm, 2) : ""];
  };

  /** msngrafo/1 → the modifications (one row per arc and measure) and the ions @param {any} g @returns {{modifications: {head: string[], rows: string[][]}, nodes: {head: string[], rows: string[][]}}} */
  F.readMsnGraph = g => {
    const byId = new Map((g.nodi || []).map((/** @type {any} */ n) => [n.id, n]));
    const mods = /** @type {string[][]} */ ([]);
    for (const a of g.archi || []) {
      const from = /** @type {any} */ (byId.get(a.da)), to = /** @type {any} */ (byId.get(a.a)), l = a.perdita || {};
      const head = [txt(a.da), fx(from && from.mz, 5), chosen(from)[0], txt(a.a), fx(to && to.mz, 5), chosen(to)[0], fx(l.da_massa, 5), txt(l.formula), fx(l.ppm, 2), a.isolamento ? "yes" : "no"];
      const ms = a.misure || [];
      if (!ms.length) mods.push([...head, "", "", "", "", "", ""]);
      for (const m of ms) mods.push([...head, txt(m.albero), fx(m.livello, 0), txt(m.att), fx(m.ce, 1), fx(m.n_scansioni, 0), fx(m.rel, 2)]);
    }
    const nodes = (g.nodi || []).map((/** @type {any} */ n) => {
      const [f, ppm] = chosen(n);
      return [txt(n.id), fx(n.livello, 0), fx(n.mz, 5), f, ppm, txt(n.formula && n.formula.fonte), String(((n.formula && n.formula.candidati) || []).length)];
    });
    return {
      modifications: { head: ["From", "From m/z", "From formula", "To", "To m/z", "To formula", "Loss mass", "Loss formula", "Loss error (ppm)", "Isolation step", "Tree", "Level", "Activation", "CE", "Scans", "Relative"], rows: mods },
      nodes: { head: ["Ion", "Level", "m/z", "Sub-formula", "Error (ppm)", "Source", "Candidates"], rows: nodes },
    };
  };

  /** the graph (when the page can draw it) and its tables under `host`, one view at a time @param {HTMLElement} host @param {any} g @param {string} label */
  F.openMsnGraph = (host, g, label) => {
    const { el } = F, graph = el("div"), tables = el("div"), bar = el("div", { class: "flx-seg" });
    const show = (/** @type {boolean} */ gr) => { graph.hidden = !gr; tables.hidden = gr; for (const b of bar.children) b.setAttribute("aria-pressed", String((b.textContent === "Graph") === gr)); };
    for (const [name, gr] of /** @type {[string, boolean][]} */ ([["Graph", true], ["Tables", false]])) bar.append(el("button", { type: "button", "aria-pressed": "false", onclick: () => show(gr), text: name }));
    host.append(bar, graph, tables);
    if (F.msnGraphView) F.msnGraphView(graph, g, label);
    show(!!F.msnGraphView);
    const t = F.readMsnGraph(g);
    if (!t.modifications.rows.length) tables.append(el("p", { class: "fl-note", text: `${label} has no modifications: the graph has no neutral loss between its ions.` }));
    else F.drawTable(tables, { caption: `${label}, modifications`, head: t.modifications.head, rows: t.modifications.rows, label: `Modifications of ${label}` });
    if (t.nodes.rows.length) F.drawTable(tables, { caption: `${label}, ions`, head: t.nodes.head, rows: t.nodes.rows, label: `Ions of ${label}` });
  };

  /** the texts the MSn steps made join the Results of their step: the graph (msngrafo/1) as a report, the MSP as a library, the CSV as a table; the graph opens under the canvas. Gives the names now in the Results @param {any} S @param {{id: string, name: string, text: string}[]} files @returns {Set<string>} */
  F.webMsnAfterRun = (S, files) => {
    const shown = new Set(), type = (/** @type {any} */ f) => /\.msp$/i.test(f.name) ? "library" : /\.csv$/i.test(f.name) ? "table" : /\.json$/i.test(f.name) && /"formato"\s*:\s*"msngrafo\/1"/.test(f.text.slice(0, 200)) ? "report" : "";
    let graph = null;
    S.results = S.results || {};
    for (const f of files || []) {
      const t = type(f);
      if (!t) continue;
      const item = { type: t, label: f.name, msn: true, text: async () => f.text };
      S.results[f.id] = [...(S.results[f.id] || []).filter((/** @type {any} */ r) => r.label !== f.name), item];
      shown.add(f.name);
      if (t === "report") graph = item;
    }
    if (!shown.size) return shown;
    if (S.redraw) S.redraw();
    // a job that ends while someone types (Auto-run) leaves the graph in the Results: opening it would take the focus from the field
    const typing = document.activeElement instanceof HTMLElement && document.activeElement.matches("input, textarea, select, [contenteditable]");
    if (graph && !typing) {
      F.openResult(graph);
      setTimeout(() => { const h = document.querySelector(".flx-res h3"); if (h instanceof HTMLElement) h.focus({ preventScroll: true }); }, 0);
    }
    return shown;
  };

  // the library (MSP) and the CSV table an MSn step made (`msn` is set by F.webMsnAfterRun): a download button, and the CSV also as a table; it goes first so it wins over the generic table view
  const msnFile = { id: "msn-file", title: "MSn file", types: ["library", "table"], accepts: (/** @type {any} */ item) => item.msn === true, open: async (/** @type {HTMLElement} */ host, /** @type {any} */ item) => {
    const text = await item.text(), name = item.label, table = F.VIEWS.find((/** @type {any} */ v) => v.id === "table");
    host.append(F.btn("export", `Download ${name}`, () => {
      const a = F.el("a", { href: URL.createObjectURL(new Blob([text], { type: "text/plain" })), download: name }); a.click(); setTimeout(() => URL.revokeObjectURL(a.href), 1000);
    }, "Download"));
    if (table && /\.csv$/i.test(name)) await table.open(host, item);
  } };
  if (F.VIEWS) F.VIEWS.unshift(msnFile);
})();
