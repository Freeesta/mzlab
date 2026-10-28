// @ts-check
// @ts-strict
"use strict";
// «What can this number be?» (block under the m/z calculator, the Adducts panel and the Neutral losses panel): the choices (high / low resolution,
// tolerance, polarity, observed m/z) and the groups of candidates for ONE number or formula. Pure functions of (argument, options, dependencies), also
// run by node in tests/test_calc_qui.py; the page parts are calc-qui-html.js (markup) and calc-qui-ui.js (the three places that show it).
// Candidates only: at unit resolution a match is a coincidence to check, never an identification (Teoria).

const CALCQUI = (() => {
  const C = typeof calcIsExpr === "function" ? { calcIsExpr, calcEval } : require("./calcola.js");
  const HR_FLOOR = 0.003;                                   // Da: the narrowest window for a mass DIFFERENCE in high resolution (as the Neutral losses table always had)
  const LIMIT = 6;                                          // rows shown per group before «show all»
  const num = (/** @type {unknown} */ s) => parseFloat(String(s == null ? "" : s).trim().replace(",", "."));

  // the choices; res / pol are null while the student has not chosen (they follow the files shown)
  const st = /** @type {CqState} */ ({ res: null, ppm: "5", da: "0.5", pol: null, obs: "" });
  const KEY = () => (typeof NOMI !== "undefined" ? NOMI.KEY("calcqui") : "mzlab.calcqui");
  const subs = /** @type {(() => void)[]} */ ([]);
  // what the files say, used while nothing was chosen
  let filesHr = () => true, filesPol = () => /** @type {1 | -1} */ (1);

  function load() {
    try {
      const s = JSON.parse(localStorage.getItem(KEY()) || "null");
      if (s) {
        if (s.res === "hr" || s.res === "lr") st.res = s.res;
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
  function save() { try { localStorage.setItem(KEY(), JSON.stringify({ res: st.res, pol: st.pol, ppm: st.ppm, da: st.da })); } catch (_) { /* blocked */ } }
  /** @param {Partial<CqState>} p */
  function set(p) { Object.assign(st, p); save(); subs.slice().forEach(f => f()); }

  const isHr = () => (st.res ? st.res === "hr" : filesHr());
  const pol = () => (st.pol || filesPol());
  /** The options a computation needs, from the choices. */
  function opts() {
    const hr = isHr(), t = num(hr ? st.ppm : st.da);
    return { hr, pol: pol(), tol: t > 0 ? t : hr ? 5 : 0.5, obs: num(st.obs) > 0 ? num(st.obs) : null, dec: hr ? 4 : 1 };
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
    st, load, save, set, opts, isHr, pol, arg, diffDa, ppmFor, num, LIMIT,
    onChange: (/** @type {() => void} */ f) => { subs.push(f); },
    /** How the choices follow the files while the student has not chosen. */
    follow: (/** @type {() => boolean} */ hr, /** @type {() => 1 | -1} */ p) => { filesHr = hr; filesPol = p; },
  };
})();

// The groups of candidates (second part of CALCQUI: the choices above, the groups here)
const CALCQUIG = (() => {
  const DIFF_REF = 400;                                     // m/z the adduct engine uses for the ppm window of a difference when no observed m/z is given
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
  /** The adducts: for a number its implied neutral masses (the number taken as an observed m/z), for a formula its table (with the observed m/z if typed). */
  async function addotti(/** @type {{ kind: string, v: number, text: string }} */ a, /** @type {any} */ o, /** @type {any} */ dep) {
    const isNum = a.kind === "num", obs = isNum ? a.v : o.obs;
    const ref = obs || DIFF_REF, f = isNum || a.kind === "none" ? "" : a.text;
    if (!f && !obs) return null;
    const tab = await dep.table(f, obs ? String(obs) : "", String(ppmFor(o, ref)), "");
    const rows = tab.rows.filter((/** @type {any} */ r) => r.pol === o.pol);
    const hit = tab.best != null && tab.rows[tab.best].pol === o.pol ? tab.rows[tab.best] : null;
    return { id: "addotti", n: rows.length, near: hit ? Math.abs(hit.ppm || 0) * (o.hr ? 1 : ref * 1e-6) / o.tol : 2, items: rows, tab, formula: !!f, obs: !!obs && !isNum };
  }
  /** The pairs of adducts whose difference is `v` (a number), or the characteristic differences (otherwise). */
  async function differenze(/** @type {{ kind: string, v: number }} */ a, /** @type {any} */ o, /** @type {any} */ dep) {
    const isNum = a.kind === "num" && a.v > 0;
    const ref = o.obs || DIFF_REF, tab = await dep.table("", o.obs ? String(o.obs) : "", String(ppmFor(o, ref, true)), isNum ? String(a.v) : "");
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
