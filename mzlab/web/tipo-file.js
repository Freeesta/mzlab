// @ts-check
"use strict";
// Experiment type of a file: the one the engine detects (Auto) or a manual choice remembered per file name. Used by the start-screen table
// (shell.js), by the file list of the viewer (explore.js) and by the XIC button; the manual choice always wins.
const TIPOF = (() => {
  const SKEY = "filekind";
  /** @returns {Record<string, string>} */
  const read = () => { try { return JSON.parse(NOMI.get(SKEY) || "{}") || {}; } catch (_) { return {}; } };
  /** Manual choice of a file, or null for Auto. @param {string} name */
  const manual = name => { const k = read()[name]; return ESP.ids.includes(k) ? k : null; };
  /** @param {string} name @param {string | null} kind null = back to Auto */
  const set = (name, kind) => { const m = read(); if (kind && ESP.ids.includes(kind)) m[name] = kind; else delete m[name]; NOMI.set(SKEY, JSON.stringify(m)); };
  /** The file with its effective kind; `kindAuto` keeps the engine's. @param {any} f @param {string} name */
  const apply = (f, name) => { const auto = f.kindAuto || f.kind, m = manual(name); return { ...f, kindAuto: auto, kind: m || auto, manual: !!m }; };
  /** The MSn infusion tools (infusione*.js) are not in the start-up list: they arrive before the first scan file (not MRM) enters the session: a DDA file has a precursor tree too (infusione-albero.js probes it). */
  const loadInf = async () => { await carica("infusione.js"); await Promise.all([carica("infusione-albero.js"), carica("infusione-menu.js")]); };
  const infusione = async (/** @type {any[]} */ session) => { if (!window.INF && session.some(s => !ESP.mrm({ type: apply(s, s.file).kind }))) await loadInf(); };
  /** Applies the choice to the files already opened in the viewer, without reloading. @param {string[]} names */
  const viewer = async names => {
    if (typeof E === "undefined" || !E.files) return;
    if (!window.INF && names.some(n => manual(n) === "infusion")) await loadInf();
    E.files.forEach((/** @type {any} */ e) => { if (names.includes(e.file)) { Object.assign(e, apply(e, e.file)); e.lv = e.kind === ESP.ms2 ? 2 : 1; } });
    if (typeof renderFileList === "function") renderFileList();
    if (window.XICBTN && XICBTN.sync) E.panels.forEach((/** @type {any} */ p) => XICBTN.sync(p));
  };
  /** Reason of the automatic choice, from the facts the engine reports (`msn`: motive, levels, nodes, keys). @param {any} f */
  const why = f => {
    const auto = f.kindAuto || f.kind, m = f.msn, p = { n: m ? m.keys : 0, max: 512 };
    const byMotive = { blocks: () => I18N.t("load.exp.why.blocks", p), method_without_lc: () => I18N.t("load.exp.why.method_without_lc", p), name: () => I18N.t("load.exp.why.name", p) };
    const byKind = { ms2: () => I18N.t("load.exp.why.ms2"), mrm: () => I18N.t("load.exp.why.mrm"), infusion: byMotive[m && m.reason] || (() => I18N.t("load.exp.why.infusion")) };
    return (byKind[auto] || (() => I18N.t("load.exp.why.full")))();
  };
  return { get KINDS() { return ESP.ids; }, manual, set, apply, viewer, why, infusione };
})();
