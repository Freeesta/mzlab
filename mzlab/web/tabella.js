"use strict";
// Feature table of the series (high resolution only; «Statistica» panel): the matrix features x files that the blank subtraction and the PCA
// work on. The program does not detect features: the table comes from the user, in one of three ways
//   - an mzmine feature table (CSV / TSV export: columns row ID, row m/z, row retention time and one area column per file),
//   - a list of «m/z; RT» lines typed or pasted: each ion is measured in every open file (the integration of the XIC panel, as the time profile),
//   - the table loaded in mzFinder, when there is one.
// The table lives in memory (it is rebuilt from the same list or file in a second); the pure parser is also run by node (tests/test_tabella.py).
const TABELLA = (() => {
  const MAX_LIST = 300;                                        // ions of a typed list (each costs one XIC per file)
  const t = (k, p) => I18N.t(k, p);
  const eh = s => String(s == null ? "" : s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  const baseName = n => String(n || "").split(/[\\/]/).pop().replace(/#.*$/, "").replace(/\.(mzml|mzxml|raw|wiff|d|dam)\b/i, "").toLowerCase().trim();

  // ---------------------------------------------------------------- pure functions
  // «m/z; RT» lines (separator: ; , tab or space; decimal comma accepted when the separator is not a comma) -> {rows: [{mz, rt}], bad: [line numbers]}
  function parseList(text) {
    const rows = [], bad = [];
    String(text || "").split(/\r?\n/).forEach((ln, i) => {
      const s = ln.trim();
      if (!s || s.startsWith("#")) return;
      const parts = s.includes(";") || s.includes("\t") ? s.split(/[;\t]/) : s.includes(",") && /,\s/.test(s) ? s.split(/,\s*/) : s.split(/\s+/);
      const mz = parseDecimal(parts[0]), rt = parseDecimal(parts[1]);
      if (mz == null || Number.isNaN(mz) || mz <= 0 || rt == null || Number.isNaN(rt)) bad.push(i + 1); else rows.push({ mz, rt });
    });
    return { rows, bad };
  }
  // CSV / TSV of an mzmine feature table -> {features: [{key, id, mz, rt, areas: {name: area}}], columns: [sample names found]}; `names` = the names of the open
  // files: a column is a sample when its header (without «datafile:», «:area», « Peak area», «.mzML») matches one of them
  function parseTable(text, names) {
    const lines = String(text || "").replace(/^﻿/, "").split(/\r?\n/).filter(l => l.trim() !== "");
    if (lines.length < 2) return { features: [], columns: [], error: "empty" };
    const sep = lines[0].includes("\t") ? "\t" : lines[0].includes(";") ? ";" : ",";
    const split = l => { const o = []; let cur = "", q = false; for (const ch of l) { if (ch === '"') q = !q; else if (ch === sep && !q) { o.push(cur); cur = ""; } else cur += ch; } o.push(cur); return o.map(x => x.trim()); };
    const head = split(lines[0]), low = head.map(h => h.toLowerCase());
    const find = re => low.findIndex(h => re.test(h));
    const cMz = find(/^(row )?(m\/z|mz|average m\/z|row m\/z)$/), cRt = find(/^(row )?(retention time|rt|average rt|row retention time|average retention time)$/), cId = find(/^(row )?(id|row id|feature id)$/);
    if (cMz < 0 || cRt < 0) return { features: [], columns: [], error: "columns" };
    const want = new Map(names.map(n => [baseName(n), n]));
    const cols = [];
    head.forEach((h, i) => {
      const k = baseName(h.replace(/^datafile:/i, "").replace(/:(area|height)$/i, "").replace(/\s+(peak )?(area|height)$/i, ""));
      if (i !== cMz && i !== cRt && i !== cId && want.has(k) && !/height/i.test(h)) cols.push([i, want.get(k)]);
    });
    const features = [];
    lines.slice(1).forEach((ln, r) => {
      const c = split(ln), mz = parseDecimal(c[cMz]), rt = parseDecimal(c[cRt]);
      if (mz == null || Number.isNaN(mz) || rt == null || Number.isNaN(rt)) return;
      const areas = {};
      cols.forEach(([i, n]) => { const v = parseDecimal(c[i]); areas[n] = v == null || Number.isNaN(v) ? 0 : v; });
      const id = cId >= 0 ? c[cId] : String(r + 1);
      features.push({ key: id, id, mz, rt, areas });
    });
    return { features, columns: cols.map(x => x[1]) };
  }
  // the matrix features x files (areas, 0 where missing) in the order of `names`
  const toMatrix = (features, names) => features.map(f => names.map(n => (f.areas && f.areas[n]) || 0));

  // ---------------------------------------------------------------- the table of the page
  let tb = null;                                               // {source, features, files}
  const files = () => (window.CAMPIONI ? CAMPIONI.files() : []).filter(f => f.kind !== "mrm");
  const roles = () => files().map(f => CAMPIONI.roleOf(f));
  function setTable(source, features) {
    tb = { source, features, files: files().map(f => f.file) };
    document.dispatchEvent(new CustomEvent("tabella"));
    return tb;
  }
  async function fromList(text, progress) {
    const p = parseList(text);
    if (p.bad.length) throw new Error(t("feat.err.lines", { n: p.bad.length, list: p.bad.slice(0, 6).join(", ") }));
    if (!p.rows.length) throw new Error(t("feat.err.empty"));
    if (p.rows.length > MAX_LIST) throw new Error(t("feat.err.many", { max: MAX_LIST }));
    const fs = files(), out = [];
    for (let i = 0; i < p.rows.length; i++) {
      const r = p.rows[i];
      const ms = await Promise.all(fs.map(f => PROFILO.measure(f, r.mz, r.rt)));
      const areas = {}; fs.forEach((f, k) => { areas[f.file] = ms[k] ? ms[k].area : 0; });
      out.push({ key: String(i + 1), id: String(i + 1), mz: r.mz, rt: r.rt, areas });
      if (progress) progress(i + 1, p.rows.length);
    }
    return setTable("list", out);
  }
  function fromText(text) {
    const r = parseTable(text, files().map(f => f.file));
    if (r.error === "columns") throw new Error(t("feat.err.columns"));
    if (!r.features.length) throw new Error(t("feat.err.empty"));
    if (!r.columns.length) throw new Error(t("feat.err.nofiles"));
    const tbl = setTable("file", r.features);
    tbl.unmatched = files().filter(f => !r.columns.includes(f.file)).map(f => f.file);
    return tbl;
  }
  function fromMzFinder() {
    const m = window.UNTUI && UNTUI.state && UNTUI.state.table;
    if (!m) throw new Error(t("feat.err.nomzf"));
    const names = (m.samples || []).map(s => (typeof s === "string" ? s : s.name)), map = new Map(files().map(f => [baseName(f.file), f.file]));
    const feats = m.features.map(f => { const areas = {}; names.forEach(n => { const k = map.get(baseName(n)); if (k) areas[k] = (f.areas && f.areas[n]) || 0; }); return { key: String(f.key), id: String(f.id ?? f.key), mz: f.mz, rt: f.rt, areas }; });
    return setTable("mzfinder", feats);
  }
  const matrix = () => (tb ? toMatrix(tb.features, tb.files) : []);

  // ---------------------------------------------------------------- the section
  const fmt = (v, d = 4) => (v == null || !isFinite(v) ? "–" : v.toFixed(d));
  function summary() {
    if (!tb) return `<p class="sm">${t("feat.none")}</p>`;
    const det = tb.features.map(f => tb.files.filter(n => f.areas[n] > 0).length);
    const rows = tb.features.slice(0, 15).map((f, i) => `<tr data-mz="${f.mz}" data-rt="${f.rt}" tabindex="0" role="button" aria-label="${eh(t("feat.row.aria", { mz: fmt(f.mz), rt: fmt(f.rt, 2) }))}"><td class="num">${eh(f.id)}</td><td class="num">${fmt(f.mz)}</td><td class="num">${fmt(f.rt, 2)}</td><td class="num">${det[i]}/${tb.files.length}</td></tr>`).join("");
    return `<div class="sm" role="status">${t("feat.sum", { n: tb.features.length, m: tb.files.length, src: tb.source === "list" ? t("feat.src.list") : tb.source === "mzfinder" ? t("feat.src.mzfinder") : t("feat.src.file") })}${tb.unmatched && tb.unmatched.length ? " " + t("feat.unmatched", { list: tb.unmatched.join(", ") }) : ""}</div>
<div class="qq-scroll"><table class="qq-tab" id="feat-tab"><thead><tr><th scope="col">ID</th><th scope="col">m/z</th><th scope="col">RT</th><th scope="col">${t("feat.col.det")}</th></tr></thead><tbody>${rows}</tbody></table></div>${tb.features.length > 15 ? `<p class="sm">${t("feat.more", { n: tb.features.length - 15 })}</p>` : ""}`;
  }
  function paint(box) {
    const mzf = !!(window.UNTUI && UNTUI.state && UNTUI.state.table);
    box.innerHTML = `<p class="sm">${t("feat.intro")}</p>
<div class="qq-form"><label class="cmp-imp"><span>${t("feat.import")}</span><input type="file" id="feat-file" accept=".csv,.tsv,.txt"></label>${mzf ? `<button type="button" id="feat-mzf">${t("feat.mzf")}</button>` : ""}${tb ? `<button type="button" id="feat-clear">${t("feat.clear")}</button>` : ""}</div>
<label class="sm" for="feat-list">${t("feat.list")}</label><textarea id="feat-list" rows="4" spellcheck="false" placeholder="305.1234; 6.02&#10;250.0000; 4.01" style="width:100%;box-sizing:border-box"></textarea>
<div class="qq-form"><button type="button" id="feat-go">${t("feat.go")}</button></div>
<div id="feat-msg" class="sm bad" role="alert"></div><div id="feat-sum">${summary()}</div>`;
  }
  function mount(box) {
    paint(box);
    const msg = s => { const m = box.querySelector("#feat-msg"); if (m) m.textContent = s || ""; };
    const done = () => { box.querySelector("#feat-sum").innerHTML = summary(); msg(""); };
    box.onchange = e => {
      if (e.target.id === "feat-file" && e.target.files[0]) e.target.files[0].text().then(tx => { try { fromText(tx); paint(box); } catch (err) { msg(String(err.message || err)); } });
    };
    box.onclick = async e => {
      if (e.target.id === "feat-go") {
        const b = e.target; b.disabled = true; msg("");
        try { await fromList(box.querySelector("#feat-list").value, (i, n) => { msg(t("feat.progress", { i, n })); box.querySelector("#feat-msg").classList.remove("bad"); }); paint(box); }
        catch (err) { msg(String(err.message || err)); box.querySelector("#feat-msg").classList.add("bad"); }
        b.disabled = false;
      } else if (e.target.id === "feat-mzf") { try { fromMzFinder(); paint(box); } catch (err) { msg(String(err.message || err)); } }
      else if (e.target.id === "feat-clear") { tb = null; document.dispatchEvent(new CustomEvent("tabella")); paint(box); }
      const tr = e.target.closest("#feat-tab tbody tr");
      if (tr && window.PROFILO) PROFILO.show(tr.dataset.mz, tr.dataset.rt);
    };
    box.onkeydown = e => { const tr = e.target.closest && e.target.closest("#feat-tab tbody tr"); if (tr && (e.key === "Enter" || e.key === " ")) { e.preventDefault(); tr.click(); } };
  }
  if (typeof STAT !== "undefined") STAT.add({ id: "features", title: "feat.title", hr: true, visible: () => true, mount });
  return { parseList, parseTable, toMatrix, baseName, fromList, fromText, fromMzFinder, mount, matrix, roles, files, get table() { return tb; } };
})();
if (typeof window !== "undefined") window.TABELLA = TABELLA;
if (typeof module !== "undefined") module.exports = TABELLA;
