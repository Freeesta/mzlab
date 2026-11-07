// @ts-strict
"use strict";
// Names of the contaminant lists for the formula dialog (asks the index of liste.js): suggestions while typing and the exact-name lookup.
const LISTENOMI = (() => {
  // Names for the formula dialog: one entry per contaminant (name, neutral formula, preferred adduct) of the active lists, in the language of the page.
  // A name is only a way to fill the formula: a scored candidate input, never an identification.
  const fold = (/** @type {string} */ t) => String(t).normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase();
  const adRank = (/** @type {string} */ a) => { const i = ["[M+H]+", "[M-H]-", "[M]+", "[M]-"].indexOf(a); return i < 0 ? 9 : i; };
  // a repeat count written as a suffix («C2H4Ox1») is expanded into plain element counts
  const plainFormula = (/** @type {string} */ f) => {
    const m = f.match(/^(.*?)x(\d+)$/); if (!m) return f;
    return m[1].replace(/([A-Z][a-z]?)(\d*)/g, (/** @type {string} */ _, /** @type {string} */ el, /** @type {string} */ n) => el + (+(n || 1) * +m[2] || ""));
  };
  // «dibutilftalato (DBP)»: the name before the bracket and the text inside it, each one a way to write the same entry
  const parts = (/** @type {string} */ n) => { const m = n.match(/^(.+?)\s*\(([^()]+)\)\s*$/); return m ? [fold(m[1]), fold(m[2])] : []; };
  let NAMES = /** @type {{ idx: any, lang: string, rows: Bag[] }} */ ({ idx: null, lang: "", rows: [] });
  function nameRows() {
    const idx = LISTE.ensure(); if (!idx) return [];
    if (NAMES.idx === idx && NAMES.lang === I18N.lang) return NAMES.rows;
    const /** @type {Map<string, Bag>} */ by = new Map();
    [...idx.pos, ...idx.neg].forEach((/** @type {Bag} */ x) => {
      if (!x.formula || !x.name) return;
      const f = plainFormula(x.formula), k = x.name + "|" + f, old = by.get(k);
      if (!old || adRank(x.adduct) < adRank(old.adduct)) by.set(k, { name: x.name, formula: f, adduct: x.adduct || "", key: fold(x.name), keyEn: fold(x.name_en || x.name), parts: parts(x.name).concat(parts(x.name_en || "")) });
    });
    NAMES = { idx, lang: I18N.lang, rows: [...by.values()] };
    return NAMES.rows;
  }
  /** Entries whose name holds every word of the text (case and accents ignored), names that start with it first. [] while the lists load. */
  function suggest(/** @type {string} */ q, max = 8) {
    const w = fold(q).split(/\s+/).filter(Boolean); if (!w.length) return [];
    const q1 = w.join(" ");
    return nameRows().filter((/** @type {Bag} */ r) => w.every(t => r.key.includes(t)))
      .map((/** @type {Bag} */ r) => ({ r, k: r.key.startsWith(q1) ? 0 : r.key.includes(" " + q1) ? 1 : 2 })).sort((a, b) => a.k - b.k || a.r.key.length - b.r.key.length).slice(0, max).map(o => o.r);
  }
  /** The text of the dialog as a name (optionally followed by an adduct): {formula, adduct} when ONE entry has exactly that name, else null.
   *  The English (ISO) name of a built-in entry is accepted on an Italian page too: «thiamethoxam» is how papers write «tiametoxam»; then either part of a
 *  name with a bracket («dibutilftalato», «DBP»). */
  function byName(/** @type {string} */ text) {
    const m = String(text).match(/^\s*(.*?)\s*(\[.*\].*)?$/); if (!m || !m[1]) return null;
    const k = fold(m[1]), by = (/** @type {(r: Bag) => boolean} */ ok) => nameRows().filter(ok);
    let hit = by(r => r.key === k); if (!hit.length) hit = by(r => r.keyEn === k); if (!hit.length) hit = by(r => r.parts.includes(k));
    return hit.length === 1 ? { formula: hit[0].formula, adduct: (m[2] || hit[0].adduct || "").trim() } : null;
  }
  return { suggest, byName };
})();
