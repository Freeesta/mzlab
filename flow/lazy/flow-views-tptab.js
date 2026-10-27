// @ts-check
// Flow (administrator mode): the detail of a tptab/1, the columns of the Tabui table that flow-views-tabui.js leaves out: the kinetics (areas along the treatment, with a line
// chart of one product and its data table), the notes, the formula candidates and the conditions where the product is present. Pure reader (F.readTptabDetail) then the view.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const fx = (/** @type {any} */ n, d = 4) => n === null || n === undefined || n === "" || !Number.isFinite(Number(n)) ? "" : String(Math.round(Number(n) * 10 ** d) / 10 ** d);
  /** the bits of the «notes» byte (crates/mzlab-feat/src/tp/mod.rs) */
  const NOTES = /** @type {[number, string][]} */ ([[1, "in time 0"], [2, "in-source fragment coelutes"], [4, "in-source fragment of a product"], [8, "mixed MS2"], [16, "taken out by the blank"], [32, "taken out by time 0"], [64, "forms in the dark too"], [128, "no time 0"]]);
  const SHOWN = 3;

  /** tptab/1 → the detail table (one row per product) @param {any} j @returns {{head: string[], rows: string[][], ids: any[]}} */
  F.readTptabDetail = j => {
    const times = j.times || [], g = j.gruppi, cond = (/** @type {any} */ r) => g ? (g.rows || []).filter((/** @type {any} */ x) => x.row === r.row).flatMap((/** @type {any} */ x) => (x.cond || []).map((/** @type {any} */ c, /** @type {number} */ k) => c.present ? g.conditions[k].id : "").filter(Boolean)).join(", ") : "";
    return {
      head: ["Row", "Areas by time", "Notes", "Formula candidates", "Conditions"],
      rows: (j.rows || []).map((/** @type {any} */ r) => [String(r.row),
        (r.series || []).map((/** @type {number} */ a, /** @type {number} */ i) => `${fx(times[i], 1)}: ${fx(a, 0)}`).join("; "),
        r.notes ? NOTES.filter(([b]) => r.notes & b).map(x => x[1]).join(", ") : "",
        (r.fit_candidates || []).slice(0, SHOWN).map((/** @type {any} */ c) => `${c.neutral_formula} (${fx(c.ppm, 1)} ppm, ${fx(c.total, 2)})`).join("; "), cond(r)]),
      ids: (j.rows || []).map((/** @type {any} */ r) => r.row),
    };
  };

  /** a line of area against time (one product) @param {number[]} xs @param {number[]} ys @param {string} label */
  const line = (xs, ys, label) => {
    const ns = "http://www.w3.org/2000/svg", W = 480, H = 160, L = 10, B = 24, T = 8, svg = document.createElementNS(ns, "svg");
    const top = Math.max(...ys, 0) || 1, x1 = Math.max(...xs, 0) || 1, at = (/** @type {number} */ x, /** @type {number} */ y) => `${L + (W - 2 * L) * x / x1},${H - B - (H - T - B) * y / top}`;
    svg.setAttribute("viewBox", `0 0 ${W} ${H}`); svg.setAttribute("class", "flx-sticks"); svg.setAttribute("role", "img"); svg.setAttribute("aria-label", label);
    const path = document.createElementNS(ns, "polyline");
    path.setAttribute("points", xs.map((x, i) => at(x, ys[i])).join(" ")); path.setAttribute("fill", "none"); path.setAttribute("style", "stroke:var(--gv1, var(--accent));stroke-width:2");
    svg.append(path);
    return svg;
  };

  /** the detail table and the kinetics of one product (a picker; the row picked in another view is the one drawn) @param {HTMLElement} host @param {any} j @param {string} label */
  F.tptabDetail = (host, j, label) => {
    const { el } = F, rows = (j.rows || []).slice(0, 300), times = j.times || [];
    const d = F.readTptabDetail(j);
    F.drawTable(host, { caption: `${label}, detail`, head: d.head, rows: d.rows, label: `Detail of ${label}`, ids: d.ids });
    if (rows.length && j.gruppi && j.gruppi.conditions && j.gruppi.conditions.length > 1 && /** @type {any} */ (globalThis).TPLAZY) {
      // several conditions: one curve per condition (the Tabui chart, with its data table), not the areas of all the files in one line
      const box = el("div", { class: "flx-one" }); host.append(box);
      const G = /** @type {any} */ (globalThis);
      G.TPLAZY.load("gruppi-vista.js").then(() => {
        if (!box.isConnected) return;
        const v = G.TPGRUPPI_VIEW.kinetics(box, j.gruppi, { parentName: label });
        const stop = F.onSelect((/** @type {string | null} */ id, /** @type {string} */ from) => {
          if (!box.isConnected) { stop(); return; }
          if (from !== "kinetics" && id != null && Number.isFinite(Number(id))) v.selectRow(Number(id));
        });
      }).catch(() => { box.textContent = "The kinetics by condition could not be loaded."; });
      return;
    }
    if (!rows.length || !times.length) return;
    const show = el("div", { class: "flx-one" }), pick = /** @type {HTMLSelectElement} */ (el("select", { "aria-label": "Product to draw", onchange: () => { draw(Number(pick.value)); F.select(String(rows[Number(pick.value)].row), "kinetics"); } }, ...rows.map((/** @type {any} */ r, /** @type {number} */ i) => el("option", { value: String(i), text: `${r.id} (m/z ${fx(r.mz, 4)})` }))));
    /** @param {number} i */
    const draw = i => {
      const r = rows[i], ys = (r.series || []).map(Number);
      show.replaceChildren(line(times.map(Number), ys, `Area of ${r.id} along the treatment`));
      F.drawTable(show, { caption: `Area of ${r.id}`, head: ["Time (min)", "Area"], rows: times.map((/** @type {number} */ t, /** @type {number} */ k) => [fx(t, 1), fx(ys[k], 0)]), label: `Area of ${r.id} by time` });
    };
    host.append(el("label", { class: "fl-note" }, "Product ", pick), show); draw(0);
    const stop = F.onSelect((/** @type {string | null} */ id, /** @type {string} */ from) => {
      if (!host.isConnected) { stop(); return; }
      const at = rows.findIndex((/** @type {any} */ r) => String(r.row) === id);
      if (from !== "kinetics" && at >= 0) { pick.value = String(at); draw(at); }
    });
  };
})();
