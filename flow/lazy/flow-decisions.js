// @ts-check
// Flow (administrator mode): the decisions of the chemist for `tp.review`: an input file of type `transform` written in place (tpdec/1: row key → level and note), kept in memory
// (S.samplesJson[input name]) with its SHA-256 like the parent, and the view of the `review/1` the step writes. The rules that allow a level are the step's (Rust); the list here only fills the menu.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  const LEVELS = ["1", "2a", "2b", "3a", "3b", "4", "5", "0"], NOTE_MAX = 200, EMPTY = { schema: "tpdec/1", rows: {} };
  const txt = (/** @type {any} */ v) => v === null || v === undefined ? "" : String(v);

  /** the rows of a tpdec/1 text, as { key: {l, n} }; throws a message for a text that is not that @param {string} text */
  const parse = text => {
    const j = JSON.parse(text);
    if (!j || j.schema !== "tpdec/1" || typeof j.rows !== "object" || !j.rows) throw new Error("Not a tpdec/1 file: it needs {\"schema\": \"tpdec/1\", \"rows\": {...}}.");
    return /** @type {Record<string, {l: string, n: string}>} */ (j.rows);
  };

  /** the decisions of the input `name` @param {any} S @param {string} name */
  const rowsOf = (S, name) => { try { return parse((S.samplesJson && S.samplesJson[name]) || JSON.stringify(EMPTY)); } catch (_) { return {}; } };

  /** keeps the decisions as the content of the input and its SHA-256; the keys are sorted so that the same decisions give the same bytes @param {any} S @param {any} input @param {Record<string, {l: string, n: string}>} rows */
  F.decisionsSet = async (S, input, rows) => {
    const sorted = Object.fromEntries(Object.keys(rows).sort().map(k => [k, { l: rows[k].l || "", n: String(rows[k].n || "").slice(0, NOTE_MAX) }]));
    const text = JSON.stringify({ schema: "tpdec/1", rows: sorted });
    S.samplesJson = S.samplesJson || {}; S.samplesJson[input.name] = text;
    input.sha256 = [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text)))].map(b => b.toString(16).padStart(2, "0")).join("");
  };

  /** the inline editor of the decisions input @param {any} S @param {any} input */
  F.decisionsEditor = (S, input) => {
    const { el } = F, box = el("div", { class: "flx-smp" }), say = el("p", { class: "fl-note", role: "status" });
    /** @param {Record<string, {l: string, n: string}>} rows */
    const save = async rows => { await F.decisionsSet(S, input, rows); draw(); if (S.redraw) S.redraw(); };
    const draw = () => {
      box.textContent = "";
      const rows = rowsOf(S, input.name), keys = Object.keys(rows).sort();
      const file = /** @type {HTMLInputElement} */ (el("input", { type: "file", accept: ".json,application/json", hidden: true, "aria-label": "Decisions file (tpdec/1)", onchange: async (/** @type {any} */ e) => {
        const f = e.target.files && e.target.files[0]; if (!f) return;
        try { await save(parse(await f.text())); } catch (err) { say.textContent = String((/** @type {any} */ (err)).message || err); }
      } }));
      box.append(el("div", { class: "fl-meta" },
        F.btn("plus", "Add a decision for a row", () => { let n = 1; while (rows["0.0000@" + n.toFixed(2)]) n++; void save({ ...rows, ["0.0000@" + n.toFixed(2)]: { l: "", n: "" } }); }, "Add a decision"),
        F.btn("import", "Read the decisions from a tpdec/1 file", () => file.click(), "Import"),
        F.btn("export", "Download the decisions as JSON", () => { const j = S.samplesJson && S.samplesJson[input.name]; if (j) G.ATTRIBUTIO_CORE.download(input.name, j, "application/json"); }, "Download"),
        el("span", { class: "fl-note", text: "SHA-256 " }), el("code", { class: "flx-smp-sha", title: input.sha256 || "", text: input.sha256 ? input.sha256.slice(0, 12) + "…" : "not saved yet" }), file), say);
      if (!keys.length) { box.append(el("p", { class: "fl-note", text: "No decisions yet. A row is named by its m/z (4 decimals) and RT in minutes, like 100.0000@1.00; the programme proposes the level and you decide." })); return; }
      const upd = (/** @type {string} */ k, /** @type {any} */ patch) => void save({ ...rows, [k]: { ...rows[k], ...patch } });
      box.append(el("div", { class: "flx-tablewrap", tabindex: 0, role: "region", "aria-label": "Decisions" }, el("table", { class: "flx-table" }, el("caption", { text: `${keys.length} decisions` }),
        el("thead", {}, el("tr", {}, ...["Row (m/z@RT)", "Decided level", "Note", ""].map(h => el("th", { scope: "col", text: h })))),
        el("tbody", {}, ...keys.map(k => el("tr", {},
          el("td", {}, el("input", { type: "text", value: k, "aria-label": `Row of ${k}`, onchange: (/** @type {any} */ e) => { const to = e.target.value.trim(); if (!to || (to !== k && rows[to])) { draw(); return; } const { [k]: old, ...rest } = rows; void save({ ...rest, [to]: old }); } })),
          el("td", {}, el("select", { "aria-label": `Decided level of ${k}`, onchange: (/** @type {any} */ e) => upd(k, { l: e.target.value }) },
            ...[["", "No decision"], ...LEVELS.map(l => [l, "L" + l]), ["nottp", "Not a TP"]].map(([v, t]) => el("option", { value: v, text: t, selected: v === rows[k].l })))),
          el("td", {}, el("input", { type: "text", maxlength: String(NOTE_MAX), value: rows[k].n || "", "aria-label": `Note for ${k}`, onchange: (/** @type {any} */ e) => upd(k, { n: e.target.value }) })),
          el("td", {}, F.btn("x", `Remove the decision of ${k}`, () => { const { [k]: _gone, ...rest } = rows; void save(rest); }, "Remove"))))))));
    };
    draw();
    return box;
  };

  /** review/1 → what the rules allowed, one row per decision @param {any} j @returns {{head: string[], rows: string[][]}} */
  F.readReview = j => ({
    head: ["Row", "Proposed", "Decided", "Result", "Reason", "Note"],
    rows: (j.rows || []).map((/** @type {any} */ r) => [txt(r.key), txt(r.proposed) ? "L" + r.proposed : "", r.level === "nottp" ? "Not a TP" : r.level ? "L" + r.level : "", txt(r.status), txt(r.reason), txt(r.note)]),
  });

  F.registerView({ id: "review", title: "Decisions on the products", types: ["transform"], accepts: (/** @type {any} */ item) => /\.json$/i.test(item.label), open: async (/** @type {HTMLElement} */ host, /** @type {any} */ item) => {
    const { el } = F;
    let j = null;
    try { j = JSON.parse(await item.text()); } catch (_) { /* said below */ }
    if (!j || j.schema !== "review/1") { host.append(el("p", { class: "fl-note", role: "status", text: `${item.label} is not a list of decisions (review/1).` })); return; }
    const s = j.summary || {};
    host.append(el("p", { class: "fl-note", role: "status", text: `${s.decided || 0} decided, ${s.not_tp || 0} not a TP, ${s.refused || 0} refused by the rules, ${s.unmatched || 0} with no row in the table.` }));
    if (j.rows && j.rows.length) F.drawTable(host, { caption: item.label, ...F.readReview(j), label: `Decisions of ${item.label}` });
  } });
})();
