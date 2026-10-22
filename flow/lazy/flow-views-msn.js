// @ts-check
// Flow (administrator mode): the result view of the step spectra.msn_graph, the fragmentation graph msngrafo/1 of an MSn infusion. Its tables are the modifications
// (neutral losses between ions, with the formulas of both ions) and the ions with their proposed sub-formulas, the same columns as the CSV of msn.table_export.
// flow-report.js hands a report JSON of this schema to F.openMsnGraph.
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

  /** the tables of a graph under `host` @param {HTMLElement} host @param {any} g @param {string} label */
  F.openMsnGraph = (host, g, label) => {
    const t = F.readMsnGraph(g);
    if (!t.modifications.rows.length) host.append(F.el("p", { class: "fl-note", text: `${label} has no modifications: the graph has no neutral loss between its ions.` }));
    else F.drawTable(host, { caption: `${label}, modifications`, head: t.modifications.head, rows: t.modifications.rows, label: `Modifications of ${label}` });
    if (t.nodes.rows.length) F.drawTable(host, { caption: `${label}, ions`, head: t.nodes.head, rows: t.nodes.rows, label: `Ions of ${label}` });
  };
})();
