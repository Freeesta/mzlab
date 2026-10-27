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
  /** Applies the choice to the files already opened in the viewer, without reloading. @param {string[]} names */
  const viewer = names => {
    if (typeof E === "undefined" || !E.files) return;
    E.files.forEach((/** @type {any} */ e) => { if (names.includes(e.file)) { Object.assign(e, apply(e, e.file)); e.lv = e.kind === ESP.ms2 ? 2 : 1; } });
    if (typeof renderFileList === "function") renderFileList();
    if (window.XICBTN && XICBTN.sync) E.panels.forEach((/** @type {any} */ p) => XICBTN.sync(p));
  };
  /** Reason of the automatic choice, from the facts the engine reports (`msn`: motive, levels, nodes, keys). @param {any} f */
  const why = f => {
    const auto = f.kindAuto || f.kind, m = f.msn, p = { n: m ? m.chiavi ?? m.nodi : 0, max: 512 };
    const byMotive = { blocchi: () => I18N.t("load.exp.why.blocchi", p), metodo_senza_lc: () => I18N.t("load.exp.why.metodo_senza_lc", p), nome: () => I18N.t("load.exp.why.nome", p) };
    const byKind = { ms2: () => I18N.t("load.exp.why.ms2"), mrm: () => I18N.t("load.exp.why.mrm"), infusion: byMotive[m && m.motivo] || (() => I18N.t("load.exp.why.infusion")) };
    return (byKind[auto] || (() => I18N.t("load.exp.why.full")))();
  };
  return { get KINDS() { return ESP.ids; }, manual, set, apply, viewer, why };
})();
