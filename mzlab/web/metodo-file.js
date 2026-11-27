// @ts-strict
// What the Method window draws from `file_method/1` (docs/agenti/method.md, `mzlab-cli file method`, /api/filemethod): the tandem-MS parameters per segment, the LC program
// (points, the last %B held to the end of the run, run time) and the diode array. Every number is a display string made in Rust (`shown`, two decimals), so site, app and CLI agree;
// every value says where it comes from (`sources`: read from a stream and field of the file, or inferred) on hover and in a collapsed list. Nothing is filled in: an eluent
// composition appears only when the file names it. Lazy module (loadScript("m:metodo-file.js")); every text from a file goes through EH().

/** @typedef {{ experiment: number | null, scan: string | null, ms_level: number | null, name: string | null, precursor_mz: number | null, q3_mz: number | null, mz_range: number[] | null, ce_eV: number | null, ce_list_eV: number[], ces_eV: number | null, dp_V: number | null, dwell_ms: number | null, fill_time_ms: number | null, q1_resolution: number | null, shown?: Record<string, string> }} Segment */
/** @typedef {{ kind: string, at: string } | null | undefined} Src */

/** @param {string} k @param {any} [p] */
const t = (k, p) => I18N.t(k, p);
const num = (/** @type {string} */ s) => `<td class="num">${s}</td>`;
const NUMERIC = ["ce_eV", "ces_eV", "dp_V", "fill_time_ms", "q1_resolution"];
// a value with its unit, as one catalogue text each ("{v} eV"): the unit is part of the wording
const WITH_UNIT = /** @type {Record<string, (v: string) => string>} */ ({ ce_eV: v => t("method.file.val.eV", { v }), ces_eV: v => t("method.file.val.eV", { v }),
  dp_V: v => t("method.file.val.V", { v }), fill_time_ms: v => t("method.file.val.ms", { v }), q1_resolution: v => v });

// the reasons of `unavailable` (file_method/1): a short code in the cell, the whole sentence as its tooltip and aria-label, one literal key each
const NA = { code: () => t("method.file.code.na"), legend: () => t("method.file.code.na.legend") }, NR = { code: () => t("method.file.code.nr"), legend: () => t("method.file.code.nr.legend") };
const REASON = /** @type {Record<string, { c: typeof NA, long: () => string }>} */ ({
  not_in_method: { c: NA, long: () => t("method.file.why.not_in_method") }, not_in_file: { c: NA, long: () => t("method.file.why.not_in_file") },
  unknown: { c: NR, long: () => t("method.file.why.unknown") }, damaged: { c: NR, long: () => t("method.file.why.damaged") } });

/** the cell of a parameter: its value, or a short code with the reason (`used` collects the codes shown, for the legend) */
function cell(/** @type {Segment} */ s, /** @type {string} */ key, /** @type {Record<string, string>} */ why, /** @type {Set<typeof NA>} */ used) {
  const v = /** @type {any} */ (s)[key], sh = s.shown || {};
  if (v != null && sh[key] != null) return num(EH(WITH_UNIT[key](sh[key])));
  if (key === "ce_eV" && s.ce_list_eV.length > 1 && sh.ce_eV != null) return num(EH(t("method.file.val.eV", { v: sh.ce_eV })));
  const r = REASON[why[key]] || { c: NA, long: () => t("method.file.unset") };
  used.add(r.c);
  const tip = EH(r.long());
  return `<td class="muted sm" title="${tip}" aria-label="${tip}">${EH(r.c.code())}</td>`;
}

function segments(/** @type {any} */ d) {
  const segs = /** @type {Segment[]} */ (d.segments || []);
  if (!segs.length) return "";
  const why = /** @type {Record<string, string>} */ (d.unavailable || {}), used = /** @type {Set<typeof NA>} */ (new Set());
  const heads = ["method.file.exp", "method.file.scan", "method.file.precursor", "method.file.ce", "method.file.ces", "method.file.dp", "method.file.fill", "method.file.q1res"];
  const th = heads.map((h, i) => `<th scope="col"${i > 1 ? ' class="num"' : ""}>${EH(t(h))}</th>`).join("");
  const rows = segs.map(s => {
    const sh = s.shown || {};
    const prec = s.precursor_mz != null ? (s.q3_mz != null ? t("method.file.product", { q1: sh.precursor_mz, q3: sh.q3_mz }) : sh.precursor_mz) : s.mz_range ? t("method.file.range", { lo: sh.mz_lo, hi: sh.mz_hi }) : "";
    const scan = [s.scan, s.ms_level != null ? t("method.file.msLevel", { n: s.ms_level }) : "", s.name].filter(x => x).join(t("method.file.sep"));
    return `<tr><td>${EH(String(s.experiment ?? ""))}</td><td>${EH(scan)}</td>${num(EH(prec))}${NUMERIC.map(k => cell(s, k, why, used)).join("")}</tr>`;
  }).join("");
  const legend = used.size ? `<div class="sm muted" data-mfile="legend">${[NA, NR].filter(c => used.has(c)).map(c => EH(c.legend())).join(EH(t("method.file.sep")))}</div>` : "";
  return `<table data-mfile="segments"><caption class="sm muted" style="text-align:left">${EH(t("method.file.segments"))}</caption><tr>${th}</tr>${rows}</table>${legend}`;
}

