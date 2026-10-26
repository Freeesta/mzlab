// @ts-check
// Flow (administrator mode): the inline editor of a parameter that is a list of the user (`x-liste` in the descriptor, liste/1): a table of rows to add, change and remove,
// import from a CSV and export as JSON, with the SHA-256 of the list. The desktop engine checks the rows (route `flow/liste`); the page keeps no parsing of its own.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const COLS = { adducts: ["name", "delta", "charge", "comment"], transformations: ["name", "delta", "max_repeat", "comment"] };
  const HEAD = { name: "Name", delta: "Change of formula", charge: "Charge", max_repeat: "Max repeats", comment: "Comment" };
  const CSS = `.fl-list{display:flex;flex-direction:column;gap:6px;min-width:0}.fl-list table{border-collapse:collapse;width:100%}.fl-list th{text-align:left;font-weight:600;color:var(--muted);padding:2px 4px}
.fl-list td{padding:2px 4px}.fl-list td input{min-height:28px;width:100%;min-width:56px;box-sizing:border-box}.fl-list .fl-b{min-width:24px;min-height:28px}.fl-list-sha{font-family:monospace;font-size:12px;overflow-wrap:anywhere;color:var(--muted)}
.fl-list-err{color:var(--bad,var(--ink));margin:0;padding-left:16px}@media (pointer:coarse){.fl-list td input,.fl-list .fl-b{min-height:40px;min-width:40px}}`;

  /** the editor, or null when the engine of the app is not there (the plain text field stays) @param {any} p descriptor of the parameter @param {any} cur its value (names, or the CSV of the list) @param {(v: any) => void} set @param {string} id */
  F.listeEditor = (p, cur, set, id) => {
    const app = /** @type {any} */ (window).MZLAB_DESKTOP, kind = /** @type {"adducts" | "transformations"} */ (p["x-liste"]), cols = COLS[kind];
    if (!app || !cols) return null;
    const { el } = F;
    if (!document.getElementById("fl-list-css")) { const s = document.createElement("style"); s.id = "fl-list-css"; s.textContent = CSS; document.head.append(s); }
    const text = typeof cur === "string" ? cur : "", own = /^\s*name\b/i.test(text);
    const box = el("div", { class: "fl-list", id });
    // the runner of the app is a late module: wait for it before the first call
    const ask = async (/** @type {any} */ body) => { if (!F.desktopPost && F.whenLate) await F.whenLate; return F.desktopPost("flow/liste", { kind, ...body }); };
    const parsed = (/** @type {any} */ body) => ask(body).then((/** @type {string} */ t) => JSON.parse(t));
    if (!own) {
      box.append(el("span", { class: "fl-note", text: text || "(none)" }), F.btn("plus", `Use my own list for ${p.title}`, () => parsed({ rows: [{ name: "[M+H]+", delta: "+H", charge: "1", comment: "" }] }).then((/** @type {any} */ a) => set(a.csv)), "Use my own list"));
      return box;
    }
    const body = el("tbody"), sha = el("span", { class: "fl-list-sha", "data-fk": id + "-sha" }), errs = el("ul", { class: "fl-list-err", role: "alert", "data-fk": id + "-err" });
    /** @param {any} r */
    const addRow = (r = {}) => {
      const tr = el("tr");
      for (const c of cols) tr.append(el("td", {}, el("input", { type: "text", value: r[c] ?? "", "aria-label": `${HEAD[/** @type {keyof typeof HEAD} */ (c)]}, row ${body.children.length + 1}`, "data-col": c, onchange: commit })));
      tr.append(el("td", {}, F.btn("x", `Remove row ${body.children.length + 1} of ${p.title}`, () => { tr.remove(); commit(); })));
      body.append(tr);
    };
    const rows = () => [...body.children].map(tr => Object.fromEntries([...tr.querySelectorAll("input")].map(i => [i.dataset.col, i.value])));
    /** @param {any} a */
    const show = a => {
      errs.replaceChildren();
      if (a.errors) { for (const e of a.errors) errs.append(el("li", { text: `Row ${e.line - 1}: ${e.key.replace("error.liste.", "")} ${e.detail}`.trim() })); return false; }
      sha.textContent = `SHA-256 ${a.sha256}`; sha.dataset.sha = a.sha256; box.dataset.json = a.json; return true;
    };
    function commit() { parsed({ rows: rows() }).then((/** @type {any} */ a) => { if (show(a)) set(a.csv); }); }
    const file = el("input", { type: "file", accept: ".csv,.txt,text/csv,text/plain", hidden: true, "aria-label": `Import a CSV into ${p.title}`, onchange: async () => {
      const f = file.files && file.files[0]; if (!f) return;
      const a = await parsed({ csv: await f.text() }); file.value = "";
      if (show(a)) { body.replaceChildren(); a.rows.forEach(addRow); set(a.csv); }
    } });
    const save = () => { const a = el("a", { href: URL.createObjectURL(new Blob([box.dataset.json || ""], { type: "application/json" })), download: `${kind}.liste.json` }); a.click(); setTimeout(() => URL.revokeObjectURL(a.href), 1000); };
    const head = el("tr", {}, ...cols.map(c => el("th", { scope: "col", text: HEAD[/** @type {keyof typeof HEAD} */ (c)] })), el("th", { scope: "col", text: "" }));
    box.append(el("table", { "aria-label": p.title }, el("thead", {}, head), body), errs,
      el("div", { class: "fl-meta" }, F.btn("plus", `Add a row to ${p.title}`, () => { addRow(); }, "Add row"), F.btn("import", `Import a CSV into ${p.title}`, () => file.click(), "Import CSV"), file,
        F.btn("export", `Export ${p.title} as JSON`, save, "Export JSON")), sha);
    parsed({ csv: text }).then((/** @type {any} */ a) => { if (show(a)) a.rows.forEach(addRow); });
    return box;
  };
})();
