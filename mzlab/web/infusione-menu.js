// @ts-check
"use strict";
// Right click on a peak of an MSn infusion spectrum: «Fragment this ion» (the child node with that precursor), «Back to the precursor», Mean | Sum. Needs infusione.js.
(() => {
  const { O, isInf, tree, cells, pick, setOpt, node } = INF;
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
  Object.assign(INF, { current, menuPeak });
})();
