// @ts-check
// Flow (administrator mode): the Import data card. Files and folders are added through the loaders of the Data tab (hidden inputs #pick and #pickdir), and the metadata of the files
// (role, time, series, replicate) are generated from the files open in Data, uploaded from a CSV/TSV table or edited by hand in the table of flow-samples.js. Used by flow-inspect.js.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  /** the columns a table may carry, by the names a lab sheet is likely to use */
  const COLS = /** @type {Record<string, RegExp>} */ ({
    file: /^(file|filename|file name|name|sample name|sample)$/i, tipo: /^(role|type|tipo|kind)$/i, tempo_min: /^(time|time \(min\)|time_min|minutes|min|tempo|t)$/i,
    serie: /^(series|serie|group|condition)$/i, replica: /^(replicate|replica|rep)$/i,
  });

  /** the cells of a delimited text: tab, semicolon or comma (the one most frequent in the first line), quotes with doubled quotes inside @param {string} text @returns {string[][]} */
  const cells = text => {
    const body = text.replace(/^﻿/, ""), first = body.split(/\r?\n/, 1)[0] || "";
    const sep = [["\t", 0], [";", 0], [",", 0]].map(([c]) => /** @type {[string, number]} */ ([c, first.split(/** @type {string} */ (c)).length])).sort((a, b) => b[1] - a[1])[0][0];
    /** @type {string[][]} */ const out = []; let row = /** @type {string[]} */ ([]), cur = "", quoted = false;
    for (let i = 0; i < body.length; i++) {
      const c = body[i];
      if (quoted) { if (c === '"') { if (body[i + 1] === '"') { cur += '"'; i++; } else quoted = false; } else cur += c; }
      else if (c === '"') quoted = true;
      else if (c === sep) { row.push(cur); cur = ""; }
      else if (c === "\n" || c === "\r") { if (c === "\r" && body[i + 1] === "\n") i++; row.push(cur); out.push(row); row = []; cur = ""; }
      else cur += c;
    }
    if (cur || row.length) { row.push(cur); out.push(row); }
    return out.filter(r => r.some(x => x.trim()));
  };

  /** a metadata table → one patch per file: `file`, and the role, minutes, series and replicate it gives; `unknown` lists the roles that are not in `kinds`, `missing` the columns it lacks @param {string} text @param {string[]} kinds
   * @returns {{rows: Record<string, any>[], unknown: string[], error: string}} */
  F.parseMetadata = (text, kinds) => {
    const t = cells(text || ""), head = (t[0] || []).map(h => h.trim()), at = /** @type {Record<string, number>} */ ({});
    for (const [k, re] of Object.entries(COLS)) at[k] = head.findIndex(h => re.test(h));
    if (at.file < 0) return { rows: [], unknown: [], error: "The table needs a column named File (or Name)." };
    const unknown = /** @type {string[]} */ ([]), rows = t.slice(1).map(r => {
      const o = /** @type {Record<string, any>} */ ({ file: (r[at.file] || "").trim() });
      if (at.tipo >= 0 && (r[at.tipo] || "").trim()) { const k = r[at.tipo].trim().toLowerCase(); if (kinds.includes(k)) o.tipo = k; else if (!unknown.includes(k)) unknown.push(k); }
      if (at.tempo_min >= 0 && (r[at.tempo_min] || "").trim() !== "" && Number.isFinite(Number(r[at.tempo_min]))) o.tempo_min = Number(r[at.tempo_min]);
      if (at.serie >= 0) o.serie = (r[at.serie] || "").trim();
      if (at.replica >= 0 && Number.isInteger(Number(r[at.replica])) && Number(r[at.replica]) > 0) o.replica = Number(r[at.replica]);
      return o;
    }).filter(o => o.file);
    return { rows, unknown, error: rows.length ? "" : "The table has no rows." };
  };

  /** the metadata of the files open in Data with the patches of a table laid over them; files the table names but Data does not have are added from the table alone
   * @param {any} base the current document @param {Record<string, any>[]} patches @returns {any} */
  F.applyMetadata = (base, patches) => {
    const rows = /** @type {any[]} */ ([...(base.campioni || [])]);
    patches.forEach(p => {
      const at = rows.findIndex(r => r.file === p.file), { file, ...rest } = p;
      if (at >= 0) rows[at] = { ...rows[at], ...rest, ...("tempo_min" in rest ? { tempo: rest.tempo_min, tempo_confermato: true } : {}) };
      else rows.push({ ...F.sampleBlank(rows.length), file, ...rest, ...("tempo_min" in rest ? { tempo: rest.tempo_min, tempo_confermato: true } : {}) });
    });
    return { ...base, campioni: rows };
  };

  /** the Import card's own block: the files in Data, the buttons that add files and folders, and the metadata with its table @param {any} S */
  F.importBox = S => {
    const { el } = F, box = el("div", { class: "flx-imp" }), say = el("p", { class: "fl-note", role: "status" }), table = el("div");
    const input = (S.plan.inputs || []).find((/** @type {any} */ i) => i.type === "samples") || (S.metaInput = S.metaInput || { name: "metadata", sha256: "", type: "samples" });
    const click = (/** @type {string} */ id) => { const n = document.getElementById(id); if (n) n.click(); else say.textContent = "The Data tab has no loader here."; };
    const drawTable = () => table.replaceChildren(F.samplesEditor ? F.samplesEditor(S, input) : "");
    const upload = el("input", { type: "file", accept: ".csv,.tsv,.txt,.xlsx,text/csv,text/tab-separated-values,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", hidden: true, "aria-label": "Metadata table (CSV, TSV or XLSX)", onchange: async (/** @type {any} */ e) => {
      const f = e.target.files && e.target.files[0]; e.target.value = ""; if (!f) return;
      let text = "";
      try { text = /\.xlsx$/i.test(f.name) ? await F.xlsxToText(await f.arrayBuffer()) : await f.text(); } catch (err) { say.textContent = `${f.name}: ${(/** @type {any} */ (err)).message || err}`; return; }
      const r = F.parseMetadata(text, G.CDOC ? G.CDOC.KINDS : []);
      if (r.error) { say.textContent = `${f.name}: ${r.error}`; return; }
      const cur = (S.samples && S.samples[input.name]) || F.samplesFromData() || { schema: "campioni/2", campioni: [] };
      await F.samplesSet(S, input, F.applyMetadata(cur, r.rows));
      const open = new Set(F.dataFiles()), absent = r.rows.filter((/** @type {any} */ x) => !open.has(x.file)).length;
      say.textContent = `${r.rows.length} rows read from ${f.name}.` + (absent ? ` ${absent} not in Data.` : "") + (r.unknown.length ? ` Unknown roles left unchanged: ${r.unknown.join(", ")}.` : "");
      drawTable();
    } });
    box.append(
      el("div", { class: "fl-meta" },
        F.btn("plus", "Add data files from the computer", () => click("pick"), "Add files"),
        F.btn("file", "Add a whole folder of data files", () => click("pickdir"), "Add a folder")),
      el("h4", { text: "Metadata" }),
      el("p", { class: "fl-note", text: "Generate them from the files in Data, upload a CSV, TSV or XLSX table (first sheet; columns File, Role, Time, Series, Replicate), then fix what is missing in the table." }),
      el("div", { class: "fl-meta" }, F.btn("import", "Upload a CSV, TSV or XLSX metadata table", () => upload.click(), "Upload table"), upload), say, table);
    drawTable();
    // the files are added by the Data loaders, which tell nobody: look for a change while the card is on the page
    let seen = F.dataFiles().join("\n");
    const timer = setInterval(() => {
      if (!box.isConnected) { clearInterval(timer); return; }
      const now = F.dataFiles().join("\n");
      if (now !== seen) { seen = now; clearInterval(timer); if (S.refresh) S.refresh(null); }
    }, 1000);
    return box;
  };
})();
