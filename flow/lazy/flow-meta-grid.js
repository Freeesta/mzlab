// @ts-check
// Flow (administrator mode): the metadata grid over a campioni/3 document (virtual rows, ARIA grid). The cells, the checks (`valida`), the name patterns (`modello`), the CSV (`csv_*`) and the data read
// from the files (`dal_file`) are all the Rust of mzlab-core reached through CDOC.post("<op>"); this file only lays the cells out and moves the keyboard focus. Used by flow-samples.js (F.samplesEditor)
// and flow-import.js (F.metaImport). The document lives in S.samples[input name]; its SHA-256 is the canonical one (F.samplesSet). English only. Docs: docs/agenti/flow-grafica.md.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  const ROW = 28, VIEW = 360, OVER = 4, KINDS = ["sample", "t0", "blank", "dark", "infusion", "std", "calibration", "qc", "pool"];
  /** the columns of the document, then the read-only ones that come from the files; kind: text, num, int, cat, ro */
  const FIXED = [
    { id: "file", label: "File", w: 200, kind: "text" }, { id: "campione", label: "Sample in file", w: 96, kind: "text" }, { id: "campione_id", label: "Sample ID", w: 150, kind: "text" },
    { id: "tipo", label: "Role", w: 96, kind: "cat", opts: KINDS }, { id: "matrice", label: "Matrix", w: 72, kind: "text" }, { id: "serie", label: "Condition", w: 100, kind: "text" },
    { id: "gruppo", label: "Group", w: 110, kind: "text" }, { id: "tempo", label: "Time", w: 72, kind: "num" }, { id: "unita_tempo", label: "Time unit", w: 80, kind: "cat", opts: ["s", "min", "h", "d"] },
    { id: "replica", label: "Replicate", w: 84, kind: "int" }, { id: "lotto", label: "Batch", w: 72, kind: "text" }, { id: "concentrazione", label: "Conc.", w: 72, kind: "num" },
    { id: "unita", label: "Conc. unit", w: 80, kind: "text" }, { id: "note", label: "Note", w: 140, kind: "text" },
  ];
  const FROMFILE = [
    { id: "polarita", label: "Polarity", w: 76, kind: "ro" }, { id: "reiniezione", label: "Re-injection", w: 96, kind: "ro" }, { id: "acquisito", label: "Acquired", w: 150, kind: "ro" },
    { id: "modello", label: "Instrument", w: 130, kind: "ro" }, { id: "mz", label: "m/z range", w: 110, kind: "ro" }, { id: "rt", label: "RT range (min)", w: 110, kind: "ro" }, { id: "scansioni", label: "Scans", w: 72, kind: "ro" },
  ];
  /** what a finding of `valida` says; the code is the stable part */
  const MSG = /** @type {Record<string, string>} */ ({
    timeMissing: "Time missing", timeUnconfirmed: "Time not confirmed", t0WithTime: "A t0 with a time other than 0", missingValue: "Missing value in a required column", duplicateKey: "Same file and sample as an earlier row",
    pairMissing: "This sample ID has only one polarity", pairMismatch: "Positive and negative of this sample differ in time, role or condition", typeMismatch: "The value does not fit the column type",
    unknownCondition: "Unknown condition in the sample ID", noControls: "No t0, blank or dark control", noT0: "No t0", noBlank: "No blank", noDark: "No dark control", onePoint: "One time point only",
    replicates: "Replicates found", noTreated: "No treated sample", noInfusion: "No infusion file",
  });
  const CELL_OF = /** @type {Record<string, string>} */ ({ timeMissing: "tempo", timeUnconfirmed: "tempo", t0WithTime: "tempo", duplicateKey: "file", pairMissing: "campione_id", pairMismatch: "campione_id" });
  const CSS = `.mg{display:flex;flex-direction:column;gap:6px;min-width:0}.mg-bar{display:flex;flex-wrap:wrap;gap:6px;align-items:center}.mg-bar select,.mg-bar input{min-height:28px;font:inherit;max-width:100%}
.mg-sc{position:relative;overflow:auto;height:${VIEW}px;border:1px solid var(--line);border-radius:6px;background:var(--panel);color:var(--ink);font-size:12px}.mg-hd{position:sticky;top:0;z-index:2;background:var(--panel)}
.mg-r{display:flex;height:${ROW}px;width:max-content;min-width:100%;box-sizing:border-box}.mg-bd{position:relative}.mg-bd .mg-r{position:absolute;left:0}.mg-c{flex:0 0 auto;box-sizing:border-box;height:${ROW}px;padding:0 6px;display:flex;align-items:center;gap:3px;overflow:hidden;white-space:nowrap;text-overflow:ellipsis;border-right:1px solid var(--line);border-bottom:1px solid var(--line)}
.mg-h{font-weight:600;padding:0}.mg-h button{all:unset;box-sizing:border-box;display:flex;align-items:center;gap:4px;width:100%;height:100%;padding:0 6px;cursor:pointer}.mg-h button:focus-visible,.mg-c:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}
.mg-ro{color:var(--muted)}.mg-sel{background:color-mix(in srgb,var(--accent) 16%,transparent)}.mg-act{outline:2px solid var(--accent);outline-offset:-2px}.mg-g1 .mg-c{background:color-mix(in srgb,var(--accent) 6%,transparent)}.mg-gs .mg-c{border-top:2px solid var(--ink)}
.mg-c.mg-err{box-shadow:inset 0 -3px 0 var(--bad)}.mg-c.mg-wrn{box-shadow:inset 0 -3px 0 var(--warn)}.mg-pair .mg-c:first-child{border-left:4px dashed var(--warn)}.mg-c input,.mg-c select{width:100%;height:100%;box-sizing:border-box;font:inherit;border:0;background:var(--panel);color:var(--ink);padding:0 2px}
.mg-chips{display:flex;flex-wrap:wrap;gap:4px}.mg-chip{display:inline-flex;align-items:center;gap:2px;border:1px solid var(--line);border-radius:12px;padding:0 4px 0 8px;min-height:24px}.mg-panel{border:1px solid var(--line);border-radius:6px;padding:8px;display:flex;flex-direction:column;gap:6px}
.mg-panel table{border-collapse:collapse}.mg-panel td,.mg-panel th{padding:2px 6px;text-align:left}.mg-panel mark{background:color-mix(in srgb,var(--accent) 28%,transparent);color:var(--ink)}.mg-panel sub{color:var(--muted)}`;

  const blankRow = (/** @type {number} */ n) => ({ acquisito: null, campione: "", campione_id: "", composto: "", concentrazione: null, extra: {}, file: "", gruppo: "", istd: false, lotto: "", matrice: "", note: "", ordine_iniezione: n, polarita: null,
    reiniezione: false, replica: 1, serie: "", sha256: "", tempo: null, tempo_confermato: false, tempo_min: null, tipo: "sample", tipo_bianco: null, unita: null, unita_tempo: "min" });
  const emptyDoc = () => ({ schema: "campioni/3", colonne: [], campioni: [] });
  const call = async (/** @type {string} */ op, /** @type {any} */ body) => {
    const r = await G.CDOC.post(op, body);
    if (r && r.error) throw new Error(String(r.error_key || r.error));
    return r;
  };
  /** any campioni document → campioni/3 (the Rust migrates) @param {any} d */
  const toV3 = async d => (d && d.schema === "campioni/3" ? d : d ? call("migra", d) : emptyDoc());
  /** the sample rows of a document and their number (kept out of template strings: the English-only guard reads those) */
  const rowsOf = (/** @type {any} */ d) => d.campioni;
  const keyOf = (/** @type {any} */ r) => (r.sha256 || r.file) + "\0" + (r.campione || "");
  const base = (/** @type {string} */ p) => p.replace(/^.*[\\/]/, "").replace(/\.[^.]*$/, "");
  const cut = (/** @type {number} */ x) => (Number.isInteger(x) ? String(x) : String(+x.toFixed(4)));

  /** the files open in the Data tab as the entries `dal_file` reads (polarity, ranges, scans, instrument) */
  const fileInfo = () => (typeof E !== "undefined" && Array.isArray(E.files) ? E.files : []).filter((/** @type {any} */ f) => f && !f.gone && f.file).map((/** @type {any} */ f) => ({
    nome: String(f.file), sha256: f.sha256 || "", polarita: f.polarity === "positive" ? [1] : f.polarity === "negative" ? [-1] : f.polarity === "mixed" ? [1, -1] : [], rt_min: f.rt_min ?? null, rt_max: f.rt_max ?? null,
    mz_min: f.mz_min ?? null, mz_max: f.mz_max ?? null, scansioni: (f.ms1 || 0) + (f.ms2 || 0) || null, modello: f.instrument || "" }));

  /** the document of the files open in Data, completed from the files; null when none is open */
  const fromData = async () => {
    const d = F.samplesFromData(); if (!d) return null;
    const v3 = await toV3(d), info = fileInfo();
    if (!info.length) return v3;
    try { return (await call("dal_file", { campioni: v3, file: info })).campioni; } catch (_) { return v3; }
  };

  /** the rows of `add` laid over `cur` (same file and sample, or same SHA-256), the others appended; the file data of the current rows are kept @param {any} cur @param {any} add */
  const merge = (cur, add) => {
    const rows = /** @type {any[]} */ ([...cur.campioni]), cols = [...cur.colonne];
    for (const c of add.colonne || []) if (!cols.some(x => x.nome === c.nome)) cols.push(c);
    for (const r of add.campioni) {
      const at = rows.findIndex(x => keyOf(x) === keyOf(r) || (r.sha256 && x.sha256 === r.sha256 && x.campione === r.campione));
      if (at < 0) { rows.push({ ...r, ordine_iniezione: rows.length + 1 }); continue; }
      const o = rows[at];
      rows[at] = { ...o, ...r, extra: { ...o.extra, ...r.extra }, sha256: r.sha256 || o.sha256, acquisito: r.acquisito ?? o.acquisito, polarita: r.polarita ?? o.polarita, dati_file: o.dati_file ?? r.dati_file, ordine_iniezione: o.ordine_iniezione };
    }
    return { ...cur, colonne: cols, campioni: rows };
  };

  /** reads a CSV/TSV text into the sample list of `input` (laid over what is there, or over the files open in Data); the old header names (Time (min), Series…) are renamed first
   * @param {any} S @param {any} input @param {string} text @returns {Promise<{count: number, absent: number, doc: any}>} */
  F.metaImport = async (S, input, text) => {
    const lines = text.replace(/^﻿/, "").split(/\r?\n/), sep = ["\t", ";", ","].map(c => [c, lines[0].split(c).length]).sort((a, b) => /** @type {number} */ (b[1]) - /** @type {number} */ (a[1]))[0][0];
    lines[0] = lines[0].split(/** @type {string} */ (sep)).map(h => { const t = h.trim().replace(/^"|"$/g, ""); const k = Object.entries(F.META_HEADERS || {}).find(([, re]) => /** @type {RegExp} */ (re).test(t)); return k ? k[0] : h; }).join(/** @type {string} */ (sep));
    const add = await call("csv_leggi", { testo: lines.join("\n"), separatore: sep });
    const cur = (S.samples && S.samples[input.name] && (await toV3(S.samples[input.name]))) || (await fromData()) || emptyDoc();
    const open = new Set(F.dataFiles()), doc = merge(cur, add);
    await F.samplesSet(S, input, doc);
    return { count: add.campioni.length, absent: add.campioni.filter((/** @type {any} */ r) => !open.has(r.file)).length, doc };
  };

  /** the cells of a pasted text (tab separated, quotes as Excel writes them) @param {string} text @returns {string[][]} */
  const tsv = text => {
    /** @type {string[][]} */ const out = []; let row = /** @type {string[]} */ ([]), cur = "", q = false;
    for (let i = 0; i < text.length; i++) {
      const c = text[i];
      if (q) { if (c === '"') { if (text[i + 1] === '"') { cur += '"'; i++; } else q = false; } else cur += c; }
      else if (c === '"' && !cur) q = true;
      else if (c === "\t") { row.push(cur); cur = ""; }
      else if (c === "\n" || c === "\r") { if (c === "\r" && text[i + 1] === "\n") i++; row.push(cur); out.push(row); row = []; cur = ""; }
      else cur += c;
    }
    if (cur || row.length) { row.push(cur); out.push(row); }
    return out;
  };

  /** the state of the grid, the columns, the text of a cell and the findings of `valida` @param {any} mg */
  const installState = mg => {
    const { el } = F;
    Object.assign(mg, { doc: emptyDoc(), ver: 0, timer: 0, warns: new Map(), pairRows: new Set(), view: [], gidx: [], sort: null, group: "", filters: [], act: { r: 0, c: 0 }, anc: { r: 0, c: 0 }, editing: null, hasFocus: false, summary: "" });
    mg.say = el("p", { class: "fl-note", role: "status" });
    mg.note = el("p", { class: "fl-note", role: "status", "aria-live": "polite" });
    mg.cols = () => [...FIXED, ...mg.doc.colonne.map((/** @type {any} */ c) => ({ id: "x:" + c.nome, label: c.nome, w: 110, kind: c.tipo === "categoria" ? "xcat" : c.tipo === "numero" ? "xnum" : "text", opts: c.valori }))].concat(FROMFILE);

    /** the text a cell shows @param {any} r @param {any} c */
    mg.show = (r, c) => {
      const id = c.id, f = r.dati_file || {};
      if (id.startsWith("x:")) { const v = r.extra[id.slice(2)]; return v == null ? "" : String(v); }
      if (id === "polarita") return r.polarita === 1 ? "positive" : r.polarita === -1 ? "negative" : "";
      if (id === "reiniezione") return r.reiniezione ? "yes" : "";
      if (id === "modello") return f.modello || "";
      if (id === "mz") return f.mz_min != null ? `${cut(f.mz_min)}–${cut(f.mz_max)}` : "";
      if (id === "rt") return f.rt_min != null ? `${cut(f.rt_min)}–${cut(f.rt_max)}` : "";
      if (id === "scansioni") return f.scansioni != null ? String(f.scansioni) : "";
      const v = r[id]; return v == null ? "" : String(v);
    };
    /** the row with `text` typed into column `c`, or null when the cell takes no such value @param {any} r @param {any} c @param {string} text */
    mg.put = (r, c, text) => {
      const t = text.trim();
      if (c.kind === "ro") return null;
      if (c.id.startsWith("x:")) {
        const k = c.id.slice(2), extra = { ...r.extra };
        if (t === "") delete extra[k]; else if (c.kind === "xnum") { const n = Number(t.replace(",", ".")); extra[k] = Number.isFinite(n) ? n : t; } else extra[k] = t;
        return { ...r, extra };
      }
      if (c.kind === "num" || c.kind === "int") {
        const n = t === "" ? null : Number(t.replace(",", "."));
        if (n != null && !Number.isFinite(n)) return null;
        if (c.id === "tempo") return { ...r, tempo: n, tempo_min: n, tempo_confermato: n != null };
        return { ...r, [c.id]: n == null ? (c.kind === "int" ? 1 : null) : c.kind === "int" ? Math.max(1, Math.round(n)) : n };
      }
      if (c.kind === "cat" && c.opts && !c.opts.includes(t)) return null;
      if (c.id === "unita" || c.id === "concentrazione") return { ...r, [c.id]: t === "" ? null : t };
      return { ...r, [c.id]: t };
    };

    mg.recompute = () => {
      const cs = mg.cols(), idx = mg.doc.campioni.map((/** @type {any} */ _, /** @type {number} */ i) => i).filter((/** @type {number} */ i) => mg.filters.every(f => {
        const c = cs.find(x => x.id === f.id); return !c || mg.show(mg.doc.campioni[i], c).toLowerCase().includes(f.text.toLowerCase());
      }));
      const by = (/** @type {string} */ id, /** @type {number} */ dir) => { const c = cs.find(x => x.id === id); if (!c) return; const num = c.kind === "num" || c.kind === "int" || c.kind === "xnum";
        idx.sort((/** @type {number} */ a, /** @type {number} */ b) => { const x = mg.show(mg.doc.campioni[a], c), y = mg.show(mg.doc.campioni[b], c); return dir * (num && x !== "" && y !== "" ? Number(x) - Number(y) : x.localeCompare(y, undefined, { numeric: true })); }); };
      if (mg.sort) by(mg.sort.id, mg.sort.dir);
      if (mg.group) by(mg.group, 1);
      mg.view = idx; mg.gidx = []; let g = 0;
      const gc = cs.find(x => x.id === mg.group);
      mg.view.forEach((i, k) => { if (gc && k && mg.show(mg.doc.campioni[i], gc) !== mg.show(mg.doc.campioni[mg.view[k - 1]], gc)) g++; mg.gidx.push(g); });
      mg.act.r = Math.min(mg.act.r, Math.max(0, mg.view.length - 1)); mg.anc.r = Math.min(mg.anc.r, Math.max(0, mg.view.length - 1));
    };

    /** the findings of `valida` by cell: row|column id @param {any} avvisi */
    mg.indexWarns = avvisi => {
      mg.warns = new Map(); mg.pairRows = new Set(); let e = 0, w = 0, n = 0; const series = /** @type {string[]} */ ([]);
      for (const a of avvisi) {
        if (a.gravita === "error") e++; else if (a.gravita === "warning") w++; else n++;
        if (a.file == null) { if (a.gravita !== "info") series.push(`${MSG[a.codice] || a.codice}${a.serie ? " in " + a.serie : ""}`); continue; }
        const col = a.colonna ? (FIXED.some(f => f.id === a.colonna) ? a.colonna : "x:" + a.colonna) : CELL_OF[a.codice] || "file", k = a.file + "|" + col;
        mg.warns.set(k, [...(mg.warns.get(k) || []), { code: a.codice, sev: a.gravita }]);
        if (a.codice === "pairMissing") mg.pairRows.add(a.file);
      }
      mg.summary = `${e} errors, ${w} warnings, ${n} notes.` + (series.length ? " " + [...new Set(series)].join("; ") + "." : "");
    };
    mg.warnsOf = (/** @type {number} */ row, /** @type {string} */ col) => mg.warns.get(row + "|" + col) || [];
  };

  /** the elements of the grid and the drawing of the header and of the rows @param {any} mg */
  const installCells = mg => {
    const { el } = F;
    mg.sc = el("div", { class: "mg-sc", role: "grid", "aria-label": "Sample metadata", "aria-multiselectable": "true" });
    mg.head = el("div", { class: "mg-hd", role: "rowgroup" });
    mg.bd = el("div", { class: "mg-bd", role: "rowgroup" });
    mg.sc.append(mg.head, mg.bd);
    mg.cellEl = (/** @type {number} */ r, /** @type {number} */ c) => /** @type {HTMLElement | null} */ (mg.bd.querySelector(`[data-r="${r}"][data-c="${c}"]`));
    mg.inSel = (/** @type {number} */ r, /** @type {number} */ c) => r >= Math.min(mg.act.r, mg.anc.r) && r <= Math.max(mg.act.r, mg.anc.r) && c >= Math.min(mg.act.c, mg.anc.c) && c <= Math.max(mg.act.c, mg.anc.c);

    mg.drawHead = () => {
      const cs = mg.cols(), tr = el("div", { class: "mg-r", role: "row", "aria-rowindex": 1 });
      cs.forEach((c, i) => {
        const on = mg.sort && mg.sort.id === c.id, b = el("button", { type: "button", "aria-label": `Sort by ${c.label}`, onclick: () => { mg.sort = on && mg.sort && mg.sort.dir === -1 ? null : { id: c.id, dir: on ? -1 : 1 }; mg.refresh(); } },
          el("span", { text: c.label }), on ? el("span", { "aria-hidden": "true", text: mg.sort && mg.sort.dir === 1 ? "▲" : "▼" }) : null);
        tr.append(el("div", { class: "mg-c mg-h", role: "columnheader", "aria-colindex": i + 1, "aria-sort": on && mg.sort ? (mg.sort.dir === 1 ? "ascending" : "descending") : "none", style: `width:${c.w}px` }, b));
      });
      mg.head.replaceChildren(tr); mg.head.style.width = cs.reduce((s, c) => s + c.w, 0) + "px";
    };

    mg.drawRows = () => {
      const keep = mg.hasFocus, cs = mg.cols(), first = Math.max(0, Math.floor(mg.sc.scrollTop / ROW) - OVER), last = Math.min(mg.view.length - 1, first + Math.ceil(VIEW / ROW) + 2 * OVER), frag = document.createDocumentFragment();
      const rows = []; for (let k = first; k <= last; k++) rows.push(k);
      if (mg.act.r < first || mg.act.r > last) rows.push(mg.act.r);                     // the active row always exists: it holds the tab stop
      for (const k of mg.view.length ? rows : []) {
        const i = mg.view[k], r = mg.doc.campioni[i], gs = mg.group && k && mg.gidx[k] !== mg.gidx[k - 1];
        const tr = el("div", { class: `mg-r${mg.gidx[k] % 2 ? " mg-g1" : ""}${gs ? " mg-gs" : ""}${mg.pairRows.has(i) ? " mg-pair" : ""}`, role: "row", "aria-rowindex": k + 2, style: `top:${k * ROW}px` });
        cs.forEach((c, j) => {
          const w = mg.warnsOf(i, c.id), bad = w.find(x => x.sev === "error"), txt = mg.show(r, c), isAct = mg.act.r === k && mg.act.c === j;
          const cell = el("div", { class: `mg-c${c.kind === "ro" ? " mg-ro" : ""}${mg.inSel(k, j) ? " mg-sel" : ""}${isAct ? " mg-act" : ""}${w.length ? (bad ? " mg-err" : " mg-wrn") : ""}`, role: "gridcell", "aria-colindex": j + 1, "aria-selected": mg.inSel(k, j) ? "true" : "false",
            "aria-invalid": bad ? "true" : null, "aria-readonly": c.kind === "ro" ? "true" : null, tabindex: isAct ? 0 : -1, "data-r": k, "data-c": j, style: `width:${c.w}px`, title: w.map(x => MSG[x.code] || x.code).join("; ") || txt });
          if (mg.editing && mg.editing.r === k && mg.editing.c === j) cell.append(mg.editor(r, c)); else { if (w.length) cell.append(el("span", { "aria-hidden": "true", text: bad ? "✖" : "⚠" })); cell.append(document.createTextNode(txt)); }
          tr.append(cell);
        });
        frag.append(tr);
      }
      mg.bd.replaceChildren(frag); mg.bd.style.height = mg.view.length * ROW + "px"; mg.bd.style.width = cs.reduce((s, c) => s + c.w, 0) + "px";
      mg.sc.setAttribute("aria-rowcount", String(mg.view.length + 1)); mg.sc.setAttribute("aria-colcount", String(cs.length));
      mg.sc.tabIndex = mg.view.length ? -1 : 0;
      if (keep) { const n = mg.editing ? mg.bd.querySelector("input,select") : mg.cellEl(mg.act.r, mg.act.c); if (n) { /** @type {HTMLElement} */ (n).focus({ preventScroll: true }); mg.hasFocus = true; } }
    };
  };

  /** the in-cell editor and the commit of a document @param {any} mg */
  const installEdit = mg => {
    const { el } = F;
    /** the in-cell editor: text, number or a list @param {any} r @param {any} c */
    mg.editor = (r, c) => {
      const label = `${c.label} of ${r.file || "row " + (mg.act.r + 1)}`, v = mg.editing && /** @type {any} */ (mg.editing).seed != null ? /** @type {any} */ (mg.editing).seed : mg.show(r, c);
      const opts = c.kind === "cat" ? c.opts : c.kind === "xcat" ? c.opts : null;
      const hit = mg.editing && /** @type {any} */ (mg.editing).seed && opts ? opts.find((/** @type {string} */ o) => o.toLowerCase().startsWith(String(/** @type {any} */ (mg.editing).seed).toLowerCase())) : null;      // a typed letter picks the first value that starts with it
      if (opts) return el("select", { "aria-label": label, onchange: (/** @type {any} */ e) => mg.finish(e.target.value, 1, 0), onkeydown: mg.editKeys }, ...[...(c.kind === "xcat" ? [""] : []), ...opts].map((/** @type {string} */ o) => el("option", { value: o, text: o, selected: hit ? o === hit : o === mg.show(r, c) })));
      return el("input", { type: "text", value: v, inputMode: c.kind === "num" || c.kind === "int" || c.kind === "xnum" ? "decimal" : "text", "aria-label": label, onkeydown: mg.editKeys, onblur: (/** @type {any} */ e) => { if (mg.editing) mg.finish(e.target.value, 0, 0); } });
    };
    /** @param {KeyboardEvent} e */
    mg.editKeys = e => {
      const t = /** @type {HTMLInputElement} */ (e.target);
      if (e.key === "Enter") { e.preventDefault(); mg.finish(t.value, e.shiftKey ? -1 : 1, 0); }
      else if (e.key === "Tab") { e.preventDefault(); mg.finish(t.value, 0, e.shiftKey ? -1 : 1); }
      else if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); mg.editing = null; mg.drawRows(); }
    };
    /** commits the editor and moves the active cell by (dr, dc) @param {string} text @param {number} dr @param {number} dc */
    mg.finish = (text, dr, dc) => {
      if (!mg.editing) return;
      const { r, c } = mg.editing, col = mg.cols()[c], i = mg.view[r]; mg.editing = null;
      const next = mg.put(mg.doc.campioni[i], col, text);
      if (next === null) mg.say.textContent = `“${text}” does not fit ${col.label}.`; else if (JSON.stringify(next) !== JSON.stringify(mg.doc.campioni[i])) mg.commit(mg.patch(mg.doc, [[i, next]]));
      mg.move(dr, dc, false);
    };
    mg.patch = (/** @type {any} */ d, /** @type {[number, any][]} */ rows) => { const cs = [...d.campioni]; for (const [i, r] of rows) cs[i] = r; return { ...d, campioni: cs }; };

    /** keeps `d` as the document: shown at once, hashed and checked a moment later @param {any} d */
    mg.commit = d => {
      mg.doc = d; mg.S.samples = mg.S.samples || {}; mg.S.samples[mg.input.name] = mg.doc; const mine = ++mg.ver;
      mg.recompute(); mg.drawHead(); mg.drawRows(); mg.noteOfActive();
      clearTimeout(mg.timer);
      mg.timer = window.setTimeout(async () => {
        try {
          await F.samplesSet(mg.S, mg.input, mg.doc); mg.sha.textContent = mg.input.sha256 ? mg.input.sha256.slice(0, 12) + "…" : "not saved yet"; mg.sha.title = mg.input.sha256 || "";
          const w = await call("valida", mg.doc);
          if (mine === mg.ver) { mg.indexWarns(w.avvisi); mg.say.textContent = mg.summary; mg.drawRows(); mg.noteOfActive(); }
        } catch (e) { mg.say.textContent = "The list could not be checked: " + ((/** @type {any} */ (e)).message || e); }
      }, 120);
    };
    mg.refresh = () => { mg.recompute(); mg.drawHead(); mg.drawRows(); mg.noteOfActive(); };
  };

  /** moving the active cell, editing and the block writes @param {any} mg */
  const installMove = mg => {
    mg.noteOfActive = () => {
      const i = mg.view[mg.act.r], c = mg.cols()[mg.act.c]; if (i == null || !c) { mg.note.textContent = ""; return; }
      const w = mg.warnsOf(i, c.id); mg.note.textContent = w.length ? `${c.label}, ${rowsOf(mg.doc)[i].file || "row " + (mg.act.r + 1)}: ${w.map(x => `${x.sev === "error" ? "error" : "warning"}, ${MSG[x.code] || x.code}`).join("; ")}` : "";
    };
    /** moves the active cell; `keep` leaves the anchor (a range) @param {number} dr @param {number} dc @param {boolean} keep */
    mg.move = (dr, dc, keep) => {
      mg.act = { r: Math.max(0, Math.min(mg.view.length - 1, mg.act.r + dr)), c: Math.max(0, Math.min(mg.cols().length - 1, mg.act.c + dc)) };
      if (!keep) mg.anc = { ...mg.act };
      const top = mg.act.r * ROW, h = VIEW - ROW * 2;
      if (top < mg.sc.scrollTop) mg.sc.scrollTop = top; else if (top + ROW > mg.sc.scrollTop + h) mg.sc.scrollTop = top + ROW - h;
      const w = mg.cols()[mg.act.c].w; let left = 0; mg.cols().slice(0, mg.act.c).forEach(c => { left += c.w; });
      if (left < mg.sc.scrollLeft) mg.sc.scrollLeft = left; else if (left + w > mg.sc.scrollLeft + mg.sc.clientWidth) mg.sc.scrollLeft = left + w - mg.sc.clientWidth;
      mg.drawRows(); mg.noteOfActive();
    };
    mg.startEdit = (/** @type {string | null} */ seed) => { const c = mg.cols()[mg.act.c]; if (!mg.view.length || c.kind === "ro") return; mg.editing = /** @type {any} */ ({ ...mg.act, seed }); mg.drawRows(); const n = /** @type {HTMLInputElement | null} */ (mg.bd.querySelector("input")); if (n && seed != null) n.setSelectionRange(seed.length, seed.length); };

    /** writes `grid[k][j]` into the cells from the active one: the rows seen, the columns after it; read-only cells are skipped @param {string[][]} grid */
    mg.writeBlock = grid => {
      const cs = mg.cols(), rows = new Map(); let n = 0, cut = 0;
      grid.forEach((line, k) => line.forEach((t, j) => {
        const r = mg.act.r + k, c = mg.act.c + j; if (r >= mg.view.length || c >= cs.length) { cut++; return; }
        const i = mg.view[r], cur = rows.get(i) || mg.doc.campioni[i], next = mg.put(cur, cs[c], t);
        if (next) { rows.set(i, next); n++; } else if (cs[c].kind !== "ro") cut++;
      }));
      if (rows.size) mg.commit(mg.patch(mg.doc, [...rows.entries()]));
      mg.say.textContent = `${n} cells written` + (cut ? `, ${cut} left out (read-only, past the end or not fitting the column).` : ".");
    };
    /** Ctrl+D: the first row of the selection (or the row above) copied down the selected columns */
    mg.fillDown = () => {
      const r0 = Math.min(mg.act.r, mg.anc.r), r1 = Math.max(mg.act.r, mg.anc.r), c0 = Math.min(mg.act.c, mg.anc.c), c1 = Math.max(mg.act.c, mg.anc.c), cs = mg.cols(), from = r1 > r0 ? r0 : r0 - 1, to = r1 > r0 ? r1 : r0;
      if (from < 0) return;
      /** @type {Map<number, any>} */ const rows = new Map();
      for (let r = from + 1; r <= to; r++) for (let c = c0; c <= c1; c++) { const i = mg.view[r], cur = rows.get(i) || mg.doc.campioni[i], next = mg.put(cur, cs[c], mg.show(mg.doc.campioni[mg.view[from]], cs[c])); if (next) rows.set(i, next); }
      if (rows.size) mg.commit(mg.patch(mg.doc, [...rows.entries()])); mg.say.textContent = `Filled ${to - from} rows down.`;
    };
    mg.clearSel = () => {
      const r0 = Math.min(mg.act.r, mg.anc.r), r1 = Math.max(mg.act.r, mg.anc.r), c0 = Math.min(mg.act.c, mg.anc.c), c1 = Math.max(mg.act.c, mg.anc.c), cs = mg.cols(); /** @type {Map<number, any>} */ const rows = new Map();
      for (let r = r0; r <= r1; r++) for (let c = c0; c <= c1; c++) { const i = mg.view[r], next = mg.put(rows.get(i) || mg.doc.campioni[i], cs[c], cs[c].kind === "cat" ? mg.doc.campioni[i][cs[c].id] : ""); if (next) rows.set(i, next); }
      if (rows.size) mg.commit(mg.patch(mg.doc, [...rows.entries()]));
    };
  };

  /** scroll, focus, mouse, clipboard and keyboard of the grid @param {any} mg */
  const installEvents = mg => {
    const { el } = F;
    mg.sc.addEventListener("scroll", () => { if (!mg.editing) requestAnimationFrame(mg.drawRows); });
    mg.sc.addEventListener("focusin", e => {
      mg.hasFocus = true; const n = /** @type {HTMLElement | null} */ (mg.editing ? null : (/** @type {HTMLElement} */ (e.target)).closest("[data-r]"));       // Tab or focus() into a cell makes it the active one
      if (n && (+(n.dataset.r || 0) !== mg.act.r || +(n.dataset.c || 0) !== mg.act.c)) { mg.act = { r: +(n.dataset.r || 0), c: +(n.dataset.c || 0) }; mg.anc = { ...mg.act }; mg.drawRows(); mg.noteOfActive(); }
    });
    mg.sc.addEventListener("focusout", e => { if (!mg.sc.contains(/** @type {Node | null} */ (e.relatedTarget))) mg.hasFocus = false; });
    mg.sc.addEventListener("mousedown", e => {
      const n = /** @type {HTMLElement | null} */ ((/** @type {HTMLElement} */ (e.target)).closest("[data-r]")); if (!n || mg.editing) return;
      mg.act = { r: +(n.dataset.r || 0), c: +(n.dataset.c || 0) }; if (!e.shiftKey) mg.anc = { ...mg.act }; mg.hasFocus = true; mg.drawRows(); mg.noteOfActive();
    });
    mg.sc.addEventListener("dblclick", () => mg.startEdit(null));
    mg.sc.addEventListener("copy", e => {
      if (mg.editing) return; const r0 = Math.min(mg.act.r, mg.anc.r), r1 = Math.max(mg.act.r, mg.anc.r), c0 = Math.min(mg.act.c, mg.anc.c), c1 = Math.max(mg.act.c, mg.anc.c), cs = mg.cols(), out = [];
      for (let r = r0; r <= r1; r++) { const line = []; for (let c = c0; c <= c1; c++) line.push(mg.show(mg.doc.campioni[mg.view[r]], cs[c])); out.push(line.join("\t")); }
      e.clipboardData && e.clipboardData.setData("text/plain", out.join("\n")); e.preventDefault();
    });
    mg.sc.addEventListener("paste", e => {
      if (mg.editing || !e.clipboardData) return; const t = e.clipboardData.getData("text/plain"); if (!t) return;
      e.preventDefault(); const t0 = performance.now(), g = tsv(t); if (g.length && !g[g.length - 1].some(x => x)) g.pop(); mg.writeBlock(g);
      mg.say.textContent += ` (${Math.round(performance.now() - t0)} ms)`;
    });
    mg.sc.addEventListener("keydown", e => {
      if (mg.editing || (e.target !== mg.sc && !(/** @type {HTMLElement} */ (e.target)).matches("[data-r]"))) return;
      const k = e.key, mod = e.ctrlKey || e.metaKey, page = Math.floor(VIEW / ROW) - 1;
      const go = (/** @type {number} */ dr, /** @type {number} */ dc) => { e.preventDefault(); mg.move(dr, dc, e.shiftKey); };
      if (k === "ArrowDown") go(1, 0); else if (k === "ArrowUp") go(-1, 0); else if (k === "ArrowRight") go(0, 1); else if (k === "ArrowLeft") go(0, -1);
      else if (k === "PageDown") go(page, 0); else if (k === "PageUp") go(-page, 0);
      else if (k === "Home") go(mod ? -mg.act.r : 0, -mg.act.c); else if (k === "End") go(mod ? mg.view.length : 0, mg.cols().length);
      else if (k === "Enter" || k === "F2") { e.preventDefault(); mg.startEdit(null); }
      else if (k === "Escape") { mg.anc = { ...mg.act }; mg.drawRows(); }
      else if (mod && k.toLowerCase() === "d") { e.preventDefault(); mg.fillDown(); }
      else if (k === "Delete" || k === "Backspace") { e.preventDefault(); mg.clearSel(); }
      else if (k.length === 1 && !mod && !e.altKey) { e.preventDefault(); mg.startEdit(k); }
    });
  };

  /** the panels of the name pattern and of the free columns @param {any} mg */
  const installPanels = mg => {
    const { el } = F;
    /** the panel of the name pattern: preview of the captured parts, then apply */
    mg.patternPanel = () => {
      const m = el("input", { type: "text", value: "{campione_id}", "aria-label": "Name pattern", size: 36 }), out = el("div", { role: "status" });
      /** @type {any} */ let res = null;
      const preview = async () => {
        try {
          res = await call("modello", { modello: m.value, campioni: mg.doc }); out.replaceChildren();
          const rows = res.campioni.campioni.slice(0, 8).map((/** @type {any} */ r, /** @type {number} */ i) => {
            const name = base(r.file), td = el("td", {}); let at = 0;
            for (const p of [...(res.posizioni[i] || [])].sort((a, b) => a.inizio - b.inizio)) { td.append(name.slice(at, p.inizio), el("mark", { text: name.slice(p.inizio, p.fine) }), el("sub", { text: ` ${p.campo}` })); at = p.fine; }
            td.append(name.slice(at)); return el("tr", {}, el("td", { text: name }), td);
          });
          const total = rowsOf(mg.doc).length, miss = res.senza_corrispondenza.length;
          out.append(el("p", { class: "fl-note", text: `${total - miss} of ${total} names match.` + (miss ? ` ${miss} stay as they are.` : "") }),
            el("table", {}, el("caption", { text: "First rows: the captured parts" }), el("thead", {}, el("tr", {}, el("th", { scope: "col", text: "Name" }), el("th", { scope: "col", text: "Parts" }))), el("tbody", {}, ...rows)));
        } catch (e) { res = null; out.textContent = String((/** @type {any} */ (e)).message || e); }
      };
      mg.panel.replaceChildren(el("div", { class: "mg-panel" }, el("label", {}, "Fill from name pattern: ", m),
        el("p", { class: "fl-note", text: "Use {campione_id}, {polarita}, {tempo}, {replica}… or a regular expression with named groups. Only the parts the pattern captures are written." }),
        el("div", { class: "mg-bar" }, F.btn("search", "Preview the captured parts", preview, "Preview"), F.btn("ok", "Write the captured parts into the grid", async () => { if (!res) await preview(); if (res) { mg.commit(res.campioni); mg.panel.replaceChildren(); mg.say.textContent = "Pattern applied."; } }, "Apply"), F.btn("x", "Close the pattern panel", () => mg.panel.replaceChildren(), "Close")), out));
      m.focus();
    };
    /** the panel of the free columns: name, type, values, required */
    mg.columnsPanel = () => {
      const rows = mg.doc.colonne.map((/** @type {any} */ c, /** @type {number} */ k) => {
        const name = el("input", { type: "text", value: c.nome, "aria-label": `Name of column ${c.nome}`, onchange: (/** @type {any} */ e) => mg.rename(k, e.target.value.trim()) });
        const type = el("select", { "aria-label": `Type of column ${c.nome}`, onchange: (/** @type {any} */ e) => mg.setCol(k, { tipo: e.target.value }) }, ...[["testo", "Text"], ["numero", "Number"], ["categoria", "Category"]].map(([v, t]) => el("option", { value: v, text: t, selected: v === c.tipo })));
        const vals = el("input", { type: "text", value: (c.valori || []).join(", "), "aria-label": `Allowed values of column ${c.nome}`, placeholder: "a, b, c", disabled: c.tipo !== "categoria", onchange: (/** @type {any} */ e) => mg.setCol(k, { valori: e.target.value.split(",").map((/** @type {string} */ x) => x.trim()).filter(Boolean) }) });
        const req = el("input", { type: "checkbox", checked: !!c.obbligatoria, "aria-label": `Column ${c.nome} is required`, onchange: (/** @type {any} */ e) => mg.setCol(k, { obbligatoria: e.target.checked }) });
        return el("tr", {}, el("td", {}, name), el("td", {}, type), el("td", {}, vals), el("td", {}, req), el("td", {}, F.btn("x", `Delete column ${c.nome}`, () => { const nome = c.nome; mg.commit({ ...mg.doc, colonne: mg.doc.colonne.filter((/** @type {any} */ _, /** @type {number} */ q) => q !== k), campioni: mg.doc.campioni.map((/** @type {any} */ r) => { const { [nome]: _gone, ...extra } = r.extra; return { ...r, extra }; }) }); mg.fillSelects(); mg.columnsPanel(); }, "Delete")));
      });
      const nn = el("input", { type: "text", "aria-label": "Name of the new column", placeholder: "New column" }), nt = el("select", { "aria-label": "Type of the new column" }, el("option", { value: "testo", text: "Text" }), el("option", { value: "numero", text: "Number" }), el("option", { value: "categoria", text: "Category" }));
      mg.panel.replaceChildren(el("div", { class: "mg-panel" }, el("h5", { text: "Free columns" }),
        rows.length ? el("table", {}, el("thead", {}, el("tr", {}, ...["Name", "Type", "Allowed values", "Required", ""].map(h => el("th", { scope: "col", text: h })))), el("tbody", {}, ...rows)) : el("p", { class: "fl-note", text: "No free column yet." }),
        el("div", { class: "mg-bar" }, nn, nt, F.btn("plus", "Add the column", () => { const nome = nn.value.trim(); if (!nome || mg.doc.colonne.some((/** @type {any} */ c) => c.nome === nome)) { mg.say.textContent = nome ? `A column named ${nome} exists.` : "Type a name."; return; }
          mg.commit({ ...mg.doc, colonne: [...mg.doc.colonne, { nome, tipo: nt.value }] }); mg.fillSelects(); mg.columnsPanel(); }, "Add column"), F.btn("x", "Close the columns panel", () => mg.panel.replaceChildren(), "Close"))));
    };
    mg.setCol = (/** @type {number} */ k, /** @type {any} */ p) => { mg.commit({ ...mg.doc, colonne: mg.doc.colonne.map((/** @type {any} */ c, /** @type {number} */ q) => (q === k ? { ...c, ...p } : c)) }); mg.columnsPanel(); };
    mg.rename = (/** @type {number} */ k, /** @type {string} */ to) => {
      const from = mg.doc.colonne[k].nome; if (!to || to === from || mg.doc.colonne.some((/** @type {any} */ c) => c.nome === to)) { mg.columnsPanel(); return; }
      mg.commit({ ...mg.doc, colonne: mg.doc.colonne.map((/** @type {any} */ c, /** @type {number} */ q) => (q === k ? { ...c, nome: to } : c)), campioni: mg.doc.campioni.map((/** @type {any} */ r) => { if (!(from in r.extra)) return r; const { [from]: v, ...extra } = r.extra; return { ...r, extra: { ...extra, [to]: v } }; }) });
      mg.fillSelects(); mg.columnsPanel();
    };
  };

  /** the elements of the bar: CSV, grouping, filters and the actions behind them @param {any} mg */
  const installBar = mg => {
    const { el } = F;
    mg.bar = el("div", { class: "mg-bar" });
    mg.chips = el("div", { class: "mg-chips" });
    mg.panel = el("div");
    mg.fileIn = el("input", { type: "file", accept: ".csv,.tsv,.txt", hidden: true, "aria-label": "Sample table (CSV or TSV)", onchange: async (/** @type {any} */ e) => {
      const f = e.target.files && e.target.files[0]; e.target.value = ""; if (!f) return;
      try { const r = await F.metaImport(mg.S, mg.input, await f.text()); mg.commit(r.doc); mg.say.textContent = `${r.count} rows read from ${f.name}.` + (r.absent ? ` ${r.absent} not in Data.` : ""); } catch (err) { mg.say.textContent = `${f.name}: ${(/** @type {any} */ (err)).message || err}`; }
    } });
    mg.sepSel = el("select", { "aria-label": "CSV separator" }, el("option", { value: ",", text: "Comma" }), el("option", { value: ";", text: "Semicolon" }), el("option", { value: "\t", text: "Tab" }));
    mg.groupSel = el("select", { "aria-label": "Group rows by", onchange: (/** @type {any} */ e) => { mg.group = e.target.value; mg.refresh(); } });
    mg.fcol = el("select", { "aria-label": "Filter column" });
    mg.ftext = el("input", { type: "text", "aria-label": "Filter text", placeholder: "contains…", onkeydown: (/** @type {KeyboardEvent} */ e) => { if (e.key === "Enter") mg.addFilter(); } });
    mg.addFilter = () => { const t = mg.ftext.value.trim(); if (!t) return; mg.filters.push({ id: mg.fcol.value, text: t }); mg.ftext.value = ""; mg.chipsDraw(); mg.refresh(); };
    mg.chipsDraw = () => mg.chips.replaceChildren(...mg.filters.map((f, k) => el("span", { class: "mg-chip" }, el("span", { text: `${(mg.cols().find(c => c.id === f.id) || { label: f.id }).label}: ${f.text}` }),
      el("button", { type: "button", class: "fl-b", "aria-label": `Remove filter ${(mg.cols().find(c => c.id === f.id) || { label: f.id }).label} ${f.text}`, onclick: () => { mg.filters.splice(k, 1); mg.chipsDraw(); mg.refresh(); } }, F.icon("x", 12)))));
    mg.fillSelects = () => {
      const cs = mg.cols().filter(c => c.kind !== "ro" || c.id === "polarita");
      mg.fcol.replaceChildren(...cs.map(c => el("option", { value: c.id, text: c.label })));
      mg.groupSel.replaceChildren(el("option", { value: "", text: "No grouping" }), ...["matrice", "gruppo", "serie"].map(id => el("option", { value: id, text: "Group by " + (FIXED.find(f => f.id === id) || { label: id }).label.toLowerCase(), selected: mg.group === id })));
    };
    mg.takeFromData = async () => {
      try { const d = await fromData(); if (!d) { mg.say.textContent = "No file is open in the Data tab."; return; } mg.commit(d); mg.say.textContent = "Roles, times and the data of the files taken from Data."; } catch (e) { mg.say.textContent = String((/** @type {any} */ (e)).message || e); }
    };
    mg.readFiles = async () => {
      const info = fileInfo(); if (!info.length) { mg.say.textContent = "No file is open in the Data tab."; return; }
      try { const r = await call("dal_file", { campioni: mg.doc, file: info }), missing = r.senza_campione; mg.commit(r.campioni); mg.say.textContent = `File data read.` + (r.discordanze.length ? ` ${r.discordanze.length} values replaced.` : "") + (missing.length ? ` ${missing.length} files have no row.` : ""); } catch (e) { mg.say.textContent = String((/** @type {any} */ (e)).message || e); }
    };
    mg.exportCsv = async () => {
      try { const r = await call("csv_scrivi", { campioni: mg.doc, separatore: mg.sepSel.value }); G.ATTRIBUTIO_CORE.download(`${mg.input.name || "samples"}.csv`, r.testo, "text/csv;charset=utf-8"); mg.say.textContent = `${rowsOf(mg.doc).length} rows exported.`; } catch (e) { mg.say.textContent = String((/** @type {any} */ (e)).message || e); }
    };
  };

  /** puts the bar and the grid together and starts with the document of the plan or of Data @param {any} mg */
  const installFinish = mg => {
    const { el } = F;
    mg.sha = el("code", { title: "", text: "not saved yet" });
    mg.gen = F.btn("import", "Take the roles, times and data of the files open in Data", mg.takeFromData, "Generate metadata");
    if (!F.dataFiles().length) { /** @type {HTMLButtonElement} */ (mg.gen).disabled = true; mg.gen.title = "No file is open in the Data tab"; }
    mg.bar.append(mg.gen, F.btn("file", "Read polarity, ranges and scans from the files open in Data", mg.readFiles, "Read file data"),
      F.btn("plus", "Add an empty row", () => { mg.commit({ ...mg.doc, campioni: [...mg.doc.campioni, blankRow(mg.doc.campioni.length + 1)] }); mg.act = { r: mg.view.length - 1, c: 0 }; mg.anc = { ...mg.act }; mg.move(0, 0, false); }, "Add row"),
      F.btn("minus", "Remove the selected rows", () => { const r0 = Math.min(mg.act.r, mg.anc.r), r1 = Math.max(mg.act.r, mg.anc.r), gone = new Set(mg.view.slice(r0, r1 + 1)); mg.commit({ ...mg.doc, campioni: mg.doc.campioni.filter((/** @type {any} */ _, /** @type {number} */ i) => !gone.has(i)) }); }, "Remove rows"),
      F.btn("search", "Fill the grid from the file names with a pattern", mg.patternPanel, "Fill from name pattern…"), F.btn("flow", "Add, rename, retype or delete free columns", mg.columnsPanel, "Columns…"),
      F.btn("import", "Read a CSV or TSV table", () => mg.fileIn.click(), "Import CSV"), mg.fileIn, mg.sepSel, F.btn("export", "Download the grid as CSV", mg.exportCsv, "Export CSV"),
      F.btn("export", "Download the sample list as JSON", () => { const j = mg.S.samplesJson && mg.S.samplesJson[mg.input.name]; if (j) G.ATTRIBUTIO_CORE.download(mg.input.name, j, "application/json"); }, "Download JSON"), el("span", { class: "fl-note", text: "SHA-256 " }), mg.sha);
    mg.root.append(mg.bar, el("div", { class: "mg-bar" }, mg.groupSel, el("label", { class: "fl-note" }, "Filter: ", mg.fcol), mg.ftext, F.btn("plus", "Add the filter", mg.addFilter, "Filter")), mg.chips, mg.panel, mg.say,
      el("p", { class: "fl-note", text: "Arrows move, Enter or typing edits, Tab moves right while editing, Ctrl+D fills down, Ctrl+V pastes a block from a spreadsheet." }), mg.sc, mg.note);

    // start: the document of the plan, else the files open in Data (the grid opens by itself)
    (async () => {
      try {
        const own = mg.S.samples && mg.S.samples[mg.input.name];
        mg.doc = own ? await toV3(own) : (await fromData()) || emptyDoc();
        if (!own && mg.doc.campioni.length) mg.say.textContent = "Taken from the files open in Data.";
        mg.fillSelects(); mg.chipsDraw(); mg.commit(mg.doc);
      } catch (e) { mg.say.textContent = String((/** @type {any} */ (e)).message || e); }
    })();
  };

  /** the grid of one sample list @param {any} S @param {any} input the `samples` input of the plan @returns {HTMLElement} */
  F.metaGrid = (S, input) => {
    if (!document.getElementById("mg-css")) { const st = document.createElement("style"); st.id = "mg-css"; st.textContent = CSS; document.head.append(st); }
    const mg = { S, input, root: F.el("div", { class: "mg" }) };
    for (const install of [installState, installCells, installEdit, installMove, installEvents, installPanels, installBar, installFinish]) install(mg);
    return mg.root;
  };
})();
