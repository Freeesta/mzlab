// @ts-check
"use strict";
// MSn infusion (kind "infusion"): the precursor tree of the file (/api/msn_albero, built in Rust from the index), the nodes as scan filters of the cells
// of the bench (filt = "msn:<id>", with mtol/mce of the tree options added to every request by qs()), the Mean | Sum switch (modo=somma) and the
// right-click entries of a peak. Classic script, loaded before banco.js; the tree panel is in infusione-albero.js.
const INF = (() => {
  /** Options of the tree: tolerance of the precursors ("nom" | "0.01" | "10ppm"), energies separate or merged, mean or sum, deepest level shown (0 = all). */
  const O = { tol: "nom", ce: true, sum: false, lv: 0 };
  const isInf = (/** @type {any} */ f) => !!f && !f.gone && f.kind === "infusion";
  const any = () => E.files.some(isInf);
  const treeUrl = (/** @type {number} */ k) => `api/msn_albero?k=${k}&tol=${O.tol}&ce=${O.ce ? 1 : 0}`;
  /** The tree of a file for the options in force. @returns {Promise<any>} */
  const tree = k => memo(`msn|${k}|${O.tol}|${O.ce}`, () => J(treeUrl(k)));
  const prec = (/** @type {any} */ n) => `${+n.prec.toFixed(4)}`;
  /** Nodes of a tree in the order of the display (depth first), each with its depth and the path of precursors from the survey. */
  function rows(/** @type {any} */ j) {
    /** @type {Map<string | null, any[]>} */
    const kids = new Map();
    j.nodi.forEach((/** @type {any} */ n) => { const l = kids.get(n.genitore) || []; l.push(n); kids.set(n.genitore, l); });
    /** @type {any[]} */
    const out = [];
    const walk = (/** @type {string | null} */ id, /** @type {number} */ depth, /** @type {string[]} */ path) => (kids.get(id) || []).forEach(n => {
      const p = [...path, prec(n)];
      out.push({ n, depth, path: p.join(" → ") });
      walk(n.id, depth + 1, p);
    });
    walk(null, 1, []);
    return out;
  }
  /** The nodes as the entries of the scan-filter menu of a cell: [{key, level, n, label, msn}] (the survey is the empty choice of the menu). */
  async function groups(/** @type {number} */ k) {
    const j = await tree(k);
    return rows(j).map(r => ({ key: "msn:" + r.n.id, level: r.n.livello, n: r.n.n, label: `MS${r.n.livello} ${r.path}${r.n.ce != null ? ` · CE ${r.n.ce}` : ""}`, msn: r.n }));
  }
  /** Adds the grouping (and the sum) of the tree to a request that names a node: the one place that knows the options. */
  const qs = (/** @type {string} */ url) => {
    if (!/msn(:|%3A)/i.test(url)) return url;
    let u = url + `&mtol=${O.tol}&mce=${O.ce ? 1 : 0}`;
    if (O.sum && /^(\.\/)?api\/(spectrum|spectra|scanavg)\b/.test(u)) u += "&modo=somma";
    return u;
  };
  const node = (/** @type {any} */ j, /** @type {string | null | undefined} */ filt) => (filt && filt.startsWith("msn:") ? j.nodi.find((/** @type {any} */ n) => "msn:" + n.id === filt) || null : null);
  const richest = (/** @type {any} */ j) => j.nodi.reduce((/** @type {any} */ b, /** @type {any} */ n) => (!b || n.n > b.n ? n : b), null);
  /** The chromatogram cell and the spectrum cells that go with a cell. */
  function cells(/** @type {any} */ p) {
    const c = p.type === "chrom" ? p : E.panels.find(q => q.id === (p.link ?? p.src)) || null;
    return { c, specs: c ? E.panels.filter(q => q.type === "spec" && q.link === c.id) : [p] };
  }
  /** Chooses a node for the cell and its companions: the filter, the time window of the whole node on the spectrum (a window dragged on the TIC wins afterwards). */
  async function pick(/** @type {any} */ p, /** @type {string} */ id) {
    const f = E.files[p.k ?? cells(p).specs[0]?.k ?? E.cur];
    if (!isInf(f)) return;
    const gs = await groups(f.k), g = gs.find(x => x.key === "msn:" + id), n = g && g.msn;
    if (!n) return;
    BANCO.setFilter(p, g.key, gs);
    const { c } = cells(p), pad = 1e-4;
    if (c && c.cur == null) c.cur = (n.rt0 + n.rt1) / 2;
    if (c) pushLinked(c, n.rt0 - pad, n.rt1 + pad, f.k, false);
    E.panels.filter(q => q.type === "spec" && q.filt === g.key && q.link == null).forEach(q => { q.r0 = n.rt0 - pad; q.r1 = n.rt1 + pad; q.k = f.k; draw(q); });
  }
  /** Starting layout of the infusion tab: the TIC of the richest node and its spectrum. */
  async function layout(/** @type {any} */ c, /** @type {any} */ sp, /** @type {any} */ f) {
    try { const j = await tree(f.k), n = richest(j); if (n) await pick(c, n.id); } catch (e) { console.info("INF layout", e); }
  }
  function reset() {
    CACHE.clear(); if (typeof SC !== "undefined") { SC.m.clear(); SC.n.clear(); }
    E.panels.forEach(p => { if (p.filt && p.filt.startsWith("msn:")) { p.filt = null; p.flv = null; } });
  }
  /** Changes an option of the tree. A new grouping makes every node id of the cells meaningless: they go back to the richest node. */
  async function setOpt(/** @type {"tol" | "ce" | "sum" | "lv"} */ key, /** @type {any} */ v) {
    const regroup = key === "tol" || key === "ce";
    /** @type {any} */ (O)[key] = v;
    if (key === "sum") { CACHE.clear(); if (typeof SC !== "undefined") { SC.m.clear(); SC.n.clear(); } }
    if (regroup) reset();
    const cs = regroup ? E.panels.filter(q => q.type === "chrom" && isInf(E.files[q.k ?? E.cur]) || q.tab === "infusion" && q.type === "chrom") : [];
    for (const c of cs) { const f = E.files.find(isInf); if (f) await layout(c, null, f); }
    E.panels.forEach(q => draw(q));
    if (window.BANCO) BANCO.renderInfo();
    uiSave();
  }
  const state = () => (any() ? { ...O } : undefined);
  const restore = (/** @type {any} */ s) => { if (s && typeof s === "object") Object.assign(O, { tol: String(s.tol || "nom"), ce: s.ce !== false, sum: !!s.sum, lv: +s.lv || 0 }); };
  /** The node a cell shows, from the tree already fetched: null when the cell shows the survey or another file. */
  async function current(/** @type {any} */ p) {
    const f = E.files[p.k ?? cells(p).specs[0]?.k ?? E.cur];
    if (!isInf(f) || !p.filt) return { j: null, n: null, f };
    const j = await tree(f.k);
    return { j, n: node(j, p.filt), f };
  }
  /** Right click on a peak of an infusion spectrum: fragment this ion (the child node with that precursor) and go back to the precursor. */
  function menuPeak(/** @type {any} */ p, /** @type {number} */ m) {
    try {
      const f = E.files[p.k];
      if (!isInf(f) || p.type !== "spec") return [];
      const out = [];
      const tolOf = (/** @type {number} */ x) => (O.tol === "nom" ? 0.5 : O.tol.endsWith("ppm") ? x * parseFloat(O.tol) * 1e-6 : parseFloat(O.tol));
      out.push({ label: I18N.t("inf.frag"), tip: I18N.t("inf.frag.tip"), fn: async () => {
        const { j: tj, n } = await current(p), jj = tj || await tree(f.k);
        const parent = n ? n.id : null;
        const kid = jj.nodi.filter((/** @type {any} */ x) => x.genitore === parent && Math.abs(x.prec - m) <= tolOf(m)).sort((/** @type {any} */ a, /** @type {any} */ b) => Math.abs(a.prec - m) - Math.abs(b.prec - m))[0];
        if (!kid) return nearMsg(I18N.t("inf.frag.none"));
        pick(p, kid.id);
      } });
      out.push({ label: I18N.t("inf.up"), tip: I18N.t("inf.up.tip"), fn: async () => {
        const { j: tj, n } = await current(p);
        if (!n || !tj) return nearMsg(I18N.t("inf.up.none"));
        if (n.genitore == null) { const { c, specs } = cells(p); [c, ...specs].filter(Boolean).forEach(q => BANCO.setFilter(q, null, [])); return; }
        pick(p, n.genitore);
      } });
      out.push({ label: I18N.t(O.sum ? "inf.toMean" : "inf.toSum"), tip: I18N.t("inf.sum.tip"), fn: () => setOpt("sum", !O.sum) });
      return out;
    } catch (e) { console.info("INF menu", e); return []; }
  }
  return { O, isInf, any, tree, rows, groups, qs, node, richest, cells, pick, layout, setOpt, state, restore, current, menuPeak, prec };
})();
window.INF = INF;