function acquired(/** @type {any} */ d) {
  const a = /** @type {any[]} */ (d.acquired || []);
  if (!a.length) return "";
  const rows = a.map(r => { const sh = r.shown || {};
    return `<tr>${num(EH(sh.precursor_mz ?? ""))}${num(EH(sh.ce_eV != null ? t("method.file.val.eV", { v: sh.ce_eV }) : ""))}${num(EH(String(r.scans ?? "")))}</tr>`; }).join("");
  return `<table data-mfile="acquired"><caption class="sm muted" style="text-align:left">${EH(t("method.file.acquired"))}</caption><tr><th scope="col" class="num">${EH(t("method.file.precursor"))}</th><th scope="col" class="num">${EH(t("method.file.ce"))}</th><th scope="col" class="num">${EH(t("method.file.scans"))}</th></tr>${rows}</table>`;
}

/** the texts of the warnings of `file_method/1`: the name/precursor warning with its numbers (`warn.file.nameMz`), any other by its generic text */
const warnTexts = (/** @type {any} */ d) => /** @type {any[]} */ (d.warnings || []).map(w => w.key === "warn.file.nameMz"
  ? t(w.key, { name: w.name, name_mz: (w.shown || {}).nearest_name_mz, precursor: (w.shown || {}).nearest_precursor, delta: (w.shown || {}).delta_mz }) : t("method.file.warnOther"));

function notes(/** @type {any} */ d) {
  const bad = /** @type {any[]} */ (d.damaged || []).map(x => (x.part === "mass_range" ? t("method.file.damaged.mass_range", { experiment: x.experiment }) : t("method.file.damaged.experiment_header", { experiment: x.experiment })));
  const warn = warnTexts(d);
  const all = bad.concat(warn);
  return all.length ? `<ul class="sm" role="note" data-mfile="notes" style="margin:4px 0 8px">${all.map(x => `<li>${EH(x)}</li>`).join("")}</ul>` : "";
}

// ---- where a value comes from: its tooltip and the collapsed list
/** @param {Src} s */
const srcText = s => (s ? t(s.kind === "read" ? "method.src.read" : "method.src.inferred", { at: s.at }) : t("method.src.none"));
/** @param {Src} s */
const tipOf = s => ` title="${EH(srcText(s))}"`;
/** the collapsed list of sources: one line per part, so a phone or a printed report has them too */
function sourceList(/** @type {[string, Src][]} */ items) {
  const li = items.filter(([, s]) => s !== undefined).map(([name, s]) => `<li><b>${EH(name)}</b>: ${EH(srcText(s))}</li>`).join("");
  return li ? `<details data-mfile="sources" style="margin-top:6px"><summary class="sm">${EH(t("method.src.title"))}</summary><ul class="sm" style="margin:4px 0 0;padding-left:18px">${li}</ul></details>` : "";
}

/** the nice tick step for an axis of `max` minutes: at most about eight labels */
const stepOf = (/** @type {number} */ max) => [1, 2, 5, 10, 20, 50].find(s => max / s <= 8) || 100;

