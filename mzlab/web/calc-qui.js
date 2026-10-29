// @ts-check
// @ts-strict
"use strict";
// «What can this number be?» (block under the m/z calculator, the Adducts panel and the Neutral losses panel): the choices (tolerance, polarity; the
// resolution is never chosen: it comes from the open files, or from the number itself when none is open) and the groups of candidates for ONE number or formula. Pure functions of (argument, options, dependencies), also
// run by node in tests/test_calc_qui.py; the page parts are calc-qui-html.js (markup) and calc-qui-ui.js (the three places that show it).
// Candidates only: at unit resolution a match is a coincidence to check, never an identification (Teoria).

const CALCQUI = (() => {
  const C = typeof calcIsExpr === "function" ? { calcIsExpr, calcEval } : require("./calcola.js");
  const HR_FLOOR = 0.003;                                   // Da: the narrowest window for a mass DIFFERENCE in high resolution (as the Neutral losses table always had)
  const LIMIT = 6;                                          // rows shown per group before «show all»
  const num = (/** @type {unknown} */ s) => parseFloat(String(s == null ? "" : s).trim().replace(",", "."));

  // the choices; pol is null while the student has not chosen (it follows the files shown)
  const st = /** @type {CqState} */ ({ ppm: "5", da: "0.5", pol: null });
  const KEY = () => (typeof NOMI !== "undefined" ? NOMI.KEY("calcqui") : "mzlab.calcqui");
  const subs = /** @type {(() => void)[]} */ ([]);
  // what the open files say: { hr, name } (name = the instrument / analyzer that decided), or null when no file is open
  let filesMode = () => /** @type {{ hr: boolean, name: string } | null} */ (null), filesPol = () => /** @type {1 | -1} */ (1);

  function load() {
    try {
      const s = JSON.parse(localStorage.getItem(KEY()) || "null");
      if (s) {
        if (s.pol === 1 || s.pol === -1) st.pol = s.pol;
        if (typeof s.ppm === "string") st.ppm = s.ppm;
        if (typeof s.da === "string") st.da = s.da;
        return;
      }
      // the calculator's older choices (polarity and ppm) carry over
      const o = JSON.parse(localStorage.getItem(NOMI.KEY("calc")) || "null");
      if (o && (o.pol === 1 || o.pol === -1)) st.pol = o.pol;
      if (o && typeof o.tol === "string") st.ppm = o.tol;
    } catch (_) { /* blocked or unreadable: the defaults */ }
  }
  function save() { try { localStorage.setItem(KEY(), JSON.stringify({ pol: st.pol, ppm: st.ppm, da: st.da })); } catch (_) { /* blocked */ } }
  /** @param {Partial<CqState>} p */
  function set(p) { Object.assign(st, p); save(); subs.slice().forEach(f => f()); }

  const pol = () => (st.pol || filesPol());
  /** The resolution, from the open files; with none open, from what is typed: a formula or a number with three or more decimals is high resolution, anything else low.
   * @param {{ kind?: string, text?: string }} [a] the argument (see `arg`) @returns {{ hr: boolean, src: "files" | "formula" | "decimals" | "short" | "none", name: string }} */
  function mode(a) {
    const f = filesMode();
    if (f) return { hr: f.hr, src: "files", name: f.name };
    if (a && a.kind === "formula") return { hr: true, src: "formula", name: "" };
    const m = a && a.kind === "num" ? /[.,](\d+)\s*$/.exec(a.text || "") : null;
    if (m && m[1].length >= 3) return { hr: true, src: "decimals", name: "" };
    return { hr: false, src: a && a.kind === "num" ? "short" : "none", name: "" };
  }
  /** The options a computation needs, from the choices and the argument. @param {{ kind?: string, text?: string }} [a] */
  function opts(a) {
    const md = mode(a), hr = md.hr, t = num(hr ? st.ppm : st.da);
    return { hr, mode: md, pol: pol(), tol: t > 0 ? t : hr ? 5 : 0.5, dec: hr ? 4 : 1 };
  }
  /** Window in Da around `v` for a mass DIFFERENCE (losses, adduct differences). @param {{ hr: boolean, tol: number }} o @param {number} v */
  const diffDa = (o, v) => (o.hr ? Math.max(v * o.tol * 1e-6, HR_FLOOR) : o.tol);
  /** The tolerance in ppm the adduct engine takes, from the choices; ref = the m/z the window is taken at. @param {{ hr: boolean, tol: number }} o @param {number} ref @param {boolean} [diff] */
  const ppmFor = (o, ref, diff) => (o.hr ? (diff ? Math.max(o.tol, HR_FLOOR / ref * 1e6) : o.tol) : o.tol / ref * 1e6);

  /** What the field holds: a number (typed, or the RESULT of an operation), a formula, or nothing.
   * @param {string} text @param {number} ans  @returns {{ kind: "none" | "num" | "formula", v: number, text: string }} */
  function arg(text, ans) {
    const t = String(text || "").trim();
    if (!t) return { kind: "none", v: NaN, text: "" };
    if (!C.calcIsExpr(t)) return { kind: "formula", v: NaN, text: t };
    const r = C.calcEval(t, ans);
    return r.v != null && isFinite(r.v) ? { kind: "num", v: r.v, text: t } : { kind: "none", v: NaN, text: t };
  }

  return {
    st, load, save, set, opts, mode, isHr: (/** @type {{ kind?: string, text?: string }} */ a) => mode(a).hr, pol, arg, diffDa, ppmFor, num, LIMIT,
    onChange: (/** @type {() => void} */ f) => { subs.push(f); },
    /** Where the resolution and the polarity come from: the open files. */
    follow: (/** @type {() => { hr: boolean, name: string } | null} */ m, /** @type {() => 1 | -1} */ p) => { filesMode = m; filesPol = p; },
  };
})();

