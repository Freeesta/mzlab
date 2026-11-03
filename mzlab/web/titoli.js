// @ts-check
// The ONE naming scheme of the panels (table in docs/agenti/titoli-pannelli.md): a title says only what the panel shows, never a file name.
// `nome(p, ctx)` is pure (tests/titoli_node.js); `di(p)` reads the state of the page. A title typed by the student is kept as written.
(() => {
  const T = (/** @type {string} */ k, /** @type {any} */ a) => I18N.t(k, a);
  const SUP = ["", "¹", "²", "³"];
  const fnm = (/** @type {number} */ v) => String(+(+v).toFixed(2));
  /** distinct precursors (0.6 Da) of the MS2 files shown @param {any[]} files */
  const nPrecOf = files => { const out = []; for (const f of files) for (const v of f.precursors || []) if (!out.some(u => Math.abs(u - v) < 0.6)) out.push(v); return out.length; };
  /** @param {any} p @param {{ nPrec?: number }} [ctx] */
  function nome(p, ctx = {}) {
    const t = typeof p.title === "string" ? p.title : "";
    const own = t && t[0] !== "@" && !/^MS²$/.test(t);  // typed by the student
    if (own) return t;
    const prec = p.prec != null && p.prec !== "" ? fnm(p.prec) : null;
    if (p.type === "chrom") {
      const kind = p.kind || "tic";
      if (kind === "pda") {
        if (p.mz0 == null) return T("panel.title.pdaAll");
        const a = fnm(p.mz0), b = p.mz1 != null && Math.abs(p.mz1 - p.mz0) > 1e-9 ? fnm(p.mz1) : null;
        return T("panel.title.pda", { nm: b ? `${a}–${b}` : a });
      }
      const K = kind === "bpc" ? "BPC" : kind === "tic" ? "TIC" : null;
      if (!K) return T(`ctl.kind.${kind}`) === `ctl.kind.${kind}` ? String(kind).toUpperCase() : T(`ctl.kind.${kind}`);
      if (p.tab === "ms2") {
        if (prec == null) return T("panel.title.ticMs2All");
        return (ctx.nPrec ?? 2) > 1 ? T("panel.title.ms2prec", { prec }) : T("panel.title.ticMs2");  // one precursor in the file: the trace IS the TIC of the experiment
      }
      return prec != null ? T("panel.title.kindPrec", { kind: K, prec }) : K;
    }
    if (p.type === "spec") {
      if (p.uv) return T("panel.title.uv");
      if (p.level > 1) return prec != null ? T("panel.title.productOf", { prec }) : "MS" + SUP[Math.min(p.level, 3)];
      return "MS¹";
    }
    if (p.type === "xic") return (p.traces || []).length === 1 ? T("panel.title.xicOne", { label: p.traces[0].label }) : T("panel.title.xic");
    if (p.type === "mrm") {
      const sim = t === "@sim", trs = Array.isArray(p.tr) ? p.tr : p.tr ? [p.tr] : [];
      return trs.length === 1 ? T(sim ? "panel.title.simOne" : "panel.title.mrmOne", { tr: trs[0] }) : T(sim ? "panel.title.sim" : "panel.title.mrm");
    }
    if (p.type === "map") return p.mapKind === "pda" ? T("panel.title.mapPda") : T("panel.title.map");
    return t;
  }
  /** the name of a live panel (precursors counted over the MS2 files that are shown) @param {any} p */
  const di = p => nome(p, { nPrec: nPrecOf(tabFiles("ms2").filter((/** @type {any} */ f) => f.vis && !f.gone)) });
  window.TITOLI = { nome, di, nPrecOf };
})();
