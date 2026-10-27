// @ts-check
// Flow (administrator mode): the table of the Raw files node, one row per file open in Data (type, MS levels, polarity, instrument, size) with Explore (the File and
// instrument, Scans and Precursor tree tabs of the Data bench, drawn by BANCO.fileView) and Export mzML. The values are the ones the engine reports for the file; nothing is worked out here.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  const TABS = [["fil", "File and instrument"], ["lst", "Scans"], ["alb", "Precursor tree"]];
  const CSS = `.fl-rf .flx-tablewrap{max-height:none}.fl-rf td .fl-b{min-height:28px;min-width:24px}.fl-rf td:last-child{display:flex;gap:4px;border:0;padding:0 0 0 4px}.fl-rf tr[aria-current=true] td{background:var(--soft)}
.fl-rf-ex{margin-top:8px}.fl-rf-tabs{display:flex;gap:4px;flex-wrap:wrap}.fl-rf-tabs button{min-height:28px;min-width:24px;padding:2px 10px}.fl-rf-tabs button[aria-selected=true]{background:var(--accent);color:var(--on-accent,#fff)}
.fl-rf-body{height:300px;overflow:auto;border:1px solid var(--line);border-radius:4px;margin-top:6px;padding:6px 8px;font-size:12px}@media (pointer:coarse){.fl-rf td .fl-b,.fl-rf-tabs button{min-height:40px;min-width:40px}}`;

  /** the name of the file an item of the Data session belongs to (a mixed file has one item per part) @param {any} f */
  const nameOf = f => String(f.file).split("#")[0];
  /** the display name of what the engine says a file is: infusion and SRM/MRM from its kind, DDA / DIA / MSn from its scan scheme, else LC-MS @param {any} f */
  const typeOf = f => f.kind === "infusion" ? "Infusion" : f.kind === "mrm" ? "SRM/MRM" : ["DDA", "DIA", "MSn"].find(a => (f.acq || []).includes(a)) || (f.kind === "ms2" ? "MS2" : "LC-MS");
  /** MS1–MS8 from the deepest level the engine counted @param {any[]} parts */
  const levels = parts => {
    const top = Math.max(0, ...parts.map(f => +f.max_level || 0)), from = parts.some(f => (f.acq || []).includes("MS1")) ? 1 : Math.min(2, top);
    return !top ? "" : top > from ? `MS${from}–MS${top}` : `MS${top}`;
  };
  /** @param {number} n bytes */
  const bytes = n => !n ? "" : n >= 1e9 ? `${(n / 1e9).toFixed(1)} GB` : n >= 1e6 ? `${(n / 1e6).toFixed(1)} MB` : `${Math.max(1, Math.round(n / 1e3))} kB`;
  /** @param {string} name */
  const sizeOf = name => {
    const st = typeof ST !== "undefined" && Array.isArray(ST.files) ? ST.files.find((/** @type {any} */ x) => x.name === name) : null, b = blobOf(name);
    return bytes(st && st.size || b && b.size || 0);
  };
  /** the mzML of a file, as the engine reads it in this page @param {string} name @returns {Blob | null} */
  const blobOf = name => { const r = /** @type {any} */ (window).MZLAB_RUST; return r && r.blobOf ? r.blobOf(name) : null; };
  const uniq = (/** @type {string[]} */ a) => [...new Set(a.filter(Boolean))];

  /** the files open in Data, a row each, in the order of their names @returns {{ name: string, k: number, parts: any[] }[]} */
  const rowsOf = () => {
    const by = new Map();
    for (const f of typeof E !== "undefined" && Array.isArray(E.files) ? E.files : []) {
      if (!f || f.gone || !f.file) continue;
      const n = nameOf(f);
      if (!by.has(n)) by.set(n, { name: n, k: f.k, parts: [] });
      by.get(n).parts.push(f);
    }
    return [...by.values()].sort((a, b) => a.name.localeCompare(b.name));
  };

  /** saves the mzML: the one made when a maker's file was read, else the file as it is; the name is <stem>.mzML @param {string} name @param {(text: string) => void} say */
  const exportMzml = async (name, say) => {
    const stem = name.replace(/\.[^./]+$/, "");
    if (typeof CONV !== "undefined" && CONV.has(name)) { await CONV.download(name); return; }
    const blob = blobOf(name);
    if (!blob) { say("No copy to export."); return; }
    saveBlob(blob, `${stem}.mzML`, 60000);
  };

  /** the table of the files with their buttons */
  F.rawFiles = () => {
    const { el } = F;
    if (!document.getElementById("fl-rf-css")) { const s = el("style", { id: "fl-rf-css", text: CSS }); document.head.append(s); }
    const rows = rowsOf(), box = el("div", { class: "fl-rf" }), say = el("span", { class: "fl-note", role: "status" }), ex = el("div", { class: "fl-rf-ex", hidden: true });
    if (!rows.length) return el("p", { class: "fl-note fl-data" }, el("span", { text: "No file loaded. " }), F.btn("import", "Go to the Data tab to open files", () => { const b = document.querySelector('#nav [data-v="data"]'); if (b) /** @type {HTMLElement} */ (b).click(); }, "Open Data"));
    let open = "";
    const tbody = el("tbody");
    const explore = (/** @type {any} */ r, /** @type {string} */ tab) => {
      open = r.name; ex.hidden = false; ex.replaceChildren();
      const has = TABS.filter(([id]) => id !== "alb" || r.parts.some((/** @type {any} */ f) => f.kind === "infusion"));
      const body = el("div", { class: "fl-rf-body", role: "tabpanel", tabindex: "0" }), tabs = el("div", { class: "fl-rf-tabs", role: "tablist", "aria-label": `Explore ${r.name}` });
      const pick = (/** @type {string} */ id) => {
        for (const b of tabs.children) b.setAttribute("aria-selected", String(b.dataset.tab === id));
        const bn = /** @type {any} */ (window).BANCO;
        if (bn && bn.fileView) bn.fileView(body, r.k, id); else body.textContent = "The Data tab is not ready.";
      };
      for (const [id, name] of has) tabs.append(el("button", { type: "button", role: "tab", class: "fl-b", "data-tab": id, "aria-selected": "false", onclick: () => pick(id) }, el("span", { text: name })));
      ex.append(tabs, body); pick(has.some(([id]) => id === tab) ? tab : "fil");
      for (const tr of tbody.children) tr.toggleAttribute("aria-current", tr.dataset.name === open);
    };
    for (const r of rows) {
      const tr = el("tr", { "data-name": r.name }, el("th", { scope: "row", text: r.name }), el("td", { text: uniq(r.parts.map(typeOf)).join(" + ") }), el("td", { text: levels(r.parts) }), el("td", { text: uniq(r.parts.map((/** @type {any} */ f) => f.polarity)).join(", ") }),
        el("td", { text: uniq(r.parts.map((/** @type {any} */ f) => f.instrument)).join(", ") }), el("td", { text: sizeOf(r.name) }),
        el("td", {}, F.btn("search", `Explore ${r.name}`, () => { if (open === r.name) { open = ""; ex.hidden = true; ex.replaceChildren(); tr.removeAttribute("aria-current"); } else explore(r, "fil"); }, "Explore"),
          F.btn("export", `Export ${r.name} as mzML`, () => { exportMzml(r.name, t => { say.textContent = t; }); }, "Export mzML")));
      tbody.append(tr);
    }
    const head = ["File", "Type", "MS levels", "Polarity", "Instrument", "Size"].map(h => el("th", { scope: "col", text: h })), act = el("th", { scope: "col" }, el("span", { class: "sr-only", text: "Actions" }));
    box.append(el("div", { class: "flx-tablewrap" }, el("table", { class: "flx-table" }, el("caption", { class: "sr-only", text: "Files in Data" }), el("thead", {}, el("tr", {}, ...head, act)), tbody)), say, ex);
    return box;
  };
})();
