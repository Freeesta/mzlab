// @ts-check
// Flow (administrator mode): the results of a node. A VIEW is registered for the data types it can show (F.registerView); a node whose steps make such a type gets a
// «Results» block in its panel: the file the last run made (desktop) or a file chosen from disk (web). The view opens under the canvas. Views: CSV tables here; JSON and MGF results in flow-views-attributio.js.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const MAX_ROWS = 200, MAX_COLS = 30;
  /** @type {{id: string, title: string, types: string[], open: (host: HTMLElement, item: any) => Promise<void>}[]} */
  F.VIEWS = [];
  /** a view shows the files of some data types: open(host, item) fills host; item = {type, label, text: () => Promise<string>} @param {any} v */
  F.registerView = v => { F.VIEWS.push(v); };
  /** blocks added to the «Results» of a node by other files: (S, node) → an element or null @type {((S: any, node: any) => HTMLElement | null)[]} */
  F.extraResults = [];
  /** the view for a type; with the item, a view that accepts it wins, then one that takes any file of the type @param {string} type @param {any} [item] */
  F.viewFor = (type, item) => {
    const mine = F.VIEWS.filter((/** @type {any} */ v) => v.types.includes(type));
    return (item && mine.find((/** @type {any} */ v) => v.accepts && v.accepts(item))) || mine.find((/** @type {any} */ v) => !v.accepts) || mine[0];
  };

  /** a table with its caption from head and rows of strings (the first MAX_ROWS rows) @param {HTMLElement} host @param {{caption: string, head: string[], rows: string[][], label: string}} t */
  F.drawTable = (host, t) => {
    const { el } = F, body = t.rows.slice(0, MAX_ROWS);
    host.append(el("div", { class: "flx-tablewrap", tabindex: 0, role: "region", "aria-label": t.label }, el("table", { class: "flx-table" }, el("caption", { text: `${t.caption}: ${body.length} of ${t.rows.length} rows` }),
      el("thead", {}, el("tr", {}, ...t.head.map(h => el("th", { scope: "col", text: h })))),
      el("tbody", {}, ...body.map(r => el("tr", {}, ...r.map(c => el("td", { text: c }))))))));
  };

  /** CSV text to rows: quoted cells, doubled quotes, CRLF; a tab-separated file is told by its first line @param {string} text @returns {string[][]} */
  F.parseCsv = text => {
    const sep = /^[^\n,]*\t/.test(text) ? "\t" : ",", rows = /** @type {string[][]} */ ([]);
    let row = /** @type {string[]} */ ([]), cell = "", quoted = false;
    for (let i = 0; i < text.length; i++) {
      const c = text[i];
      if (quoted) { if (c === '"') { if (text[i + 1] === '"') { cell += '"'; i++; } else quoted = false; } else cell += c; }
      else if (c === '"') quoted = true;
      else if (c === sep) { row.push(cell); cell = ""; }
      else if (c === "\n" || c === "\r") { if (c === "\r" && text[i + 1] === "\n") i++; row.push(cell); rows.push(row); row = []; cell = ""; }
      else cell += c;
    }
    if (cell || row.length) { row.push(cell); rows.push(row); }
    return rows;
  };

  /** what each column holds: filled and empty cells, and the range of a column that is all numbers (the whole file, not only the rows shown) @param {string[][]} rows @returns {{name: string, filled: number, empty: number, min?: number, max?: number}[]} */
  F.tableSummary = rows => {
    const head = rows[0] || [], body = rows.slice(1);
    return head.map((name, c) => {
      let filled = 0, min = Infinity, max = -Infinity, numeric = true;
      for (const r of body) {
        const v = (r[c] || "").trim();
        if (!v) continue;
        filled++;
        const n = Number(v);
        if (Number.isFinite(n)) { if (n < min) min = n; if (n > max) max = n; } else numeric = false;
      }
      return numeric && filled ? { name, filled, empty: body.length - filled, min, max } : { name, filled, empty: body.length - filled };
    });
  };

  F.registerView({ id: "table", title: "Table", types: ["table", "annot", "tp", "candidates", "formulas", "library", "spectra"], accepts: item => /\.(csv|tsv|txt)$/i.test(item.label), open: async (host, item) => {
    const { el } = F;
    if (!/\.(csv|tsv|txt)$/i.test(item.label)) { host.append(el("p", { class: "fl-note", text: `${item.label} is not a text table, so there is nothing to show here.` })); return; }
    const rows = F.parseCsv(await item.text()), head = rows[0] || [], body = rows.slice(1, MAX_ROWS + 1), cols = Math.min(head.length, MAX_COLS);
    const table = el("table", { class: "flx-table" }, el("caption", { text: `${item.label}: ${body.length} of ${Math.max(rows.length - 1, 0)} rows, ${cols} of ${head.length} columns` }),
      el("thead", {}, el("tr", {}, ...head.slice(0, cols).map((/** @type {string} */ h) => el("th", { scope: "col", text: h })))),
      el("tbody", {}, ...body.map((/** @type {string[]} */ r) => el("tr", {}, ...r.slice(0, cols).map(c => el("td", { text: c }))))));
    const sum = F.tableSummary(rows), fmt = (/** @type {number | undefined} */ n) => n === undefined ? "" : String(Math.round(n * 1e6) / 1e6);
    const qc = el("details", { class: "flx-qc" }, el("summary", { text: `Columns: what is filled in (${sum.length})` }),
      el("div", { class: "flx-tablewrap", tabindex: 0, role: "region", "aria-label": "Column summary" }, el("table", { class: "flx-table" }, el("caption", { text: `${Math.max(rows.length - 1, 0)} rows in the file` }),
        el("thead", {}, el("tr", {}, ...["Column", "Filled", "Empty", "Lowest", "Highest"].map(h => el("th", { scope: "col", text: h })))),
        el("tbody", {}, ...sum.map(c => el("tr", {}, el("th", { scope: "row", text: c.name }), el("td", { text: String(c.filled) }), el("td", { text: String(c.empty) }), el("td", { text: fmt(c.min) }), el("td", { text: fmt(c.max) })))))));
    host.append(qc, el("div", { class: "flx-tablewrap", tabindex: 0, role: "region", "aria-label": `Table ${item.label}` }, table));
  } });

  /** shows a result under the canvas, with a way back @param {any} item */
  F.openResult = async item => {
    const root = document.getElementById("flx-root"), view = F.viewFor(item.type, item);
    if (!root || !view) return;
    root.querySelectorAll(".flx-res").forEach(n => n.remove());
    const { el } = F, host = el("div", { class: "flx-res-body" }), head = el("h3", { tabindex: -1, text: `${view.title}: ${item.label}` });
    const box = el("section", { class: "flx-res", "aria-label": "Results" }, el("div", { class: "fl-meta" }, head, F.btn("x", "Close the results", () => { box.remove(); /** @type {HTMLElement | null} */ (root.querySelector("#flx-run"))?.focus(); }, "Close")), host);
    root.append(box); head.focus({ preventScroll: false });
    try { await view.open(host, item); } catch (e) { host.append(el("p", { class: "fl-note", role: "alert", text: `The file could not be read: ${/** @type {any} */ (e) && /** @type {any} */ (e).message || e}` })); }
  };

  /** the «Results» block of a node: what the last run made, and a file picker for a file made before @param {any} S @param {any} node @returns {HTMLElement | null} */
  F.resultsCard = (S, node) => {
    const { el } = F, mine = /** @type {{id: string, port: any}[]} */ ([]);
    for (const id of node.ids || []) {
      const inst = F.instanceOf(S.plan, id), step = inst && (F.stepOf ? F.stepOf(S.reg, inst) : S.reg.byId.get(inst.step.split("@")[0]));
      for (const p of step ? F.ports(step.out) : []) if (F.viewFor(p.type)) mine.push({ id, port: p });
    }
    if (!mine.length) return null;
    const box = el("div", { class: "fl-results" }, el("h4", { text: "Results" }));
    S.results = S.results || {};
    for (const { id, port } of mine) {
      const made = (S.results[id] || []).filter((/** @type {any} */ r) => r.type === port.type);
      for (const r of made) box.append(F.btn("file", `Open ${r.label}`, () => F.openResult(r), r.label));
      const pick = el("input", { type: "file", hidden: true, accept: ".csv,.tsv,.txt,.json,.mgf,.msp", "aria-label": `Choose a ${port.title} file`, onchange: (/** @type {any} */ e) => {
        const f = e.target.files && e.target.files[0]; if (!f) return;
        const item = { type: port.type, label: f.name, text: () => f.text() };
        S.results[id] = [...(S.results[id] || []).filter((/** @type {any} */ r) => r.label !== f.name), item];
        F.openResult(item); if (S.redraw) S.redraw();
      } });
      box.append(pick, F.btn("import", `Open a ${port.title} file from disk`, () => pick.click(), `Open a ${port.title.toLowerCase()} file…`));
    }
    for (const f of F.extraResults) { const b = f(S, node); if (b) box.append(b); }
    return box;
  };
})();