/** the plot of %B against time: the points of the file as a solid line, the last %B held to the end of the run as a dashed one */
function plot(/** @type {any} */ lc) {
  const g = /** @type {any[]} */ (lc.gradient || []), hold = lc.hold;
  const last = g.length ? g[g.length - 1].t_min : 0, max = Math.max(lc.run_time_min || 0, last) || 1;
  const w = 440, hh = Math.max(160, (g.length + 2) * 27), l = 40, r = 10, top = 10, b = 34;
  const X = (/** @type {number} */ v) => l + v / max * (w - l - r), Y = (/** @type {number} */ v) => top + (100 - v) / 100 * (hh - top - b);
  const ink = css("--muted"), acc = css("--accent"), line = css("--line");
  const ttl = hold ? t("method.lc.holdTip", { b: hold.shown.b, from: hold.shown.from, to: hold.shown.to }) : "";
  let svg = `<svg role="img" aria-label="${EH(t("method.lc.plotAria"))}" viewBox="0 0 ${w} ${hh}" width="${w}" height="${hh}" style="background:#fff;border:1px solid ${line};border-radius:6px;flex:none;max-width:100%" font-family="system-ui" font-size="11"><path d="M${l} ${top}V${hh - b}H${w - r}" fill="none" stroke="${ink}"/>`;
  for (const v of [0, 25, 50, 75, 100]) svg += `<path d="M${l - 3} ${Y(v)}H${l}" stroke="${ink}"/><text x="${l - 6}" y="${Y(v) + 4}" text-anchor="end" fill="${ink}">${v}</text>`;
  for (let v = 0; v <= max; v += stepOf(max)) svg += `<path d="M${X(v)} ${hh - b}v3" stroke="${ink}"/><text x="${X(v)}" y="${hh - b + 15}" text-anchor="middle" fill="${ink}">${v}</text>`;
  const pts = g.filter(q => q.B_pct != null);
  svg += `<polyline fill="none" stroke="${acc}" stroke-width="2" points="${pts.map(q => X(q.t_min) + "," + Y(q.B_pct)).join(" ")}"><title>${EH(srcText(lc.sources && lc.sources.gradient))}</title></polyline>`;
  svg += pts.map(q => `<circle cx="${X(q.t_min)}" cy="${Y(q.B_pct)}" r="2.6" fill="${acc}"/>`).join("");
  if (hold) svg += `<line data-mlc="hold" x1="${X(hold.from_min)}" y1="${Y(hold.B_pct)}" x2="${X(hold.to_min)}" y2="${Y(hold.B_pct)}" stroke="${acc}" stroke-width="2" stroke-dasharray="5 4" opacity=".7"><title>${EH(ttl)}</title></line>`;
  svg += `<text x="${(l + w - r) / 2}" y="${hh - 4}" text-anchor="middle" fill="${css("--ink")}">${EH(t("method.time"))}</text><text transform="translate(11 ${(top + hh - b) / 2}) rotate(-90)" text-anchor="middle" fill="${css("--ink")}">% B</text></svg>`;
  return svg;
}

/** the LC section: plot and table side by side, the run time under the table it belongs to */
function lcSection(/** @type {any} */ lc, /** @type {string} */ id = "") {
  if (!lc) return "";
  const g = /** @type {any[]} */ (lc.gradient || []), hold = lc.hold, s = lc.sources || {};
  const th = (/** @type {string} */ k, /** @type {Src} */ src) => `<th scope="col" class="num"${tipOf(src)}>${EH(t(k))}</th>`;
  const cellOf = (/** @type {any} */ v, /** @type {Src} */ src) => `<td class="num"${tipOf(src)}>${EH(v ?? "")}</td>`;
  const body = g.map(q => `<tr>${cellOf(q.shown.t, s.gradient)}${cellOf(q.shown.a, s.a_pct)}${cellOf(q.shown.b, s.gradient)}${cellOf(q.shown.flow, s.gradient)}</tr>`).join("");
  const heldRow = hold ? `<tr data-mlc="holdrow" class="muted" style="font-style:italic" title="${EH(t("method.lc.holdTip", { b: hold.shown.b, from: hold.shown.from, to: hold.shown.to }))}">${cellOf(hold.shown.to, s.hold)}${cellOf(hold.shown.a, s.hold)}${cellOf(hold.shown.b, s.hold)}${cellOf(hold.shown.flow, s.hold)}</tr>` : "";
  const tbl = g.length ? `<table class="lct" data-mlc="table"><tr>${th("method.time", s.gradient)}${th("method.lc.colA", s.a_pct)}${th("method.lc.colB", s.gradient)}${th("method.flow", s.gradient)}</tr>${body}${heldRow}</table>` : "";
  const chip = (/** @type {string} */ k, /** @type {string} */ v, /** @type {Src} */ src, /** @type {string} */ mark) => `<span${tipOf(src)} data-mlc="${mark}"><span class="muted">${EH(t(k))}</span> <b>${EH(v)}</b></span>`;
  const chips = [lc.run_time_shown != null ? chip("method.runTime", t("method.val.min", { v: lc.run_time_shown }), s.run_time, "run") : "", lc.flow_shown != null ? chip("method.flow", lc.flow_shown, s.flow, "flow") : "",
    lc.oven_shown != null ? chip("method.oven", t("method.val.C", { v: lc.oven_shown }), s.oven, "oven") : ""].filter(Boolean).join("");
  const eluents = /** @type {any[]} */ (lc.eluents || []).map(e => `<span${tipOf(s.eluents)}>${EH(e.channel)} = ${EH(e.name)}</span>`).join(EH(t("method.file.sep")));
  const list = sourceList([[t("method.src.gradient"), g.length ? s.gradient || null : undefined], [t("method.src.a"), g.length ? s.a_pct : undefined], [t("method.runTime"), lc.run_time_shown != null ? s.run_time || null : undefined],
    [t("method.flow"), lc.flow_shown != null ? s.flow || null : undefined], [t("method.oven"), lc.oven_shown != null ? s.oven || null : undefined], [t("method.src.hold"), hold ? s.hold : undefined], [t("method.src.eluents"), eluents ? s.eluents || null : undefined]]);
  return `<section class="msec"${id ? ` id="${id}"` : ""} data-mlc="lc"><h4>${EH(t("method.lc.title"))}</h4>${eluents ? `<div class="sm" style="margin:0 0 8px" data-mlc="eluents">${eluents}</div>` : ""}
   <div style="display:flex;gap:22px;align-items:flex-start;flex-wrap:wrap">${g.length ? plot(lc) : ""}<div style="flex:1 1 300px;max-width:520px">${tbl}${chips ? `<div class="mchips" data-mlc="foot" style="justify-content:flex-end;margin-top:6px">${chips}</div>` : ""}</div></div>${list}</section>`;
}

