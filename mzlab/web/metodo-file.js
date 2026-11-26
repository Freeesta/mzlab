// @ts-strict
// The tandem-MS parameters of the open file in the Method window: `file_method/1` (docs/agenti/method.md, `mzlab-cli file method`) from /api/filemethod, one row per segment with the
// precursor, CE, CES, DP, fill time and Q1 resolution as the file holds them. A value the file lacks shows its reason (`unavailable`, `damaged`), a name/precursor warning shows
// its numbers and blocks nothing. Lazy module (loadScript("m:metodo-file.js")); every text from the file goes through EH().

/** @typedef {{ experiment: number | null, scan: string | null, ms_level: number | null, name: string | null, precursor_mz: number | null, q3_mz: number | null, mz_range: number[] | null, ce_eV: number | null, ce_list_eV: number[], ces_eV: number | null, dp_V: number | null, dwell_ms: number | null, fill_time_ms: number | null, q1_resolution: number | null }} Segment */

/** @param {string} k @param {any} [p] */
const t = (k, p) => I18N.t(k, p);
const n = (/** @type {any} */ v) => (v == null ? "" : String(+(+v).toFixed(4)));
const num = (/** @type {string} */ s) => `<td class="num">${s}</td>`;
const NUMERIC = ["ce_eV", "ces_eV", "dp_V", "fill_time_ms", "q1_resolution"];
const UNIT = /** @type {Record<string, string>} */ ({ ce_eV: " eV", ces_eV: " eV", dp_V: " V", fill_time_ms: " ms", q1_resolution: "" });

// the reasons of `unavailable` (file_method/1), one literal key each
const REASON = /** @type {Record<string, () => string>} */ ({ not_in_method: () => t("method.file.why.not_in_method"), not_in_file: () => t("method.file.why.not_in_file"),
  unknown: () => t("method.file.why.unknown"), damaged: () => t("method.file.why.damaged") });

/** the cell of a parameter: its value, or why the file has none */
function cell(/** @type {Segment} */ s, /** @type {string} */ key, /** @type {Record<string, string>} */ why) {
  const v = /** @type {any} */ (s)[key];
  if (v != null) return num(EH(n(v) + UNIT[key]));
  if (key === "ce_eV" && s.ce_list_eV.length > 1) return num(EH(s.ce_list_eV.map(n).join(", ") + " eV"));
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
    const prec = s.precursor_mz != null ? n(s.precursor_mz) + (s.q3_mz != null ? ` → ${n(s.q3_mz)}` : "") : s.mz_range ? `${n(s.mz_range[0])}–${n(s.mz_range[1])}` : "";
    const scan = [s.scan, s.ms_level != null ? "MS" + s.ms_level : "", s.name].filter(x => x).join(" · ");
    return `<tr><td>${EH(String(s.experiment ?? ""))}</td><td>${EH(scan)}</td>${num(EH(prec))}${NUMERIC.map(k => cell(s, k, why)).join("")}</tr>`;
  }).join("");
  return `<table data-mfile="segments"><caption class="sm muted" style="text-align:left">${EH(t("method.file.segments"))}</caption><tr>${th}</tr>${rows}</table>`;
}

function acquired(/** @type {any} */ d) {
  const a = /** @type {any[]} */ (d.acquired || []);
  if (!a.length) return "";
  const rows = a.map(r => `<tr>${num(EH(n(r.precursor_mz)))}${num(EH(r.ce_eV != null ? n(r.ce_eV) + " eV" : ""))}${num(EH(n(r.scans)))}</tr>`).join("");
  return `<table data-mfile="acquired"><caption class="sm muted" style="text-align:left">${EH(t("method.file.acquired"))}</caption><tr><th scope="col" class="num">${EH(t("method.file.precursor"))}</th><th scope="col" class="num">${EH(t("method.file.ce"))}</th><th scope="col" class="num">${EH(t("method.file.scans"))}</th></tr>${rows}</table>`;
}

function notes(/** @type {any} */ d) {
  const bad = /** @type {any[]} */ (d.damaged || []).map(x => (x.part === "mass_range" ? t("method.file.damaged.mass_range", { experiment: x.experiment }) : t("method.file.damaged.experiment_header", { experiment: x.experiment })));
  const warn = /** @type {any[]} */ (d.warnings || []).map(w => w.key === "warn.file.nameMz"
    ? t(w.key, { name: w.name, name_mz: n(w.nearest_name_mz), precursor: n(w.nearest_precursor), delta: n(w.delta_mz) }) : String(w.key));
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

/** @type {any} */ (window).METODO_FILE = { html };
