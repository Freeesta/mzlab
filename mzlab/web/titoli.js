// @ts-check
// The ONE naming scheme of the panels (table in docs/agenti/titoli-pannelli.md): a title says only what the panel shows, never a file name.
// `nome(p, ctx)` is pure (tests/titoli_node.js); `di(p)` reads the page state.
(() => {
  const T = (/** @type {string} */ k, /** @type {any} */ a) => I18N.t(k, a);
  // experiment name and MRM test from esperimenti.js (ESP.mrm is a predicate)
  const MS2 = ESP.ms2, isMrm = ESP.mrm;
  const SUP = ["", "¹", "²", "³"];
  const fnm = (/** @type {number} */ v) => String(+(+v).toFixed(2));
  /** distinct precursors (0.6 Da) of the MS2 files shown @param {any[]} files */
  const nPrecOf = files => { const out = []; for (const f of files) for (const v of f.precursors || []) if (!out.some(u => Math.abs(u - v) < 0.6)) out.push(v); return out.length; };
  /** @param {any} p @param {{ nPrec?: number, lv?: number }} [ctx] */
  function nome(p, ctx = {}) {
    const t = typeof p.title === "string" ? p.title : "";
    const own = t && t[0] !== "@" && !/^MS²$/.test(t);  // typed by the student: kept
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
      if (p.tab === MS2) {
        const ms = "MS" + SUP[Math.min(ctx.lv ?? 2, 3)];
        if (prec == null) return T("panel.title.ticMs2All", { ms });
        return (ctx.nPrec ?? 2) > 1 ? T("panel.title.ms2prec", { ms, prec }) : T("panel.title.ticMs2", { ms });  // one precursor: the trace IS the TIC
      }
      return prec != null ? T("panel.title.kindPrec", { kind: K, prec }) : K;
    }
    if (p.type === "spec") {
      if (p.uv) return T("panel.title.uv");
      const ms = "MS" + SUP[Math.min(p.level, 3)];
      if (p.level > 1) return prec != null ? T("panel.title.productOf", { ms, prec }) : ms;
      return "MS¹";
    }
    if (p.type === "xic") return (p.traces || []).length === 1 ? T("panel.title.xicOne", { label: p.traces[0].label }) : T("panel.title.xic");
    if (isMrm(p)) {
      const sim = t === "@sim", trs = Array.isArray(p.tr) ? p.tr : p.tr ? [p.tr] : [];
      // a SIM ion is named as in its menu (m/z or «ion N» from 1), never by key
      return trs.length === 1 ? T(sim ? "panel.title.simOne" : "panel.title.mrmOne", { tr: sim ? SIMUI.label(trs[0]) : trs[0] }) : T(sim ? "panel.title.sim" : "panel.title.mrm");
    }
    if (p.type === "map") return p.mapKind === "pda" ? T("panel.title.mapPda") : T("panel.title.map");
    return t;
  }
  /** the name of a live panel (precursors counted over the MS2 files shown, level of those with its precursor) @param {any} p */
  const di = p => { const fs = tabFiles(MS2).filter((/** @type {any} */ f) => f.vis && !f.gone), fp = fs.filter(f => p.prec && f.precursors?.some(q => Math.abs(q - p.prec) < .05)); return nome(p, { nPrec: nPrecOf(fs), lv: (fp[0] ? fp : fs).reduce((m, f) => Math.min(m, f.lv), 9) % 9 || 2 }); };
  window.TITOLI = { nome, di, nPrecOf };
})();
