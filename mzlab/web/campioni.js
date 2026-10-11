"use strict";
// Sample metadata («Statistica» panel, first section): one row per file with the fields of the mzmine metadata table
//   filename, sample_type (sample | blank | qc | standard | dark), time + time_unit, concentration + concentration_unit, replicate.
// The values are the ones the file name gave (project.py, read at loading) and can be edited here; the table is the only place that
// writes them after loading: f.type / f.time (minutes) / f.conc / f.cunit / f.rep of E.files, the QC and dark roles of QAQC, the
// server copy (/api/meta), the notebook (uiSave) and the .mzflow file. Export and import of the table as TSV (mzmine column names).
// The pure functions at the top (TSV, replicate groups, time scale) are also run by node in tests/test_campioni.py.
const CAMPIONI = (() => {
  const TYPES = ["sample", "blank", "qc", "standard", "dark"];
  const BASE_TYPE = { sample: "sample", blank: "blank", qc: "sample", standard: "standard", dark: "sample" };     // type kept by the rest of the program
  const COLS = ["filename", "sample_type", "time", "time_unit", "concentration", "concentration_unit", "replicate"];
  const T_UNITS = ["min", "h", "d"];
  const C_UNITS = ["mg/L", "µg/L", "ng/L", "g/L"];

  // ---------------------------------------------------------------- pure functions
  // text of the table, tab separated; `rows`: [{filename, type, time (in `tunit`), conc, cunit, rep}]
  function toTsv(rows, tunit = "min") {
    const clean = v => String(v == null ? "" : v).replace(/[\t\r\n]+/g, " ");
    const out = [COLS.join("\t")];
    rows.forEach(r => out.push([r.filename, r.type, r.time ?? "", r.time == null ? "" : tunit, r.conc ?? "", r.conc == null ? "" : (r.cunit || ""), r.rep || ""].map(clean).join("\t")));
    return out.join("\n") + "\n";
  }
  // TSV (or CSV with ; or ,) -> {rows: [{filename, type, time (minutes), conc, cunit, rep}], warnings: [text]}
  function parseTsv(text) {
    const lines = String(text || "").replace(/^﻿/, "").split(/\r?\n/).filter(l => l.trim() !== "");
    const warn = [];
    if (!lines.length) return { rows: [], warnings: ["empty"] };
    const sep = lines[0].includes("\t") ? "\t" : lines[0].includes(";") ? ";" : ",";
    const head = lines[0].split(sep).map(h => h.trim().toLowerCase().replace(/[\s-]+/g, "_"));
    const col = n => head.indexOf(n);
    if (col("filename") < 0) return { rows: [], warnings: ["filename"] };
    const num = s => { const v = parseDecimal(s); return v == null || Number.isNaN(v) ? null : v; };
    const rows = [];
    lines.slice(1).forEach((ln, i) => {
      const c = ln.split(sep).map(s => s.trim()), g = n => (col(n) < 0 ? "" : c[col(n)] || "");
      if (!g("filename")) return;
      const ty = g("sample_type").toLowerCase();
      let type = null;
      if (ty) { if (TYPES.includes(ty)) type = ty; else warn.push(`${g("filename")}: ${ty}`); }
      const tu = g("time_unit").toLowerCase() || "min";
      let time = num(g("time"));
      if (time != null) { if (TIME_UNITS[tu]) time = timeStored(time, tu); else { warn.push(`${g("filename")}: ${tu}`); time = null; } }
      const cu = C_UNITS.find(u => u.toLowerCase() === g("concentration_unit").toLowerCase()) || (g("concentration_unit") ? null : undefined);
      rows.push({ filename: g("filename"), type, time, conc: num(g("concentration")), cunit: cu === undefined ? null : cu, rep: g("replicate") || null, line: i + 2 });
    });
    return { rows, warnings: warn };
  }
  // name of a file without folder and extension, lower case: how the table and the files are matched
  const baseName = n => String(n || "").split(/[\\/]/).pop().replace(/#.*$/, "").replace(/\.(mzml|mzxml|raw|wiff|d|dam)$/i, "").toLowerCase();
  // replicate group of a file: the one typed, else files of the same type, time and concentration (a file with neither is alone)
  function repKey(f, role) {
    if (f.rep) return String(f.rep);
    if (f.time == null && f.conc == null) return "#" + (f.file || f.label || "");
    return `${role}|${f.time ?? ""}|${f.conc ?? ""}${f.conc != null ? "|" + (f.cunit || "") : ""}`;
  }
  // [{key, files: [f...]}] in the order of the first appearance of each group
  function groups(list, roleOf) {
    const m = new Map();
    list.forEach(f => { const k = repKey(f, roleOf(f)); if (!m.has(k)) m.set(k, []); m.get(k).push(f); });
    return [...m].map(([key, files]) => ({ key, files }));
  }
  // position 0..1 of a time in the scale of a list of times (null when there is no scale)
  function timeFrac(t, times) {
    const v = times.filter(x => x != null);
    if (t == null || !v.length) return null;
    const lo = Math.min(...v), hi = Math.max(...v);
    return hi > lo ? (t - lo) / (hi - lo) : 0.5;
  }

  // ---------------------------------------------------------------- the files of the table
  const t = (k, p) => I18N.t(k, p);
  const eh = s => EH(s == null ? "" : s);
  const files = () => (typeof E !== "undefined" && E.files ? E.files : []).filter(f => !f.gone && (f.lv || 1) === 1 && f.kind !== "empty");
  const roleOf = f => (window.QAQC ? QAQC.roleOf(f) : f.type || "sample");
  const roleName = r => t("qaqc.role." + r);
  const repOf = f => repKey(f, roleOf(f));
  const shownRep = f => (f.rep ? f.rep : f.time == null && f.conc == null ? "" : repKey(f, roleOf(f)).replace(/\|/g, " "));

  // apply the edits {file, type?, time?, conc?, cunit?, rep?} to the open files (time in minutes) and tell everyone
  function apply(entries) {
    const srv = [];
    entries.forEach(e => {
      const f = files().find(x => x.file === e.file); if (!f) return;
      if (e.type && TYPES.includes(e.type)) { if (window.QAQC) QAQC.setRole(f.file, e.type); f.type = BASE_TYPE[e.type]; }
      if ("time" in e) f.time = e.time;
      if ("conc" in e) { f.conc = e.conc; if (e.conc == null) f.cunit = null; }
      if ("cunit" in e) f.cunit = e.cunit;
      if ("rep" in e) f.rep = e.rep || undefined;
      srv.push({ file: f.file, type: f.type, time: f.time ?? null, conc: f.conc ?? null, cunit: f.cunit ?? null, rep: f.rep || null });
    });
    if (!srv.length) return 0;
    fetch("api/meta", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ files: srv }) }).catch(() => {});
    if (typeof paintFiles === "function") paintFiles();
    if (typeof renderFileList === "function") renderFileList();
    if (typeof redrawAll === "function") redrawAll();
    if (typeof uiSave === "function") uiSave();
    document.dispatchEvent(new CustomEvent("campioni"));
    return srv.length;
  }
  function rows() {
    const u = tunitGet();
    return files().map(f => ({ filename: f.file, type: roleOf(f), time: timeShown(f.time, u), conc: f.conc ?? null, cunit: f.cunit || (f.conc != null ? "mg/L" : ""), rep: f.rep || "" }));
  }
  // table imported from TSV -> the open files; returns {changed, unknown: [filenames without a file], warnings}
  function importTsv(text) {
    const p = parseTsv(text);
    const byBase = new Map(files().map(f => [baseName(f.file), f]));
    const entries = [], unknown = [];
    p.rows.forEach(r => {
      const f = byBase.get(baseName(r.filename));
      if (!f) { unknown.push(r.filename); return; }
      const e = { file: f.file, time: r.time, conc: r.conc, rep: r.rep };
      if (r.type) e.type = r.type;
      if (r.conc != null) e.cunit = r.cunit || f.cunit || "mg/L";
      entries.push(e);
    });
    return { changed: apply(entries), unknown, warnings: p.warnings };
  }

  // ---------------------------------------------------------------- the legend of the time gradient (also used by the other sections)
  // colour bar from the first to the last time of the files that have one; "" when no file has a time
  function gradientLegend(fs) {
    const tm = (fs || files()).filter(f => f.time != null && f.type === "sample");
    if (tm.length < 2) return "";
    const lo = Math.min(...tm.map(f => f.time)), hi = Math.max(...tm.map(f => f.time));
    if (!(hi > lo)) return "";
    const stops = Array.from({ length: 6 }, (_, i) => `${typeof seqColor === "function" ? seqColor(i / 5) : "#888"} ${i * 20}%`).join(",");
    return `<span class="cmp-grad" role="img" aria-label="${eh(t("meta.grad.aria", { a: fmtTime(lo), b: fmtTime(hi) }))}"><span>${eh(fmtTime(lo))}</span><i style="background:linear-gradient(90deg,${stops})"></i><span>${eh(fmtTime(hi))}</span></span>`;
  }

  // ---------------------------------------------------------------- the section
  function paint(box) {
    const fs = files(), u = tunitGet();
    if (!fs.length) { box.innerHTML = `<p class="sm">${t("meta.none")}</p>`; return; }
    const body = fs.map((f, k) => {
      const r = roleOf(f), id = `data-f="${eh(f.file)}"`, lab = t("meta.of", { f: f.label || f.file });
      return `<tr><td><span class="qq-n">${k + 1}</span> ${eh(f.label || f.file)}</td>
<td><select ${id} data-m="type" aria-label="${eh(lab)} · ${eh(t("qaqc.col.type"))}">${TYPES.map(x => `<option value="${x}"${x === r ? " selected" : ""}>${eh(roleName(x))}</option>`).join("")}</select></td>
<td class="num"><input ${id} data-m="time" inputmode="decimal" size="5" value="${eh(timeShown(f.time, u) ?? "")}" aria-label="${eh(lab)} · ${eh(t("meta.time"))}"></td>
<td class="num"><input ${id} data-m="conc" inputmode="decimal" size="6" value="${eh(f.conc ?? "")}" aria-label="${eh(lab)} · ${eh(t("meta.conc"))}"><select ${id} data-m="cunit" aria-label="${eh(lab)} · ${eh(t("meta.cunit"))}">${C_UNITS.map(x => `<option${x === (f.cunit || "mg/L") ? " selected" : ""}>${x}</option>`).join("")}</select></td>
<td><input ${id} data-m="rep" size="8" value="${eh(f.rep || "")}" placeholder="${eh(shownRep(f))}" aria-label="${eh(lab)} · ${eh(t("meta.rep"))}"></td></tr>`;
    }).join("");
    box.innerHTML = `<p class="sm">${t("meta.intro")}</p>
<div class="qq-scroll"><table class="qq-tab" id="cmp-tab"><thead><tr><th scope="col">${t("qaqc.col.file")}</th><th scope="col">${t("qaqc.col.type")}</th>
<th scope="col">${t("meta.time")} <select id="cmp-tu" aria-label="${eh(t("load.table.tunit.title"))}">${T_UNITS.map(x => `<option value="${x}"${x === u ? " selected" : ""}>${eh(tunitLabel(x))}</option>`).join("")}</select></th>
<th scope="col">${t("meta.conc")}</th><th scope="col">${t("meta.rep")}</th></tr></thead><tbody>${body}</tbody></table></div>
<div class="qq-form"><button type="button" id="cmp-exp">${t("meta.export")}</button><button type="button" id="cmp-copy">${t("meta.copy")}</button><label class="cmp-imp"><span>${t("meta.import")}</span><input type="file" id="cmp-imp" accept=".tsv,.csv,.txt"></label></div>
<div id="cmp-msg" class="sm" role="status"></div>${gradientLegend(fs)}`;
  }
  function msg(box, text, bad) { const m = box.querySelector("#cmp-msg"); if (m) { m.textContent = text; m.classList.toggle("bad", !!bad); } }
  function tsvText() { return toTsv(rows(), tunitGet()); }
  function mount(box) {
    paint(box);
    box.onchange = e => {
      const el = e.target, f = el.dataset && el.dataset.f;
      if (el.id === "cmp-tu") { tunitSet(el.value); paint(box); if (typeof renderFileList === "function") renderFileList(); return; }
      if (el.id === "cmp-imp") {
        const file = el.files && el.files[0]; if (!file) return;
        file.text().then(tx => {
          const r = importTsv(tx);
          paint(box);
          msg(box, r.changed ? t("meta.imported", { n: r.changed }) + (r.unknown.length ? " " + t("meta.unknown", { list: r.unknown.join(", ") }) : "") : t("meta.imported.none"), !r.changed);
        });
        return;
      }
      if (!f) return;
      const m = el.dataset.m, ent = { file: f };
      if (m === "type") ent.type = el.value;
      else if (m === "rep") ent.rep = el.value.trim();
      else if (m === "cunit") ent.cunit = el.value;
      else {
        const v = parseDecimal(el.value);
        if (Number.isNaN(v)) { paint(box); msg(box, t("meta.err.num"), true); return; }
        if (m === "time") ent.time = timeStored(v, tunitGet()); else { ent.conc = v; if (v != null) ent.cunit = (box.querySelector(`select[data-f="${CSS.escape(f)}"][data-m=cunit]`) || {}).value || "mg/L"; }
      }
      apply([ent]); paint(box);
    };
    box.onclick = e => {
      if (e.target.id === "cmp-exp") {
        saveBlob(new Blob([tsvText()], { type: "text/tab-separated-values" }), "metadata.tsv", 2000);
      } else if (e.target.id === "cmp-copy") {
        const tx = tsvText();
        (navigator.clipboard ? navigator.clipboard.writeText(tx) : Promise.reject()).then(() => msg(box, t("meta.copied")), () => msg(box, t("meta.copy.fail"), true));
      }
    };
  }

  // ---------------------------------------------------------------- workflow file
  const exportState = () => files().map(f => ({ nome: f.file, ruolo: roleOf(f), concentrazione: f.conc ?? null, unita_conc: f.cunit ?? null, replica: f.rep || null }));
  function importState(list) {
    if (!Array.isArray(list)) return 0;
    const ent = [];
    list.forEach(x => { if (x && x.nome) { const e = { file: x.nome }; if (TYPES.includes(x.ruolo)) e.type = x.ruolo; if ("concentrazione" in x) { e.conc = x.concentrazione; e.cunit = x.unita_conc || null; } if ("replica" in x) e.rep = x.replica || ""; ent.push(e); } });
    return apply(ent);
  }

  if (typeof STAT !== "undefined") STAT.add({ id: "meta", title: "meta.title", visible: () => true, mount });
  return { TYPES, COLS, toTsv, parseTsv, baseName, repKey, groups, timeFrac, files, roleOf, roleName, repOf, rows, apply, importTsv, gradientLegend, mount, paint, exportState, importState };
})();
if (typeof window !== "undefined") window.CAMPIONI = CAMPIONI;
if (typeof module !== "undefined") module.exports = CAMPIONI;