/** the diode-array section: labelled rows, the raw stream and field only as a tooltip and in the collapsed list */
function dadSection(/** @type {any} */ dad) {
  if (!dad || !(dad.rows || []).length) return "";
  const rows = /** @type {any[]} */ (dad.rows);
  const LABEL = /** @type {Record<string, () => string>} */ ({ points: () => t("method.dad.points"), channels: () => t("method.dad.channels"), duration: () => t("method.dad.duration"), range: () => t("method.dad.range"),
    range_method: () => t("method.dad.range_method"), step: () => t("method.dad.step"), frequency: () => t("method.dad.frequency"), cell: () => t("method.dad.cell"), slit: () => t("method.dad.slit"), lamp: () => t("method.dad.lamp") });
  const label = (/** @type {string} */ id) => (LABEL[id] ? LABEL[id]() : id);
  const tr = rows.map(r => `<tr><th scope="row" class="muted" style="font-weight:normal;text-align:left"${tipOf(r.source)}>${EH(label(r.id))}</th><td${tipOf(r.source)}>${EH(r.unit ? t("method.val.unit", { v: r.shown, unit: r.unit }) : r.shown)}</td></tr>`).join("");
  return `<section class="msec" data-mdad="dad"><h4>${EH(t("method.dad.title"))}</h4><table>${tr}</table>${sourceList(rows.map(r => [label(r.id), r.source || null]))}</section>`;
}

/** the document of the open item `f` (null when the route is not served) */
async function load(/** @type {{ k: number }} */ f) {
  try { const d = await J("api/filemethod?k=" + f.k); return d && d.schema === "file_method/1" ? d : null; } catch (_) { return null; }
}

/** the parts of the Method window drawn from the document: the tandem-MS table (`seg`), the LC program (`lc`) and the diode array (`dad`); `lcView` is the view of a `.dam` program, when the method comes from there */
function parts(/** @type {any} */ d, /** @type {any} */ lcView = null) {
  if (!d) return { seg: `<div class="muted sm" data-mfile="none">${EH(t("method.file.fail"))}</div>`, lc: lcView ? lcSection(lcView.lc) : "", dad: lcView ? dadSection(lcView.dad) : "" };
  const body = notes(d) + segments(d) + acquired(d);
  // the program of a loaded .dam wins when it has a gradient; else the file's own method (the section the gradient tests look for)
  const own = !!(lcView && lcView.lc && (lcView.lc.gradient || []).length > 1);
  return { seg: body ? `<h5 style="margin:8px 0 2px">${EH(t("method.file.title"))}</h5>${body}` : "", lc: own ? lcSection(lcView.lc) : lcSection(d.lc || (lcView && lcView.lc), "grad-sec"), dad: dadSection(d.dad || (lcView && lcView.dad)) };
}

/** the section for the open item `f` (an empty string if the file gives nothing to show) */
async function html(/** @type {{ k: number }} */ f) { return parts(await load(f)).seg; }

/** the name/precursor warning of the open item `f` for the scan list (an empty string when the name is fine or the document is not served; never blocks, never throws) */
async function nameWarning(/** @type {{ k: number }} */ f) {
  const d = await load(f);
  if (!d) return "";
  const w = warnTexts({ warnings: (d.warnings || []).filter((/** @type {any} */ x) => x.key === "warn.file.nameMz") });
  return w.length ? `<ul class="sm" role="note" tabindex="0" data-mfile="namewarn" style="margin:0 0 6px;padding:0 0 0 16px;max-height:12em;overflow:auto">${w.map(x => `<li>${EH(x)}</li>`).join("")}</ul>` : "";
}

/** @type {any} */ (window).METODO_FILE = { html, nameWarning, load, parts };
