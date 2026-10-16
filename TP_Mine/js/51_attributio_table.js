// @ts-check
// TPMINE-PRIVATE  Table of levels of Attributio (admin mode): `ATTRIBUTIO_TABLE.mount(box, data, hooks)` draws the rows of the engine (`{columns, rows}`) with TPTAB (42_tp_table.js): a closed row with the level
// (with its sublevel, «L2b»), the name, m/z, RT and polarity, chips of level and polarity, sort from the headers, "Columns", "Export view"; a click opens the row on the spot with the basis and the reasons of the
// level (tab Identity) and the button that takes the row to the Banco 2b. Pure DOM, no engine: the colours of the levels are the `.lvb.lv-*` classes of the page (CSS variables).
// Texts in English, without keys of lang/ (the exception of mzFinder); the helps are keys of the catalogue of Tabui (`TT`).
(() => {
  "use strict";
  const G = /** @type {any} */ (window);
  const esc = (/** @type {any} */ s) => String(s ?? "").replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c] || c));
  const LEVELS = ["1", "2a", "2b", "3a", "3b", "4", "0"];
  /** the order of the levels, best first (L0 = detected without annotation is the last) */
  const RANK = /** @type {Record<string, number>} */ ({ "1": 0, "2a": 1, "2b": 2, "3a": 3, "3b": 4, "4": 5, "0": 6 });
  /** @typedef {{ i: number, pol: string, id: string, mz: number | null, rt: number | null, key: string, basis: string, name: string, identity: string, reasons: string }} Row */

  /** the level of a row as the key of the profile («2a», «3b», «4», «0») @param {any[]} r */
  const key = r => String(r[4]) + (r[5] || "");
  /** @param {any[][]} rows @returns {Record<string, number>} */
  function counts(rows) {
    /** @type {Record<string, number>} */
    const c = {};
    for (const r of rows) c[key(r)] = (c[key(r)] || 0) + 1;
    return c;
  }
  /** the line of text above the table: «15 features · L1 1 · L2a 3 …» @param {any[][]} rows */
  function summary(rows) {
    const c = counts(rows);
    return `${rows.length} features` + LEVELS.filter(l => c[l]).map(l => ` · L${l} ${c[l]}`).join("");
  }
  /** the model of a row of the engine (`columns` of the table: pol, id, mz, rt, level, sub, basis, name, identity, reasons) @param {any[]} r @param {number} i @returns {Row} */
  const model = (r, i) => ({ i, pol: String(r[0] ?? ""), id: String(r[1] ?? ""), mz: r[2] == null ? null : Number(r[2]), rt: r[3] == null ? null : Number(r[3]), key: key(r), basis: String(r[6] ?? ""), name: String(r[7] ?? ""), identity: String(r[8] ?? ""), reasons: String(r[9] ?? "") });
  const fix = (/** @type {number | null} */ v, /** @type {number} */ d) => v == null || !Number.isFinite(v) ? "" : v.toFixed(d);
  /** what a row is called: the name, else the identity, else its m/z @param {Row} r */
  const called = r => r.name || r.identity || (r.mz == null ? "no name" : "m/z " + fix(r.mz, 4));

  /** @param {HTMLElement} box @param {{columns: string[], rows: any[][]}} data @param {{ onReview?: (list: {pol: string, id: number}[], index: number) => void }} [hooks] the Banco 2b opens from the button of the open row */
  function mount(box, data, hooks = {}) {
    const TPTAB = G.TPTAB;
    /** @type {Row[]} */
    const rows = data.rows.map(model);
    box.innerHTML = `<p id="at-counts" role="status"></p><div id="at-tbl"></div>`;
    /** @type {HTMLElement} */ (box.querySelector("#at-counts")).textContent = summary(data.rows);
    const lvl = (/** @type {Row} */ r) => TPTAB.level(r.key);
    /** @type {{ key: string, label: string, help?: string, test: (r: Row) => boolean }[]} */
    const chips = [
      ...[["l2", "L2 or better", "2b"], ["l3", "L3 or better", "3b"], ["l4", "L4 or better", "4"]].map(([k, label, to]) => ({ key: k, label, help: "ft.chip.level", test: (/** @type {Row} */ r) => RANK[r.key] <= RANK[to] })),
      { key: "pos", label: "POS", help: "ft.chip.pol", test: r => r.pol === "POS" }, { key: "neg", label: "NEG", help: "ft.chip.pol", test: r => r.pol === "NEG" },
    ];
    const cols = [
      { key: "level", label: "Level", help: "ft.col.level", sticky: true, sort: (/** @type {Row} */ r) => RANK[r.key] ?? 9, cell: lvl, text: (/** @type {Row} */ r) => "L" + r.key },
      { key: "identity", label: "Name", help: "ft.col.identity", sticky: true, sort: (/** @type {Row} */ r) => called(r).toLowerCase(), cell: (/** @type {Row} */ r) => esc(called(r)), text: called },
      { key: "mz", label: "m/z", help: "ft.col.mz", num: true, sort: (/** @type {Row} */ r) => r.mz, cell: (/** @type {Row} */ r) => fix(r.mz, 4) },
      { key: "rt", label: "RT", help: "ft.col.rt", num: true, sort: (/** @type {Row} */ r) => r.rt, cell: (/** @type {Row} */ r) => fix(r.rt, 2) },
      { key: "pol", label: "Pol", help: "ft.col.pol", sort: (/** @type {Row} */ r) => r.pol, cell: (/** @type {Row} */ r) => esc(r.pol) },
      { key: "basis", off: true, label: "Basis", help: "ft.col.basis", sort: (/** @type {Row} */ r) => r.basis.toLowerCase(), cell: (/** @type {Row} */ r) => esc(r.basis) },
      { key: "ident", off: true, label: "Identity", help: "ft.col.identity", sort: (/** @type {Row} */ r) => r.identity.toLowerCase(), cell: (/** @type {Row} */ r) => esc(r.identity) },
      { key: "reasons", off: true, label: "Reasons", help: "ft.col.reasons", cell: (/** @type {Row} */ r) => esc(r.reasons.split("\n")[0]) },
      { key: "id", off: true, label: "ID", help: "ft.col.id", num: true, sort: (/** @type {Row} */ r) => Number(r.id), cell: (/** @type {Row} */ r) => esc(r.id) },
    ];
    /** the head of the open row: level, the words (the first reason) and the button of the Banco 2b @param {Row} r @param {HTMLElement} h */
    const head = (r, h) => {
      const why = r.reasons.split("\n")[0];
      h.innerHTML = `<span class="tpr-head">${lvl(r)}</span>${why ? `<span class="tpr-words">${esc(why)}</span>` : ""}${hooks.onReview ? `<button type="button" class="tpr-act" data-act="banco"${G.TPHELP ? ` data-tt="ft.go.banco"` : ""}>Open in Bench 2b</button>` : ""}`;
      const b = h.querySelector("button[data-act=banco]");
      if (b) b.addEventListener("click", () => {
        const list = T.view.map((/** @type {Row} */ x) => ({ pol: x.pol, id: Number(x.id) })), at = T.view.findIndex((/** @type {Row} */ x) => x.i === r.i);
        if (hooks.onReview) hooks.onReview(list, Math.max(0, at));
      });
    };
    /** the tab Identity: the basis and the reasons of the level, in words @param {Row} r @param {HTMLElement} body */
    const fill = (r, body) => {
      const why = r.reasons.split("\n").filter(Boolean);
      body.innerHTML = `<dl class="tpr-q"><dt>Level</dt><dd>L${esc(r.key)}</dd><dt>Basis</dt><dd>${esc(r.basis || "–")}</dd><dt>Name</dt><dd>${esc(r.name || "–")}</dd><dt>Identity</dt><dd>${esc(r.identity || "–")}</dd>`
        + `<dt><i>m/z</i> · RT · polarity</dt><dd>${esc(fix(r.mz, 4) || "–")} · ${esc(fix(r.rt, 2) || "–")} min · ${esc(r.pol)}</dd></dl>`
        + (why.length ? `<p class="tpr-id"><b>Reasons</b></p><ul class="tpr-pf">${why.map(x => `<li><span>${esc(x)}</span></li>`).join("")}</ul>` : `<p class="tpr-none">Reasons: none given for this row.</p>`);
    };
    const T = TPTAB.mount(/** @type {HTMLElement} */ (box.querySelector("#at-tbl")), {
      id: "at-table", label: "Features with their level of identification", panelId: "at-det", cols, rowId: (/** @type {Row} */ r) => r.i, esc,
      name: (/** @type {Row} */ r) => `${r.pol} ${r.id}: ${called(r)}`, columns: true, chips, search: (/** @type {Row} */ r) => `${r.pol} ${r.id} ${r.name} ${r.identity} ${r.basis} L${r.key}`, exportName: "attributio_view", height: 560,
      tabs: [{ key: "identity", label: "Identity", help: "ft.tab.identity" }], head, onOpen: (/** @type {Row} */ r, /** @type {HTMLElement} */ body) => fill(r, body),
    });
    T.setRows(rows);
    return T;
  }
  G.ATTRIBUTIO_TABLE = { mount, summary, counts, key };
})();
