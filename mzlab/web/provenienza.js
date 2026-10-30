// @ts-strict
// Provenance of an exported table (schema provenienza/1). The Rust engine (api/provenance, mzlab-core::provenance) writes the first line
// «# mzLab x.y.z · file=... · background=... · ...»; this file only collects what the page knows (files, background, SNIP, smoothing, tolerance)
// and puts the result in the three places a table leaves the program: dl() (CSV/TSV: the line, first row), dlx() (.xlsx: a last sheet) and the clipboard (PROV.tsv).
// The line starts with «#»: pandas skips it (read_csv(..., comment="#")), R too (comment.char = "#"); Excel shows it as row 1 (delete it, or Data > From Text/CSV and start at row 2).
// Every CSV/TSV and .xlsx goes through dl()/dlx() and gets it by default; an export adds the fields it knows ({table, file, p = panel}); the files of mzFinder, made for other programs, pass false.
const PROV = (() => {
  /** the fields of the line, in this order; anything else the page collects is left out */
  const ORDER = ["file", "background", "snip", "smooth", "tolerance", "polarity", "precursor", "charge", "activation", "ce", "rt", "scan", "table", "date", "note"];
  /** what the page knows of the scan behind an MS2/MSn spectrum; unknown fields are left out (an average or several precursors give no single scan) @param {any} p @returns {Record<string, string>} */
  const ofScan = p => {
    /** @type {Record<string, string>} */
    const o = {}, j = p._scan, d = p._a && p._a.data || [], avg = p.avg && p.avg.length > 1;
    const precs = [...new Set(d.map(/** @param {any} x */ x => x.prec).filter(/** @param {any} v */ v => v > 0))], f = E.files[p.k];
    const prec = !avg && j && j.prec > 0 ? j.prec : precs.length === 1 ? precs[0] : p.prec > 0 && !p.all ? p.prec : null;
    if (prec != null) o.precursor = String(+(+prec).toFixed(4));
    if (j && !avg) {
      const z = j.charge ?? j.z; if (z) o.charge = String(z);
      if (j.act) o.activation = j.act;
      if (j.ce != null) o.ce = j.nce ? `NCE ${+(+j.ce).toFixed(2)}` : `${+(+j.ce).toFixed(2)} eV`;
      if (j.rt != null) o.rt = `${(+j.rt).toFixed(3)} min`;
      if (j.no != null) o.scan = String(j.no);
    } else if (!avg && p.r0 != null && p.r1 != null && p.r1 - p.r0 < 0.01) o.rt = `${((p.r0 + p.r1) / 2).toFixed(3)} min`;
    if (f && (f.polarity === "positive" || f.polarity === "negative")) o.polarity = f.polarity;
    return o;
  };
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
    if (p.type === "spec" && p.level >= 2) Object.assign(o, ofScan(p));
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
    return Object.fromEntries(ORDER.filter(k => o[k] != null).map(k => [k, o[k]]));
  }
  /** @param {any} [over] @returns {Promise<{schema: string, version: string, line: string, rows: string[][]}|null>} null (and a message) when the engine does not answer */
  async function get(over) {
    try {
      const r = await fetch("api/provenance?" + new URLSearchParams(ctx(over)));
      if (!r.ok) throw new Error("provenance");
      // «# mzLab x.y.z · key=value ...»
      const [schema, line] = (await r.text()).split("\n"), parts = line.split(" · "), version = parts[0].slice(8);
      return { schema, version, line, rows: [[window.APP_NAME, version], ...parts.slice(1).map(x => [x.slice(0, x.indexOf("=")), x.slice(x.indexOf("=") + 1)])] };
    } catch (_) { if (window.toast) window.toast(I18N.t("prov.fail")); return null; }
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
