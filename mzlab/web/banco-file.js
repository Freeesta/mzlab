// @ts-check
// «File and instrument» tab of the bench information bar (banco.js, BODY.fil): the HTML of its sections on ONE grid (labels column of the same width in
// every section; «Scans by type» as a list: name and count on a line, the RT range small under it) and the helper of the MS level label (MS<sup>n</sup>),
// also used by the other tabs of the bar. Loaded by banco.js on first use; the numbers come from /api/fileinfo, nothing is computed here.

const CSS = `#hri-body .fi-kv{display:grid;grid-template-columns:minmax(9ch,34%) minmax(0,1fr);gap:2px 8px;margin:0}
#hri-body .fi-kv dt{color:var(--muted);overflow-wrap:break-word}#hri-body .fi-kv dd{margin:0;overflow-wrap:anywhere;user-select:text}
#hri-body .fi-nw{white-space:nowrap}#hri-body .fi-ph{text-decoration:underline dotted;text-underline-offset:2px;cursor:help}
#hri-body .fi-types{list-style:none;margin:0;padding:0}#hri-body .fi-types li{display:grid;grid-template-columns:minmax(6ch,1fr) auto;gap:0 8px;padding:2px 0;border-bottom:1px solid var(--line)}
#hri-body .fi-tn{min-width:0}#hri-body .fi-w{display:inline-block;max-width:100%;overflow-wrap:anywhere}#hri-body .fi-c{white-space:nowrap;text-align:right;min-width:6ch;font-variant-numeric:tabular-nums;font-weight:600}
#hri-body .fi-rt{grid-column:1/-1;white-space:nowrap;font-size:11px;color:var(--muted);font-variant-numeric:tabular-nums}
#hri-body sup,.ms-lv sup{font-size:.75em;line-height:0}`;

/** @param {unknown} s */
const esc = s => EH(String(s ?? ""));
/** @param {unknown} v @param {number} d */
const fix = (v, d) => (v == null || !Number.isFinite(+v) ? "" : (+v).toFixed(d));
/** First letter of a name in capitals when it is all lowercase («orbitrap» → «Orbitrap»); acronyms such as FTMS stay as the file says them. @param {string} s */
const cap = s => s.replace(/^[a-zà-ÿ]/, c => c.toUpperCase());

/** The MS level as HTML: MS<sup>n</sup> (copied as plain «MSn»). @param {number | string} l */
export const msLv = l => `<span class="ms-lv">MS<sup>${esc(l)}</sup></span>`;

/** One section: heading and rows [label, value as HTML]; rows with no value are left out, and a section with no rows is not drawn.
 *  @param {string} title @param {[string, string][]} rows */
function section(title, rows) {
  const r = rows.filter(x => x[1] !== "");
  return r.length ? `<h4>${esc(title)}</h4><dl class="fi-kv">${r.map(([a, v]) => `<dt>${esc(a)}</dt><dd>${v}</dd>`).join("")}</dl>` : "";
}

/** @param {string[] | undefined} a @param {(s: string) => string} f */
const list = (a, f) => (a || []).filter(Boolean).map(f).join(", ");
const PLACEHOLDER = /custom unreleased software tool/i;

/** The body of the tab. @param {any} j answer of /api/fileinfo */
export function fileHtml(j) {
  const t = I18N.t, lv = Object.keys(j.levels || {}).sort((a, b) => +a - +b);
  const range = (/** @type {number[]} */ r) => `${fix(r[0], 2)} – ${fix(r[1], 2)}`;
  const mz = lv.filter(l => j.mz_range && j.mz_range[l]).map(l => `<span class="fi-nw">${msLv(l)}: ${range(j.mz_range[l])}</span>`).join(" · ");
  const scans = lv.length ? `${esc(j.scans)} (${lv.map(l => `<span class="fi-nw">${msLv(l)}: ${esc(j.levels[l])}</span>`).join(", ")})` : esc(j.scans);
  const sw = list(j.software, s => (PLACEHOLDER.test(s) ? `<span class="fi-ph" title="${esc(t("bn.f.placeholder"))}">${esc(s)}</span>` : esc(s)));
  const types = (j.filters || []).map((/** @type {any} */ g) => `<li><span class="fi-tn">${String(g.key).split(/\s+/).map(w => `<span class="fi-w">${esc(w)}</span>`).join(" ")}</span><span class="fi-c">${esc(g.n)}</span>`
    + `<span class="fi-rt">${esc(t("bn.f.rtType", { a: fix(g.rt0, 2), b: fix(g.rt1, 2) }))}</span></li>`).join("");
  return section(t("side.file"), [[t("lib.col.name"), esc(j.file)], [t("bn.f.date"), esc(j.start || t("bn.f.noDate"))], [t("bn.f.rtRange"), range(j.rt)], [t("bn.f.mzRange"), mz], [t("lib.all.scans"), scans]])
    + section(t("bn.f.instrument"), [[t("bn.f.model"), esc(cap(j.instrument || ""))], [t("bn.f.serial"), esc(j.serial || "")], [t("bn.f.analyzers"), list(j.analyzers, s => esc(cap(s)))], [t("bn.f.components"), list(j.components, s => esc(cap(s)))]])
    + section(t("bn.f.software"), [["Software", sw], [t("bn.f.source"), list(j.source_files, esc)], [t("bn.f.conversion"), list(j.conversion, esc)]])
    + (types ? `<h4>${esc(t("bn.f.byType"))}</h4><ul class="fi-types">${types}</ul>` : "");
}

if (!document.getElementById("fi-css")) { const st = document.createElement("style"); st.id = "fi-css"; st.textContent = CSS; document.head.appendChild(st); }
window.BFIL = { fileHtml, msLv };
