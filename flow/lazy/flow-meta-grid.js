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

  /** the grid of one sample list @param {any} S @param {any} input the `samples` input of the plan @returns {HTMLElement} */
  F.metaGrid = (S, input) => {
    if (!document.getElementById("mg-css")) { const st = document.createElement("style"); st.id = "mg-css"; st.textContent = CSS; document.head.append(st); }
    const { el } = F, root = el("div", { class: "mg" });
    const say = el("p", { class: "fl-note", role: "status" }), note = el("p", { class: "fl-note", role: "status", "aria-live": "polite" });
    let doc = /** @type {any} */ (emptyDoc()), ver = 0, timer = 0, warns = /** @type {Map<string, {code: string, sev: string}[]>} */ (new Map()), pairRows = /** @type {Set<number>} */ (new Set());
    let view = /** @type {number[]} */ ([]), gidx = /** @type {number[]} */ ([]), sort = /** @type {{id: string, dir: number} | null} */ (null), group = "", filters = /** @type {{id: string, text: string}[]} */ ([]);
    let act = { r: 0, c: 0 }, anc = { r: 0, c: 0 }, editing = /** @type {{r: number, c: number} | null} */ (null), hasFocus = false, summary = "";
    const cols = () => [...FIXED, ...doc.colonne.map((/** @type {any} */ c) => ({ id: "x:" + c.nome, label: c.nome, w: 110, kind: c.tipo === "categoria" ? "xcat" : c.tipo === "numero" ? "xnum" : "text", opts: c.valori }))].concat(FROMFILE);

    /** the text a cell shows @param {any} r @param {any} c */
    const show = (r, c) => {
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
    const put = (r, c, text) => {
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

    const recompute = () => {
      const cs = cols(), idx = doc.campioni.map((/** @type {any} */ _, /** @type {number} */ i) => i).filter((/** @type {number} */ i) => filters.every(f => {
        const c = cs.find(x => x.id === f.id); return !c || show(doc.campioni[i], c).toLowerCase().includes(f.text.toLowerCase());
      }));
      const by = (/** @type {string} */ id, /** @type {number} */ dir) => { const c = cs.find(x => x.id === id); if (!c) return; const num = c.kind === "num" || c.kind === "int" || c.kind === "xnum";
        idx.sort((/** @type {number} */ a, /** @type {number} */ b) => { const x = show(doc.campioni[a], c), y = show(doc.campioni[b], c); return dir * (num && x !== "" && y !== "" ? Number(x) - Number(y) : x.localeCompare(y, undefined, { numeric: true })); }); };
      if (sort) by(sort.id, sort.dir);
      if (group) by(group, 1);
      view = idx; gidx = []; let g = 0;
      const gc = cs.find(x => x.id === group);
      view.forEach((i, k) => { if (gc && k && show(doc.campioni[i], gc) !== show(doc.campioni[view[k - 1]], gc)) g++; gidx.push(g); });
      act.r = Math.min(act.r, Math.max(0, view.length - 1)); anc.r = Math.min(anc.r, Math.max(0, view.length - 1));
    };

    /** the findings of `valida` by cell: row|column id @param {any} avvisi */
    const indexWarns = avvisi => {
      warns = new Map(); pairRows = new Set(); let e = 0, w = 0, n = 0; const series = /** @type {string[]} */ ([]);
      for (const a of avvisi) {
        if (a.gravita === "error") e++; else if (a.gravita === "warning") w++; else n++;
        if (a.file == null) { if (a.gravita !== "info") series.push(`${MSG[a.codice] || a.codice}${a.serie ? " in " + a.serie : ""}`); continue; }
        const col = a.colonna ? (FIXED.some(f => f.id === a.colonna) ? a.colonna : "x:" + a.colonna) : CELL_OF[a.codice] || "file", k = a.file + "|" + col;
        warns.set(k, [...(warns.get(k) || []), { code: a.codice, sev: a.gravita }]);
        if (a.codice === "pairMissing") pairRows.add(a.file);
      }
      summary = `${e} errors, ${w} warnings, ${n} notes.` + (series.length ? " " + [...new Set(series)].join("; ") + "." : "");
    };
    const warnsOf = (/** @type {number} */ row, /** @type {string} */ col) => warns.get(row + "|" + col) || [];

    // the elements
    const sc = el("div", { class: "mg-sc", role: "grid", "aria-label": "Sample metadata", "aria-multiselectable": "true" }), head = el("div", { class: "mg-hd", role: "rowgroup" }), bd = el("div", { class: "mg-bd", role: "rowgroup" });
    sc.append(head, bd);
    const cellEl = (/** @type {number} */ r, /** @type {number} */ c) => /** @type {HTMLElement | null} */ (bd.querySelector(`[data-r="${r}"][data-c="${c}"]`));
    const inSel = (/** @type {number} */ r, /** @type {number} */ c) => r >= Math.min(act.r, anc.r) && r <= Math.max(act.r, anc.r) && c >= Math.min(act.c, anc.c) && c <= Math.max(act.c, anc.c);

    const drawHead = () => {
      const cs = cols(), tr = el("div", { class: "mg-r", role: "row", "aria-rowindex": 1 });
      cs.forEach((c, i) => {
        const on = sort && sort.id === c.id, b = el("button", { type: "button", "aria-label": `Sort by ${c.label}`, onclick: () => { sort = on && sort && sort.dir === -1 ? null : { id: c.id, dir: on ? -1 : 1 }; refresh(); } },
          el("span", { text: c.label }), on ? el("span", { "aria-hidden": "true", text: sort && sort.dir === 1 ? "▲" : "▼" }) : null);
        tr.append(el("div", { class: "mg-c mg-h", role: "columnheader", "aria-colindex": i + 1, "aria-sort": on && sort ? (sort.dir === 1 ? "ascending" : "descending") : "none", style: `width:${c.w}px` }, b));
      });
      head.replaceChildren(tr); head.style.width = cs.reduce((s, c) => s + c.w, 0) + "px";
    };

    const drawRows = () => {
      const keep = hasFocus, cs = cols(), first = Math.max(0, Math.floor(sc.scrollTop / ROW) - OVER), last = Math.min(view.length - 1, first + Math.ceil(VIEW / ROW) + 2 * OVER), frag = document.createDocumentFragment();
      const rows = []; for (let k = first; k <= last; k++) rows.push(k);
      if (act.r < first || act.r > last) rows.push(act.r);                     // the active row always exists: it holds the tab stop
      for (const k of view.length ? rows : []) {
        const i = view[k], r = doc.campioni[i], gs = group && k && gidx[k] !== gidx[k - 1];
        const tr = el("div", { class: `mg-r${gidx[k] % 2 ? " mg-g1" : ""}${gs ? " mg-gs" : ""}${pairRows.has(i) ? " mg-pair" : ""}`, role: "row", "aria-rowindex": k + 2, style: `top:${k * ROW}px` });
        cs.forEach((c, j) => {
          const w = warnsOf(i, c.id), bad = w.find(x => x.sev === "error"), txt = show(r, c), isAct = act.r === k && act.c === j;
          const cell = el("div", { class: `mg-c${c.kind === "ro" ? " mg-ro" : ""}${inSel(k, j) ? " mg-sel" : ""}${isAct ? " mg-act" : ""}${w.length ? (bad ? " mg-err" : " mg-wrn") : ""}`, role: "gridcell", "aria-colindex": j + 1, "aria-selected": inSel(k, j) ? "true" : "false",
            "aria-invalid": bad ? "true" : null, "aria-readonly": c.kind === "ro" ? "true" : null, tabindex: isAct ? 0 : -1, "data-r": k, "data-c": j, style: `width:${c.w}px`, title: w.map(x => MSG[x.code] || x.code).join("; ") || txt });
          if (editing && editing.r === k && editing.c === j) cell.append(editor(r, c)); else { if (w.length) cell.append(el("span", { "aria-hidden": "true", text: bad ? "✖" : "⚠" })); cell.append(document.createTextNode(txt)); }
          tr.append(cell);
        });
        frag.append(tr);
      }
      bd.replaceChildren(frag); bd.style.height = view.length * ROW + "px"; bd.style.width = cs.reduce((s, c) => s + c.w, 0) + "px";
      sc.setAttribute("aria-rowcount", String(view.length + 1)); sc.setAttribute("aria-colcount", String(cs.length));
      sc.tabIndex = view.length ? -1 : 0;
      if (keep) { const n = editing ? bd.querySelector("input,select") : cellEl(act.r, act.c); if (n) { /** @type {HTMLElement} */ (n).focus({ preventScroll: true }); hasFocus = true; } }
    };

    /** the in-cell editor: text, number or a list @param {any} r @param {any} c */
    const editor = (r, c) => {
      const label = `${c.label} of ${r.file || "row " + (act.r + 1)}`, v = editing && /** @type {any} */ (editing).seed != null ? /** @type {any} */ (editing).seed : show(r, c);
      const opts = c.kind === "cat" ? c.opts : c.kind === "xcat" ? c.opts : null;
      const hit = editing && /** @type {any} */ (editing).seed && opts ? opts.find((/** @type {string} */ o) => o.toLowerCase().startsWith(String(/** @type {any} */ (editing).seed).toLowerCase())) : null;      // a typed letter picks the first value that starts with it
      if (opts) return el("select", { "aria-label": label, onchange: (/** @type {any} */ e) => finish(e.target.value, 1, 0), onkeydown: editKeys }, ...[...(c.kind === "xcat" ? [""] : []), ...opts].map((/** @type {string} */ o) => el("option", { value: o, text: o, selected: hit ? o === hit : o === show(r, c) })));
      return el("input", { type: "text", value: v, inputMode: c.kind === "num" || c.kind === "int" || c.kind === "xnum" ? "decimal" : "text", "aria-label": label, onkeydown: editKeys, onblur: (/** @type {any} */ e) => { if (editing) finish(e.target.value, 0, 0); } });
    };
    /** @param {KeyboardEvent} e */
    const editKeys = e => {
      const t = /** @type {HTMLInputElement} */ (e.target);
      if (e.key === "Enter") { e.preventDefault(); finish(t.value, e.shiftKey ? -1 : 1, 0); }
      else if (e.key === "Tab") { e.preventDefault(); finish(t.value, 0, e.shiftKey ? -1 : 1); }
      else if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); editing = null; drawRows(); }
    };
    /** commits the editor and moves the active cell by (dr, dc) @param {string} text @param {number} dr @param {number} dc */
    const finish = (text, dr, dc) => {
      if (!editing) return;
      const { r, c } = editing, col = cols()[c], i = view[r]; editing = null;
      const next = put(doc.campioni[i], col, text);
      if (next === null) say.textContent = `“${text}” does not fit ${col.label}.`; else if (JSON.stringify(next) !== JSON.stringify(doc.campioni[i])) commit(patch(doc, [[i, next]]));
      move(dr, dc, false);
    };
    const patch = (/** @type {any} */ d, /** @type {[number, any][]} */ rows) => { const cs = [...d.campioni]; for (const [i, r] of rows) cs[i] = r; return { ...d, campioni: cs }; };

    /** keeps `d` as the document: shown at once, hashed and checked a moment later @param {any} d */
    const commit = d => {
      doc = d; S.samples = S.samples || {}; S.samples[input.name] = doc; const mine = ++ver;
      recompute(); drawHead(); drawRows(); noteOfActive();
      clearTimeout(timer);
      timer = window.setTimeout(async () => {
        try {
          await F.samplesSet(S, input, doc); sha.textContent = input.sha256 ? input.sha256.slice(0, 12) + "…" : "not saved yet"; sha.title = input.sha256 || "";
          const w = await call("valida", doc);
          if (mine === ver) { indexWarns(w.avvisi); say.textContent = summary; drawRows(); noteOfActive(); }
        } catch (e) { say.textContent = "The list could not be checked: " + ((/** @type {any} */ (e)).message || e); }
      }, 120);
    };
    const refresh = () => { recompute(); drawHead(); drawRows(); noteOfActive(); };

    const noteOfActive = () => {
      const i = view[act.r], c = cols()[act.c]; if (i == null || !c) { note.textContent = ""; return; }
      const w = warnsOf(i, c.id); note.textContent = w.length ? `${c.label}, ${doc.campioni[i].file || "row " + (act.r + 1)}: ${w.map(x => `${x.sev === "error" ? "error" : "warning"}, ${MSG[x.code] || x.code}`).join("; ")}` : "";
    };
    /** moves the active cell; `keep` leaves the anchor (a range) @param {number} dr @param {number} dc @param {boolean} keep */
    const move = (dr, dc, keep) => {
      act = { r: Math.max(0, Math.min(view.length - 1, act.r + dr)), c: Math.max(0, Math.min(cols().length - 1, act.c + dc)) };
      if (!keep) anc = { ...act };
      const top = act.r * ROW, h = VIEW - ROW * 2;
      if (top < sc.scrollTop) sc.scrollTop = top; else if (top + ROW > sc.scrollTop + h) sc.scrollTop = top + ROW - h;
      const w = cols()[act.c].w; let left = 0; cols().slice(0, act.c).forEach(c => { left += c.w; });
      if (left < sc.scrollLeft) sc.scrollLeft = left; else if (left + w > sc.scrollLeft + sc.clientWidth) sc.scrollLeft = left + w - sc.clientWidth;
      drawRows(); noteOfActive();
    };
    const startEdit = (/** @type {string | null} */ seed) => { const c = cols()[act.c]; if (!view.length || c.kind === "ro") return; editing = /** @type {any} */ ({ ...act, seed }); drawRows(); const n = /** @type {HTMLInputElement | null} */ (bd.querySelector("input")); if (n && seed != null) n.setSelectionRange(seed.length, seed.length); };

    /** writes `grid[k][j]` into the cells from the active one: the rows seen, the columns after it; read-only cells are skipped @param {string[][]} grid */
    const writeBlock = grid => {
      const cs = cols(), rows = new Map(); let n = 0, cut = 0;
      grid.forEach((line, k) => line.forEach((t, j) => {
        const r = act.r + k, c = act.c + j; if (r >= view.length || c >= cs.length) { cut++; return; }
        const i = view[r], cur = rows.get(i) || doc.campioni[i], next = put(cur, cs[c], t);
        if (next) { rows.set(i, next); n++; } else if (cs[c].kind !== "ro") cut++;
      }));
      if (rows.size) commit(patch(doc, [...rows.entries()]));
      say.textContent = `${n} cells written` + (cut ? `, ${cut} left out (read-only, past the end or not fitting the column).` : ".");
    };
    /** Ctrl+D: the first row of the selection (or the row above) copied down the selected columns */
    const fillDown = () => {
      const r0 = Math.min(act.r, anc.r), r1 = Math.max(act.r, anc.r), c0 = Math.min(act.c, anc.c), c1 = Math.max(act.c, anc.c), cs = cols(), from = r1 > r0 ? r0 : r0 - 1, to = r1 > r0 ? r1 : r0;
      if (from < 0) return;
      /** @type {Map<number, any>} */ const rows = new Map();
      for (let r = from + 1; r <= to; r++) for (let c = c0; c <= c1; c++) { const i = view[r], cur = rows.get(i) || doc.campioni[i], next = put(cur, cs[c], show(doc.campioni[view[from]], cs[c])); if (next) rows.set(i, next); }
      if (rows.size) commit(patch(doc, [...rows.entries()])); say.textContent = `Filled ${to - from} rows down.`;
    };
    const clearSel = () => {
      const r0 = Math.min(act.r, anc.r), r1 = Math.max(act.r, anc.r), c0 = Math.min(act.c, anc.c), c1 = Math.max(act.c, anc.c), cs = cols(); /** @type {Map<number, any>} */ const rows = new Map();
      for (let r = r0; r <= r1; r++) for (let c = c0; c <= c1; c++) { const i = view[r], next = put(rows.get(i) || doc.campioni[i], cs[c], cs[c].kind === "cat" ? doc.campioni[i][cs[c].id] : ""); if (next) rows.set(i, next); }
      if (rows.size) commit(patch(doc, [...rows.entries()]));
    };

    sc.addEventListener("scroll", () => { if (!editing) requestAnimationFrame(drawRows); });
    sc.addEventListener("focusin", e => {
      hasFocus = true; const n = /** @type {HTMLElement | null} */ (editing ? null : (/** @type {HTMLElement} */ (e.target)).closest("[data-r]"));       // Tab or focus() into a cell makes it the active one
      if (n && (+(n.dataset.r || 0) !== act.r || +(n.dataset.c || 0) !== act.c)) { act = { r: +(n.dataset.r || 0), c: +(n.dataset.c || 0) }; anc = { ...act }; drawRows(); noteOfActive(); }
    });
    sc.addEventListener("focusout", e => { if (!sc.contains(/** @type {Node | null} */ (e.relatedTarget))) hasFocus = false; });
    sc.addEventListener("mousedown", e => {
      const n = /** @type {HTMLElement | null} */ ((/** @type {HTMLElement} */ (e.target)).closest("[data-r]")); if (!n || editing) return;
      act = { r: +(n.dataset.r || 0), c: +(n.dataset.c || 0) }; if (!e.shiftKey) anc = { ...act }; hasFocus = true; drawRows(); noteOfActive();
    });
    sc.addEventListener("dblclick", () => startEdit(null));
    sc.addEventListener("copy", e => {
      if (editing) return; const r0 = Math.min(act.r, anc.r), r1 = Math.max(act.r, anc.r), c0 = Math.min(act.c, anc.c), c1 = Math.max(act.c, anc.c), cs = cols(), out = [];
      for (let r = r0; r <= r1; r++) { const line = []; for (let c = c0; c <= c1; c++) line.push(show(doc.campioni[view[r]], cs[c])); out.push(line.join("\t")); }
      e.clipboardData && e.clipboardData.setData("text/plain", out.join("\n")); e.preventDefault();
    });
    sc.addEventListener("paste", e => {
      if (editing || !e.clipboardData) return; const t = e.clipboardData.getData("text/plain"); if (!t) return;
      e.preventDefault(); const t0 = performance.now(), g = tsv(t); if (g.length && !g[g.length - 1].some(x => x)) g.pop(); writeBlock(g);
      say.textContent += ` (${Math.round(performance.now() - t0)} ms)`;
    });
    sc.addEventListener("keydown", e => {
      if (editing || (e.target !== sc && !(/** @type {HTMLElement} */ (e.target)).matches("[data-r]"))) return;
      const k = e.key, mod = e.ctrlKey || e.metaKey, page = Math.floor(VIEW / ROW) - 1;
      const go = (/** @type {number} */ dr, /** @type {number} */ dc) => { e.preventDefault(); move(dr, dc, e.shiftKey); };
      if (k === "ArrowDown") go(1, 0); else if (k === "ArrowUp") go(-1, 0); else if (k === "ArrowRight") go(0, 1); else if (k === "ArrowLeft") go(0, -1);
      else if (k === "PageDown") go(page, 0); else if (k === "PageUp") go(-page, 0);
      else if (k === "Home") go(mod ? -act.r : 0, -act.c); else if (k === "End") go(mod ? view.length : 0, cols().length);
      else if (k === "Enter" || k === "F2") { e.preventDefault(); startEdit(null); }
      else if (k === "Escape") { anc = { ...act }; drawRows(); }
      else if (mod && k.toLowerCase() === "d") { e.preventDefault(); fillDown(); }
      else if (k === "Delete" || k === "Backspace") { e.preventDefault(); clearSel(); }
      else if (k.length === 1 && !mod && !e.altKey) { e.preventDefault(); startEdit(k); }
    });

    // the bar: files, CSV, name pattern, columns, grouping, filters
    const bar = el("div", { class: "mg-bar" }), chips = el("div", { class: "mg-chips" }), panel = el("div");
    const fileIn = el("input", { type: "file", accept: ".csv,.tsv,.txt", hidden: true, "aria-label": "Sample table (CSV or TSV)", onchange: async (/** @type {any} */ e) => {
      const f = e.target.files && e.target.files[0]; e.target.value = ""; if (!f) return;
      try { const r = await F.metaImport(S, input, await f.text()); commit(r.doc); say.textContent = `${r.count} rows read from ${f.name}.` + (r.absent ? ` ${r.absent} not in Data.` : ""); } catch (err) { say.textContent = `${f.name}: ${(/** @type {any} */ (err)).message || err}`; }
    } });
    const sepSel = el("select", { "aria-label": "CSV separator" }, el("option", { value: ",", text: "Comma" }), el("option", { value: ";", text: "Semicolon" }), el("option", { value: "\t", text: "Tab" }));
    const groupSel = el("select", { "aria-label": "Group rows by", onchange: (/** @type {any} */ e) => { group = e.target.value; refresh(); } });
    const fcol = el("select", { "aria-label": "Filter column" }), ftext = el("input", { type: "text", "aria-label": "Filter text", placeholder: "contains…", onkeydown: (/** @type {KeyboardEvent} */ e) => { if (e.key === "Enter") addFilter(); } });
    const addFilter = () => { const t = ftext.value.trim(); if (!t) return; filters.push({ id: fcol.value, text: t }); ftext.value = ""; chipsDraw(); refresh(); };
    const chipsDraw = () => chips.replaceChildren(...filters.map((f, k) => el("span", { class: "mg-chip" }, el("span", { text: `${(cols().find(c => c.id === f.id) || { label: f.id }).label}: ${f.text}` }),
      el("button", { type: "button", class: "fl-b", "aria-label": `Remove filter ${(cols().find(c => c.id === f.id) || { label: f.id }).label} ${f.text}`, onclick: () => { filters.splice(k, 1); chipsDraw(); refresh(); } }, F.icon("x", 12)))));
    const fillSelects = () => {
      const cs = cols().filter(c => c.kind !== "ro" || c.id === "polarita");
      fcol.replaceChildren(...cs.map(c => el("option", { value: c.id, text: c.label })));
      groupSel.replaceChildren(el("option", { value: "", text: "No grouping" }), ...["matrice", "gruppo", "serie"].map(id => el("option", { value: id, text: "Group by " + (FIXED.find(f => f.id === id) || { label: id }).label.toLowerCase(), selected: group === id })));
    };
    const takeFromData = async () => {
      try { const d = await fromData(); if (!d) { say.textContent = "No file is open in the Data tab."; return; } commit(d); say.textContent = "Roles, times and the data of the files taken from Data."; } catch (e) { say.textContent = String((/** @type {any} */ (e)).message || e); }
    };
    const readFiles = async () => {
      const info = fileInfo(); if (!info.length) { say.textContent = "No file is open in the Data tab."; return; }
      try { const r = await call("dal_file", { campioni: doc, file: info }); commit(r.campioni); say.textContent = `File data read.` + (r.discordanze.length ? ` ${r.discordanze.length} values replaced.` : "") + (r.senza_campione.length ? ` ${r.senza_campione.length} files have no row.` : ""); } catch (e) { say.textContent = String((/** @type {any} */ (e)).message || e); }
    };
    const exportCsv = async () => {
      try { const r = await call("csv_scrivi", { campioni: doc, separatore: sepSel.value }); G.ATTRIBUTIO_CORE.download(`${input.name || "samples"}.csv`, r.testo, "text/csv;charset=utf-8"); say.textContent = `${doc.campioni.length} rows exported.`; } catch (e) { say.textContent = String((/** @type {any} */ (e)).message || e); }
    };

    /** the panel of the name pattern: preview of the captured parts, then apply */
    const patternPanel = () => {
      const m = el("input", { type: "text", value: "{campione_id}", "aria-label": "Name pattern", size: 36 }), out = el("div", { role: "status" });
      /** @type {any} */ let res = null;
      const preview = async () => {
        try {
          res = await call("modello", { modello: m.value, campioni: doc }); out.replaceChildren();
          const rows = res.campioni.campioni.slice(0, 8).map((/** @type {any} */ r, /** @type {number} */ i) => {
            const name = base(r.file), td = el("td", {}); let at = 0;
            for (const p of [...(res.posizioni[i] || [])].sort((a, b) => a.inizio - b.inizio)) { td.append(name.slice(at, p.inizio), el("mark", { text: name.slice(p.inizio, p.fine) }), el("sub", { text: ` ${p.campo}` })); at = p.fine; }
            td.append(name.slice(at)); return el("tr", {}, el("td", { text: name }), td);
          });
          out.append(el("p", { class: "fl-note", text: `${doc.campioni.length - res.senza_corrispondenza.length} of ${doc.campioni.length} names match.` + (res.senza_corrispondenza.length ? ` ${res.senza_corrispondenza.length} stay as they are.` : "") }),
            el("table", {}, el("caption", { text: "First rows: the captured parts" }), el("thead", {}, el("tr", {}, el("th", { scope: "col", text: "Name" }), el("th", { scope: "col", text: "Parts" }))), el("tbody", {}, ...rows)));
        } catch (e) { res = null; out.textContent = String((/** @type {any} */ (e)).message || e); }
      };
      panel.replaceChildren(el("div", { class: "mg-panel" }, el("label", {}, "Fill from name pattern: ", m),
        el("p", { class: "fl-note", text: "Use {campione_id}, {polarita}, {tempo}, {replica}… or a regular expression with named groups. Only the parts the pattern captures are written." }),
        el("div", { class: "mg-bar" }, F.btn("search", "Preview the captured parts", preview, "Preview"), F.btn("ok", "Write the captured parts into the grid", async () => { if (!res) await preview(); if (res) { commit(res.campioni); panel.replaceChildren(); say.textContent = "Pattern applied."; } }, "Apply"), F.btn("x", "Close the pattern panel", () => panel.replaceChildren(), "Close")), out));
      m.focus();
    };
    /** the panel of the free columns: name, type, values, required */
    const columnsPanel = () => {
      const rows = doc.colonne.map((/** @type {any} */ c, /** @type {number} */ k) => {
        const name = el("input", { type: "text", value: c.nome, "aria-label": `Name of column ${c.nome}`, onchange: (/** @type {any} */ e) => rename(k, e.target.value.trim()) });
        const type = el("select", { "aria-label": `Type of column ${c.nome}`, onchange: (/** @type {any} */ e) => setCol(k, { tipo: e.target.value }) }, ...[["testo", "Text"], ["numero", "Number"], ["categoria", "Category"]].map(([v, t]) => el("option", { value: v, text: t, selected: v === c.tipo })));
        const vals = el("input", { type: "text", value: (c.valori || []).join(", "), "aria-label": `Allowed values of column ${c.nome}`, placeholder: "a, b, c", disabled: c.tipo !== "categoria", onchange: (/** @type {any} */ e) => setCol(k, { valori: e.target.value.split(",").map((/** @type {string} */ x) => x.trim()).filter(Boolean) }) });
        const req = el("input", { type: "checkbox", checked: !!c.obbligatoria, "aria-label": `Column ${c.nome} is required`, onchange: (/** @type {any} */ e) => setCol(k, { obbligatoria: e.target.checked }) });
        return el("tr", {}, el("td", {}, name), el("td", {}, type), el("td", {}, vals), el("td", {}, req), el("td", {}, F.btn("x", `Delete column ${c.nome}`, () => { const nome = c.nome; commit({ ...doc, colonne: doc.colonne.filter((/** @type {any} */ _, /** @type {number} */ q) => q !== k), campioni: doc.campioni.map((/** @type {any} */ r) => { const { [nome]: _gone, ...extra } = r.extra; return { ...r, extra }; }) }); fillSelects(); columnsPanel(); }, "Delete")));
      });
      const nn = el("input", { type: "text", "aria-label": "Name of the new column", placeholder: "New column" }), nt = el("select", { "aria-label": "Type of the new column" }, el("option", { value: "testo", text: "Text" }), el("option", { value: "numero", text: "Number" }), el("option", { value: "categoria", text: "Category" }));
      panel.replaceChildren(el("div", { class: "mg-panel" }, el("h5", { text: "Free columns" }),
        rows.length ? el("table", {}, el("thead", {}, el("tr", {}, ...["Name", "Type", "Allowed values", "Required", ""].map(h => el("th", { scope: "col", text: h })))), el("tbody", {}, ...rows)) : el("p", { class: "fl-note", text: "No free column yet." }),
        el("div", { class: "mg-bar" }, nn, nt, F.btn("plus", "Add the column", () => { const nome = nn.value.trim(); if (!nome || doc.colonne.some((/** @type {any} */ c) => c.nome === nome)) { say.textContent = nome ? `A column named ${nome} exists.` : "Type a name."; return; }
          commit({ ...doc, colonne: [...doc.colonne, { nome, tipo: nt.value }] }); fillSelects(); columnsPanel(); }, "Add column"), F.btn("x", "Close the columns panel", () => panel.replaceChildren(), "Close"))));
    };
    const setCol = (/** @type {number} */ k, /** @type {any} */ p) => { commit({ ...doc, colonne: doc.colonne.map((/** @type {any} */ c, /** @type {number} */ q) => (q === k ? { ...c, ...p } : c)) }); columnsPanel(); };
    const rename = (/** @type {number} */ k, /** @type {string} */ to) => {
      const from = doc.colonne[k].nome; if (!to || to === from || doc.colonne.some((/** @type {any} */ c) => c.nome === to)) { columnsPanel(); return; }
      commit({ ...doc, colonne: doc.colonne.map((/** @type {any} */ c, /** @type {number} */ q) => (q === k ? { ...c, nome: to } : c)), campioni: doc.campioni.map((/** @type {any} */ r) => { if (!(from in r.extra)) return r; const { [from]: v, ...extra } = r.extra; return { ...r, extra: { ...extra, [to]: v } }; }) });
      fillSelects(); columnsPanel();
    };

    const sha = el("code", { title: "", text: "not saved yet" });
    const gen = F.btn("import", "Take the roles, times and data of the files open in Data", takeFromData, "Generate metadata");
    if (!F.dataFiles().length) { /** @type {HTMLButtonElement} */ (gen).disabled = true; gen.title = "No file is open in the Data tab"; }
    bar.append(gen, F.btn("file", "Read polarity, ranges and scans from the files open in Data", readFiles, "Read file data"),
      F.btn("plus", "Add an empty row", () => { commit({ ...doc, campioni: [...doc.campioni, blankRow(doc.campioni.length + 1)] }); act = { r: view.length - 1, c: 0 }; anc = { ...act }; move(0, 0, false); }, "Add row"),
      F.btn("minus", "Remove the selected rows", () => { const r0 = Math.min(act.r, anc.r), r1 = Math.max(act.r, anc.r), gone = new Set(view.slice(r0, r1 + 1)); commit({ ...doc, campioni: doc.campioni.filter((/** @type {any} */ _, /** @type {number} */ i) => !gone.has(i)) }); }, "Remove rows"),
      F.btn("search", "Fill the grid from the file names with a pattern", patternPanel, "Fill from name pattern…"), F.btn("flow", "Add, rename, retype or delete free columns", columnsPanel, "Columns…"),
      F.btn("import", "Read a CSV or TSV table", () => fileIn.click(), "Import CSV"), fileIn, sepSel, F.btn("export", "Download the grid as CSV", exportCsv, "Export CSV"),
      F.btn("export", "Download the sample list as JSON", () => { const j = S.samplesJson && S.samplesJson[input.name]; if (j) G.ATTRIBUTIO_CORE.download(input.name, j, "application/json"); }, "Download JSON"), el("span", { class: "fl-note", text: "SHA-256 " }), sha);
    root.append(bar, el("div", { class: "mg-bar" }, groupSel, el("label", { class: "fl-note" }, "Filter: ", fcol), ftext, F.btn("plus", "Add the filter", addFilter, "Filter")), chips, panel, say,
      el("p", { class: "fl-note", text: "Arrows move, Enter or typing edits, Tab moves right while editing, Ctrl+D fills down, Ctrl+V pastes a block from a spreadsheet." }), sc, note);

    // start: the document of the plan, else the files open in Data (the grid opens by itself)
    (async () => {
      try {
        const own = S.samples && S.samples[input.name];
        doc = own ? await toV3(own) : (await fromData()) || emptyDoc();
        if (!own && doc.campioni.length) say.textContent = "Taken from the files open in Data.";
        fillSelects(); chipsDraw(); commit(doc);
      } catch (e) { say.textContent = String((/** @type {any} */ (e)).message || e); }
    })();
    return root;
  };
})();
