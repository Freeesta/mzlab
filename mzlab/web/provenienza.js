// @ts-check
// Provenance of an exported table (schema provenienza/1). The Rust engine (api/provenance, mzlab-core::provenance) writes the first line
// «# mzLab x.y.z · file=... · background=... · ...»; this file only collects what the page knows (files, background, SNIP, smoothing, tolerance)
// and puts the result in the three places a table leaves the program: dl() (CSV/TSV: the line, first row), dlx() (.xlsx: a last sheet) and the clipboard (PROV.tsv).
// The line starts with «#»: pandas skips it (read_csv(..., comment="#")), R too (comment.char = "#"); Excel shows it as row 1 (delete it, or Data > From Text/CSV and start at row 2).
// Every CSV/TSV and .xlsx goes through dl()/dlx() and gets it by default; an export adds the fields it knows ({table, file, p = panel}); the files of mzFinder, made for other programs, pass false.
const PROV = (() => {
  /** @param {any} p panel @returns {Record<string, string>} */
  const ofPanel = p => {
    /** @type {Record<string, string>} */
    const o = {};
    if (!p) return o;
    const fo = p.fondo || { mode: "none" };
    o.background = fo.mode === "auto" ? "SNIP" : fo.mode === "tract" ? "tract" : p.bk != null && E.files[p.bk] ? `file ${E.files[p.bk].label}` : "none";
    if (fo.mode === "auto") o.snip = `${fo.w} min`;
    if (p.smooth != null) o.smooth = p.smooth ? "on" : "off";
    if (p.tol != null && p.type === "xic") o.tolerance = `${p.tol} Da`;
    return o;
  };
  /** @param {any} [over] fields to set (or {p: panel}); the page state fills the rest @returns {Record<string, string>} */
  function ctx(over) {
    const { p, ...mine } = over || {}, panel = p || (E.active && E.panels.includes(E.active) ? E.active : null);
    const names = (E.files || []).filter(/** @param {any} f */ f => !f.gone && f.kind === E.tab).map(/** @param {any} f */ f => f.label || f.file);
    /** @type {Record<string, string>} */
    const o = { file: names.length > 3 ? names.slice(0, 3).join(", ") + ` (+${names.length - 3})` : names.join(", "), ...ofPanel(panel), date: new Date().toISOString().slice(0, 19) + "Z" };
    if (typeof UIP !== "undefined" && UIP.hrPpm != null && E.tab === "hr") o.tolerance = `${UIP.hrPpm} ppm`;
    for (const [k, v] of Object.entries(mine)) if (v != null) o[k] = String(v);
    return o;
  }
  /** @param {any} [over] @returns {Promise<{schema: string, version: string, line: string, rows: string[][]}|null>} null (and a message) when the engine does not answer */
  async function get(over) {
    try { return await J("api/provenance?" + new URLSearchParams(ctx(over))); }
    catch (_) { if (window.toast) window.toast(I18N.t("prov.fail")); return null; }
  }
  /** text of a table (rows separated by \n) with the provenance line first; unchanged when it already has it or the engine does not answer @param {string} text @param {any} [over] */
  async function tsv(text, over) {
    if (/^# mzLab /.test(text)) return text;
    const r = await get(over);
    return r ? r.line + "\n" + text : text;
  }
  /** the sheets plus a last sheet «Provenance» (field / value) @param {any[]} sheets @param {any} [over] */
  async function sheets(sheets, over) {
    const r = await get(over);
    if (!r) return sheets;
    return [...sheets, { name: I18N.t("prov.sheet"), head: [I18N.t("prov.field"), I18N.t("prov.value")], rows: [["schema", r.schema], ...r.rows], widths: [16, 70] }];
  }
  return { ctx, get, tsv, sheets };
})();
