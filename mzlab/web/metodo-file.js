// @ts-strict
// The tandem-MS parameters of the open file in the Method window: `file_method/1` (docs/agenti/method.md, `mzlab-cli file method`) from /api/filemethod, one row per segment with the
// precursor, CE, CES, DP, fill time and Q1 resolution as the file holds them. A value the file lacks shows its reason (`unavailable`, `damaged`), a name/precursor warning shows
// its numbers and blocks nothing; `nameWarning` gives that warning alone to the scan list (banco.js). Lazy module (loadScript("m:metodo-file.js")); every text from the file goes through EH().

/** @typedef {{ experiment: number | null, scan: string | null, ms_level: number | null, name: string | null, precursor_mz: number | null, q3_mz: number | null, mz_range: number[] | null, ce_eV: number | null, ce_list_eV: number[], ces_eV: number | null, dp_V: number | null, dwell_ms: number | null, fill_time_ms: number | null, q1_resolution: number | null }} Segment */

/** @param {string} k @param {any} [p] */
const t = (k, p) => I18N.t(k, p);
const n = (/** @type {any} */ v) => (v == null ? "" : String(+(+v).toFixed(4)));
const num = (/** @type {string} */ s) => `<td class="num">${s}</td>`;
const NUMERIC = ["ce_eV", "ces_eV", "dp_V", "fill_time_ms", "q1_resolution"];
// a value with its unit, as one catalogue text each ("{v} eV"): the unit is part of the wording
const WITH_UNIT = /** @type {Record<string, (v: string) => string>} */ ({ ce_eV: v => t("method.file.val.eV", { v }), ces_eV: v => t("method.file.val.eV", { v }),
  dp_V: v => t("method.file.val.V", { v }), fill_time_ms: v => t("method.file.val.ms", { v }), q1_resolution: v => v });

// the reasons of `unavailable` (file_method/1), one literal key each
const REASON = /** @type {Record<string, () => string>} */ ({ not_in_method: () => t("method.file.why.not_in_method"), not_in_file: () => t("method.file.why.not_in_file"),
  unknown: () => t("method.file.why.unknown"), damaged: () => t("method.file.why.damaged") });

/** the cell of a parameter: its value, or why the file has none */
function cell(/** @type {Segment} */ s, /** @type {string} */ key, /** @type {Record<string, string>} */ why) {
  const v = /** @type {any} */ (s)[key];
  if (v != null) return num(EH(WITH_UNIT[key](n(v))));
  if (key === "ce_eV" && s.ce_list_eV.length > 1) return num(EH(t("method.file.val.eV", { v: s.ce_list_eV.map(n).join(t("method.file.listSep")) })));
  const r = why[key] ? REASON[why[key]]() : t("method.file.unset");
  return `<td class="muted sm">${EH(r)}</td>`;
}

function segments(/** @type {any} */ d) {
  const segs = /** @type {Segment[]} */ (d.segments || []);
  if (!segs.length) return "";
  const why = /** @type {Record<string, string>} */ (d.unavailable || {});
  const heads = ["method.file.exp", "method.file.scan", "method.file.precursor", "method.file.ce", "method.file.ces", "method.file.dp", "method.file.fill", "method.file.q1res"];
  const th = heads.map((h, i) => `<th scope="col"${i > 1 ? ' class="num"' : ""}>${EH(t(h))}</th>`).join("");
  const rows = segs.map(s => {
    const prec = s.precursor_mz != null ? (s.q3_mz != null ? t("method.file.product", { q1: n(s.precursor_mz), q3: n(s.q3_mz) }) : n(s.precursor_mz)) : s.mz_range ? t("method.file.range", { lo: n(s.mz_range[0]), hi: n(s.mz_range[1]) }) : "";
    const scan = [s.scan, s.ms_level != null ? t("method.file.msLevel", { n: s.ms_level }) : "", s.name].filter(x => x).join(t("method.file.sep"));
    return `<tr><td>${EH(String(s.experiment ?? ""))}</td><td>${EH(scan)}</td>${num(EH(prec))}${NUMERIC.map(k => cell(s, k, why)).join("")}</tr>`;
  }).join("");
  return `<table data-mfile="segments"><caption class="sm muted" style="text-align:left">${EH(t("method.file.segments"))}</caption><tr>${th}</tr>${rows}</table>`;
}

function acquired(/** @type {any} */ d) {
  const a = /** @type {any[]} */ (d.acquired || []);
  if (!a.length) return "";
  const rows = a.map(r => `<tr>${num(EH(n(r.precursor_mz)))}${num(EH(r.ce_eV != null ? t("method.file.val.eV", { v: n(r.ce_eV) }) : ""))}${num(EH(n(r.scans)))}</tr>`).join("");
  return `<table data-mfile="acquired"><caption class="sm muted" style="text-align:left">${EH(t("method.file.acquired"))}</caption><tr><th scope="col" class="num">${EH(t("method.file.precursor"))}</th><th scope="col" class="num">${EH(t("method.file.ce"))}</th><th scope="col" class="num">${EH(t("method.file.scans"))}</th></tr>${rows}</table>`;
}

/** the texts of the warnings of `file_method/1`: the name/precursor warning with its numbers (`warn.file.nameMz`), any other by its generic text */
const warnTexts = (/** @type {any} */ d) => /** @type {any[]} */ (d.warnings || []).map(w => w.key === "warn.file.nameMz"
  ? t(w.key, { name: w.name, name_mz: n(w.nearest_name_mz), precursor: n(w.nearest_precursor), delta: n(w.delta_mz) }) : t("method.file.warnOther"));

function notes(/** @type {any} */ d) {
  const bad = /** @type {any[]} */ (d.damaged || []).map(x => (x.part === "mass_range" ? t("method.file.damaged.mass_range", { experiment: x.experiment }) : t("method.file.damaged.experiment_header", { experiment: x.experiment })));
  const warn = warnTexts(d);
  const all = bad.concat(warn);
  return all.length ? `<ul class="sm" role="note" data-mfile="notes" style="margin:4px 0 8px">${all.map(x => `<li>${EH(x)}</li>`).join("")}</ul>` : "";
}

/** the section for the open item `f` (an empty string if the file gives nothing to show) */
async function html(/** @type {{ k: number }} */ f) {
  let d;
  try { d = await J("api/filemethod?k=" + f.k); } catch (_) { return `<div class="muted sm" data-mfile="none">${EH(t("method.file.fail"))}</div>`; }
  if (!d || d.schema !== "file_method/1") return "";
  const body = notes(d) + segments(d) + acquired(d);
  return body ? `<h5 style="margin:8px 0 2px">${EH(t("method.file.title"))}</h5>${body}` : "";
}

/** the name/precursor warning of the open item `f` for the scan list (an empty string when the name is fine or the document is not served; never blocks, never throws) */
async function nameWarning(/** @type {{ k: number }} */ f) {
  let d;
  try { d = await J("api/filemethod?k=" + f.k); } catch (_) { return ""; }
  if (!d || d.schema !== "file_method/1") return "";
  const w = warnTexts({ warnings: (d.warnings || []).filter((/** @type {any} */ x) => x.key === "warn.file.nameMz") });
  return w.length ? `<ul class="sm" role="note" tabindex="0" data-mfile="namewarn" style="margin:0 0 6px;padding:0 0 0 16px;max-height:12em;overflow:auto">${w.map(x => `<li>${EH(x)}</li>`).join("")}</ul>` : "";
}

/** @type {any} */ (window).METODO_FILE = { html, nameWarning };
