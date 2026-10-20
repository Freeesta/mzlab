// @ts-check
// Flow (administrator mode): result views of the transformation-product steps. The JSON a step or Tabui writes becomes tables: tptab/1 (products, and the conditions
// of gruppi/1 with a bar chart of one product), interp/1 (the reading of one row) and reatt/1 (the reactivity of the parent). Pure readers (F.read*) then views.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const txt = (/** @type {any} */ v) => v === null || v === undefined ? "" : String(v);
  const fx = (/** @type {any} */ n, d = 4) => n === null || n === undefined || n === "" || !Number.isFinite(Number(n)) ? "" : String(Math.round(Number(n) * 10 ** d) / 10 ** d);
  const STATUS = ["Product", "Control", "In-source fragment", "Formed, not explained", "Parent"], KIN = ["None", "Doubtful", "Possible", "Probable", "Certain"];
  /** the bits of the «via» byte: rule 1, cleavage 2, imported 4, MS2 8, network 16 */
  const VIA = /** @type {[number, string][]} */ ([[1, "rule"], [2, "cleavage"], [4, "imported"], [8, "MS2"], [16, "network"]]);
  const word = (/** @type {string[]} */ list, /** @type {any} */ n) => n === null || n === undefined ? "" : list[n] || String(n);

  /** tptab/1 → products table @param {any} j @returns {{head: string[], rows: string[][]}} */
  F.readTptab = j => ({
    head: ["Row", "ID", "m/z", "RT", "Status", "Kinship", "Explained by", "Files detected", "Largest area", "Time of maximum", "In time 0", "In blank"],
    rows: (j.rows || []).map((/** @type {any} */ r) => [txt(r.row), txt(r.id), fx(r.mz, 4), fx(r.rt, 2), word(STATUS, r.status), word(KIN, r.kinship),
      r.via ? VIA.filter(([b]) => r.via & b).map(x => x[1]).join(", ") : "", txt(r.n_det), fx(r.max_area, 0), fx(r.tmax, 1),
      r.in_t0 ? `yes (×${fx(r.t0_ratio, 2)})` : "no", r.in_blank ? `yes (×${fx(r.blank_ratio, 2)})` : "no"]),
  });

  /** gruppi/1 → the presence matrix (one row per product, one column per condition), the sets, the parent's kinetics @param {any} g */
  F.readGruppi = g => {
    const ids = (g.conditions || []).map((/** @type {any} */ c) => c.id);
    const cell = (/** @type {any} */ c) => !c.present ? "absent" : `present, area ${fx(c.area, 0)}${c.ratio_ref !== null && c.ratio_ref !== undefined && c.ratio_ref !== 1 ? `, ×${fx(c.ratio_ref, 2)} of the reference` : ""}`;
    return {
      matrix: { head: ["Product", "m/z", "RT", "Neutral formula", ...ids], rows: (g.rows || []).map((/** @type {any} */ r) => [txt(r.tp_name), fx(r.mz, 4), fx(r.rt, 2), txt(r.neutral_formula), ...(r.cond || []).map(cell)]) },
      sets: { head: ["Conditions", "Products"], rows: (g.sets || []).map((/** @type {any} */ s) => [(s.conditions || []).join(" + "), String((s.rows || []).length)]) },
      parent: { head: ["Condition", "Rate constant k (1/min)", "Half-life (min)", "Share left"], rows: (g.conditions || []).map((/** @type {any} */ c) => [c.id, fx(c.parent && c.parent.k, 5), fx(c.parent && c.parent.half_life, 1), fx(c.parent && c.parent.remaining, 3)]) },
      ids,
    };
  };

  /** interp/1 → a list of «what, value» rows and the candidate classes of the site @param {any} j */
  F.readInterp = j => {
    const kv = /** @type {string[][]} */ ([]), add = (/** @type {string} */ k, /** @type {any} */ v) => { if (v !== "" && v !== null && v !== undefined) kv.push([k, String(v)]); };
    const ch = j.change || {}, ms2 = j.ms2 || {}, scans = ms2.scans || [], vp = j.vs_parent || {}, site = j.site || {}, p = j.parent || {};
    add("Row", j.id); add("Parent", [p.ion_formula, p.mz !== undefined ? `m/z ${fx(p.mz, 4)}` : ""].filter(Boolean).join(", "));
    add("Change", ch.delta); add("Product ion formula", ch.ion_formula); add("Mass error (ppm)", fx(ch.ppm, 2));
    add("Other changes considered", (ch.alternatives || []).map((/** @type {any} */ a) => a.delta).join(", "));
    add("MS2 scans", scans.length); add("Files with MS2", ms2.files_with_ms2); add("Mixed spectra left out", scans.filter((/** @type {any} */ s) => s.chimeric).length || "");
    add("Modified cosine to the parent", fx(vp.cos_mod, 3)); add("Spectral entropy similarity", fx(vp.entropy, 3));
    add("Fragments shared with the parent", (vp.equal || []).length || ""); add("Fragments shifted by the change", (vp.shifted || []).length || "");
    add("Site: change", site.transform); add("Site: confidence", txt(site.confidence)); add("Site: informative fragments", site.informative); add("Site: margin", fx(site.margin, 2));
    add("Row hidden by", j.hidden);
    return {
      summary: { head: ["What", "Value"], rows: kv },
      classes: { head: ["Class", "Kind", "Status", "Group", "Support", "For", "Against"], rows: (site.classes || []).map((/** @type {any} */ c) => [txt(c.id), txt(c.kind), txt(c.status), txt(c.group), fx(c.support, 3), txt(c.n_support), txt(c.n_against)]) },
    };
  };

  /** reatt/1 → one row per atom: the Fukui indices and the charge @param {any} j @returns {{head: string[], rows: string[][]}} */
  F.readReatt = j => ({
    head: ["Atom", "Element", "Heavy-atom index", "Charge", "f(+)", "f(0)", "f(−)", "Dual"],
    rows: (j.atoms || []).map((/** @type {any} */ a) => [txt(a.i), txt(a.el), txt(a.heavy_index), fx(a.q, 3), fx(a.f_plus, 3), fx(a.f_zero, 3), fx(a.f_minus, 3), fx(a.dual, 3)]),
  });

  /** bars of one number per condition: the data table sits under it (the caller) @param {{id: string, v: number}[]} items @param {string} label */
  F.condBars = (items, label) => {
    const ns = "http://www.w3.org/2000/svg", W = 480, H = 160, L = 10, B = 40, T = 8, svg = document.createElementNS(ns, "svg"), top = Math.max(...items.map(i => i.v), 0) || 1;
    svg.setAttribute("viewBox", `0 0 ${W} ${H}`); svg.setAttribute("class", "flx-sticks"); svg.setAttribute("role", "img"); svg.setAttribute("aria-label", label);
    const step = (W - L * 2) / Math.max(items.length, 1), bw = Math.min(48, step * 0.6);
    items.forEach((it, k) => {
      const h = (H - T - B) * it.v / top, x = L + step * k + (step - bw) / 2, r = document.createElementNS(ns, "rect");
      r.setAttribute("x", String(x)); r.setAttribute("y", String(H - B - h)); r.setAttribute("width", String(bw)); r.setAttribute("height", String(Math.max(h, 0)));
      r.setAttribute("class", "flx-bar"); r.setAttribute("style", `fill:var(--gv${(k % 8) + 1}, var(--accent))`);
      const t = document.createElementNS(ns, "text"); t.setAttribute("x", String(x + bw / 2)); t.setAttribute("y", String(H - B + 14)); t.setAttribute("text-anchor", "middle"); t.setAttribute("class", "flx-tick"); t.textContent = it.id;
      svg.append(r, t);
    });
    return svg;
  };

  /** @param {HTMLElement} host @param {string} caption @param {{head: string[], rows: string[][]}} t @param {string} label */
  const table = (host, caption, t, label) => t.rows.length && F.drawTable(host, { caption, head: t.head, rows: t.rows, label });

  /** the conditions block of a tptab/1: matrix, a chart of the area of one product per condition with its table, the sets and the parent @param {HTMLElement} host @param {any} g @param {string} name */
  const conditions = (host, g, name) => {
    const { el } = F, d = F.readGruppi(g);
    table(host, `${name}, conditions`, d.matrix, `Conditions of ${name}`);
    if (g.rows && g.rows.length && d.ids.length) {
      const show = el("div", { class: "flx-one" }), pick = /** @type {HTMLSelectElement} */ (el("select", { "aria-label": "Product to draw", onchange: () => draw(Number(pick.value)) }, ...g.rows.slice(0, 300).map((/** @type {any} */ r, /** @type {number} */ i) => el("option", { value: String(i), text: `${r.tp_name} (m/z ${fx(r.mz, 4)})` }))));
      /** @param {number} i */
      const draw = i => {
        const r = g.rows[i], items = d.ids.map((/** @type {string} */ id, /** @type {number} */ k) => ({ id, v: Number(r.cond && r.cond[k] && r.cond[k].area) || 0 }));
        show.replaceChildren(F.condBars(items, `Area of ${r.tp_name} in each condition`));
        F.drawTable(show, { caption: `Area of ${r.tp_name}`, head: ["Condition", "Area"], rows: items.map((/** @type {any} */ x) => [x.id, fx(x.v, 0)]), label: `Area of ${r.tp_name} per condition` });
      };
      host.append(el("label", { class: "fl-note" }, "Product ", pick), show); draw(0);
    }
    table(host, `${name}, sets`, d.sets, `Sets of ${name}`); table(host, `${name}, parent`, d.parent, `Parent of ${name}`);
  };

  /** JSON → the view of its format; a wrong shape says so @param {string} text @param {string} label @param {HTMLElement} host */
  const open = (text, label, host) => {
    const { el } = F, j = JSON.parse(text), kind = j.schema || j.format;
    if (kind === "tptab/1") {
      if (!j.rows || !j.rows.length) { host.append(el("p", { class: "fl-note", text: `${label} has no products to show.` })); return; }
      table(host, label, F.readTptab(j), `Table ${label}`);
      if (j.gruppi) conditions(host, j.gruppi, label);
    } else if (kind === "gruppi/1") conditions(host, j, label);
    else if (kind === "interp/1") { const d = F.readInterp(j); table(host, label, d.summary, `Reading of ${label}`); table(host, `${label}, site classes`, d.classes, `Site classes of ${label}`); }
    else if (kind === "reatt/1" || j.atoms) table(host, label, F.readReatt(j), `Table ${label}`);
    else host.append(el("p", { class: "fl-note", text: `${label} is not a file this view knows (tptab/1, gruppi/1, interp/1 or reatt/1).` }));
  };
  F.openTabui = open;
  F.registerView({ id: "tp", title: "Products", types: ["tp"], accepts: (/** @type {any} */ item) => /\.json$/i.test(item.label), open: async (/** @type {HTMLElement} */ host, /** @type {any} */ item) => open(await item.text(), item.label, host) });
})();