// The groups of candidates (second part of CALCQUI: the choices above, the groups here)
const CALCQUIG = (() => {
  const DIFF_REF = 400;                                     // m/z the adduct engine uses for the ppm window of a difference
  const { diffDa, ppmFor } = CALCQUI;
  /** Known ions of the lists (contaminants, Keller, suspects) at `v` as an ion m/z. */
  function liste(/** @type {number} */ v, /** @type {any} */ o, /** @type {any} */ dep) {
    if (!dep.idx) return null;
    const items = o.hr ? dep.find(dep.idx, o.pol, v, o.tol, false) : dep.findNominal(dep.idx, o.pol, v, o.tol);
    const near = items.length ? Math.min(...items.map((/** @type {any} */ x) => (o.hr ? Math.abs(x.err) / o.tol : Math.abs(x.diffDa) / o.tol))) : 1;
    return { id: "liste", n: items.length, near, items };
  }
  /** Single losses, pairs and repetitions that explain `v` as a mass difference. */
  function perdite(/** @type {number} */ v, /** @type {any} */ o, /** @type {any} */ dep) {
    const w = diffDa(o, v), c = dep.lossCombos(v, dep.mass, w, dep.losses);
    const row = (/** @type {string} */ kind, /** @type {any[]} */ ls, /** @type {number} */ n) => {
      const m = ls.reduce((a, l) => a + dep.mass(l.f), 0) * n;
      return { kind, ls, n, mass: m, err: m - v };
    };
    const items = [...c.single.map((/** @type {any[]} */ x) => row("single", x, 1)), ...c.pairs.map((/** @type {any[]} */ p) => row("pair", p, 1)), ...c.reps.map((/** @type {any} */ r) => row("rep", [r.l], r.n))]
      .sort((a, b) => Math.abs(a.err) - Math.abs(b.err));
    return { id: "perdite", n: items.length, near: items.length ? Math.abs(items[0].err) / w : 1, items };
  }
  /** The adducts: for a number its implied neutral masses (the number taken as an observed m/z), for a formula its table. */
  async function addotti(/** @type {{ kind: string, v: number, text: string }} */ a, /** @type {any} */ o, /** @type {any} */ dep) {
    const isNum = a.kind === "num", obs = isNum ? a.v : 0;
    const ref = obs || DIFF_REF, f = isNum || a.kind === "none" ? "" : a.text;
    if (!f && !obs) return null;
    const tab = await dep.table(f, obs ? String(obs) : "", String(ppmFor(o, ref)), "");
    const rows = tab.rows.filter((/** @type {any} */ r) => r.pol === o.pol);
    const hit = tab.best != null && tab.rows[tab.best].pol === o.pol ? tab.rows[tab.best] : null;
    return { id: "addotti", n: rows.length, near: hit ? Math.abs(hit.ppm || 0) * (o.hr ? 1 : ref * 1e-6) / o.tol : 2, items: rows, tab, formula: !!f, obs: false };
  }
  /** The pairs of adducts whose difference is `v` (a number), or the characteristic differences (otherwise). */
  async function differenze(/** @type {{ kind: string, v: number }} */ a, /** @type {any} */ o, /** @type {any} */ dep) {
    const isNum = a.kind === "num" && a.v > 0;
    const ref = DIFF_REF, tab = await dep.table("", "", String(ppmFor(o, ref, true)), isNum ? String(a.v) : "");
    if (!isNum) return { id: "diff", n: tab.diffs.length, near: 3, items: tab.diffs, tab, catalogue: true };
    const items = (tab.delta ? tab.delta.hits : []).slice().sort((x, y) => Math.abs(x.err_mda) - Math.abs(y.err_mda));
    return { id: "diff", n: items.length, near: items.length ? Math.abs(items[0].err_mda) / 1000 / (ref * ppmFor(o, ref, true) * 1e-6) : 1, items, tab };
  }

  /** The groups for the argument, tightest match first; empty groups are dropped (the catalogue of differences stays for a formula or an empty field).
   * dep: { idx, find, findNominal, table, mass, losses, lossCombos }  @returns {Promise<CqGroup[]>} */
  async function gruppi(/** @type {{ kind: "none" | "num" | "formula", v: number, text: string }} */ a, /** @type {any} */ o, /** @type {any} */ dep) {
    /** @type {any[]} */ const gs = [];
    const isNum = a.kind === "num" && a.v > 0;
    if (isNum) gs.push(liste(a.v, o, dep), perdite(a.v, o, dep));
    const [ad, df] = await Promise.all([addotti(a, o, dep), differenze(a, o, dep)]);
    gs.push(ad, df);
    return gs.filter(g => g && (g.n > 0 || g.id === "diff" && g.catalogue)).sort((x, y) => x.near - y.near);
  }
  return Object.assign(CALCQUI, { gruppi, liste, perdite });
})();
if (typeof module !== "undefined") module.exports = CALCQUI;
