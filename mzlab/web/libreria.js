// @ts-strict
// @ts-noany
"use strict";
// Part D: compare a MS2 spectrum with the spectral libraries (MSP, MGF) the user has loaded. Classic script, loaded after settings.js/explore.js.
// This stub stays at start-up: the menu actions, the synchronous helpers (query of a spectrum, levels, grouping, mirror plot) and LIB, whose other members
// load libreria-corpo.js on first use (worker supervisor, «Librerie» window, low-resolution results window). «What is it?» and «Identify all the MS2» are libreria-finestra.js.
// Nothing here touches drawSpec: the mirror is a canvas of its own.
const LIB = (() => {
  // tolerances: the profile of hr.js when it exists, otherwise 10 ppm for a high resolution instrument and 0.5 Da
  function profOf(/** @type {Bag} */ f) {
    try { if (window.HR && HR.prof) { const q = HR.prof(f, 2); if (q) return { hr: !!q.hr, v: q.hr ? q.tol : 0.5, unit: q.hr ? "ppm" : "Da" }; } } catch (e) { /* the fallback below */ }
    const hr = /orbitrap|exploris|fusion|q.?exactive|tof|ft-?icr|astral/i.test(String((f && f.instrument) || ""));
    return hr ? { hr: true, v: 10, unit: "ppm" } : { hr: false, v: 0.5, unit: "Da" };
  }
  function queryOf(/** @type {Bag} */ p) {
    const a = p._a, d0 = a && a.data && a.data[0]; if (!d0 || !d0.d) return null;
    const d = d0.d, y = d.y0 || d.y, f = d0.f, peaks = d.mz.map((/** @type {number} */ m, /** @type {number} */ i) => [m, y[i]]).filter((/** @type {number[]} */ q) => q[1] > 0);
    const prec = [d.prec, p._prec, p.prec, d.scan && d.scan.prec].map(Number).find(v => Number.isFinite(v) && v > 0);
    const pol = f && f.polarity === "positive" ? 1 : f && f.polarity === "negative" ? -1 : 0;
    const rt = p.r0 != null ? (p.r0 + p.r1) / 2 : null;
    return { peaks, prec, pol, f, rt, title: `${(f && f.label) || ""}${rt != null ? " · RT " + rt.toFixed(2) + " min" : ""}` };
  }

  // ------------------------------------------------------------ the results window
  function mirror(/** @type {HTMLCanvasElement} */ cv, /** @type {Bag} */ q, /** @type {Bag} */ r) {
    // description and table: the experimental peaks above, the library ones below
    if (window.TDATI) {
      cv.setAttribute("aria-label", I18N.t("lr.mirror.aria")); TDATI.describe(cv, I18N.t("lr.mirror.desc", { n: q.mz.length, m: r.mz.length, name: r.name || "" }));
      TDATI.addButton(cv, () => ({ title: I18N.t("lr.mirror.aria"), head: [I18N.t("lr.mirror.col.src"), "m/z", I18N.t("tdati.col.rel")], rowHeader: true,
        rows: [...q.mz.map((/** @type {number} */ m, /** @type {number} */ i) => [I18N.t("lr.mirror.exp"), TDATI.num(m, 4), TDATI.num(q.it[i], 4)]), ...r.mz.map((/** @type {number} */ m, /** @type {number} */ i) => [I18N.t("lr.mirror.lib"), TDATI.num(m, 4), TDATI.num(r.it[i], 4)])] }));
    }
    const s = setup(cv), g = s.g, W = s.W, H = s.H, ml = 56, mr = 14, mt = 14, mb = 34, mid = mt + (H - mt - mb) / 2, half = (H - mt - mb) / 2 - 6;
    const all = [...q.mz, ...r.mz], lo = Math.floor(minOf(all) / 10) * 10 - 5, hi = Math.ceil(maxOf(all, r.prec) / 10) * 10 + 5, X = (/** @type {number} */ m) => ml + (m - lo) / (hi - lo) * (W - ml - mr);
    const acc = css("--accent"), ink = css("--ink"), mut = css("--muted"), line = css("--line"), shc = css("--ok");
    g.clearRect(0, 0, W, H); g.fillStyle = css("--panel"); g.fillRect(0, 0, W, H);
    const common = { q: new Set(r.pairs.map((/** @type {Bag} */ p) => p[0])), l: new Set(r.pairs.map((/** @type {Bag} */ p) => p[1])) };
    // lines between the paired peaks (type 0 = same m/z, 1 = shifted by the precursor difference) and between peaks that share a neutral loss without sharing the m/z; what is drawn is recorded in cv.dataset
    const nlp = [], nlq = new Set(), nll = new Set(), /** @type {any[][]} */ lines = [], tolNl = q.frag || 0.01, qy = (/** @type {number} */ i) => mid - q.it[i] * half, ly = (/** @type {number} */ j) => mid + r.it[j] * half, warn = css("--warn");
    if (q.prec > 0 && r.prec > 0) {
      const used = new Set((r.pairs || []).map((/** @type {Bag} */ p) => p[0] + "|" + p[1]));
      for (let i = 0; i < q.mz.length; i++) for (let j = 0; j < r.mz.length; j++) {
        const a = q.prec - q.mz[i], b = r.prec - r.mz[j];
        if (a > 0.5 && b > 0.5 && Math.abs(a - b) <= tolNl && !used.has(i + "|" + j) && !nlq.has(i) && !nll.has(j)) { nlp.push([i, j]); nlq.add(i); nll.add(j); }
      }
    }
    g.save(); g.globalAlpha = 0.55; g.lineWidth = 1;
    (r.pairs || []).forEach((/** @type {any[]} */ pr) => { const t = pr[2] ? 1 : 0; lines.push([pr[0], pr[1], t]); g.beginPath(); g.setLineDash(t ? [5, 3] : []); g.strokeStyle = t ? shc : acc; g.moveTo(X(q.mz[pr[0]]), qy(pr[0])); g.lineTo(X(r.mz[pr[1]]), ly(pr[1])); g.stroke(); });
    nlp.forEach(pr => { g.beginPath(); g.setLineDash([1, 3]); g.strokeStyle = warn; g.lineWidth = 1.5; g.moveTo(X(q.mz[pr[0]]), qy(pr[0])); g.lineTo(X(r.mz[pr[1]]), ly(pr[1])); g.stroke(); });
    g.restore(); cv.dataset.lines = JSON.stringify(lines); cv.dataset.nl = JSON.stringify(nlp);
    g.lineWidth = 1.5; g.font = fpx(12);
    // analogues: peaks matched after the shift
    const shq = new Set((r.pairs || []).filter((/** @type {Bag} */ p) => p[2]).map((/** @type {Bag} */ p) => p[0])), shl = new Set((r.pairs || []).filter((/** @type {Bag} */ p) => p[2]).map((/** @type {Bag} */ p) => p[1]));
    const bars = (/** @type {number[]} */ mz, /** @type {number[]} */ it, /** @type {Set<number>} */ set, /** @type {number} */ dir) => mz.forEach((/** @type {number} */ m, /** @type {number} */ i) => {
      const hh = it[i] * half, sh = (dir > 0 ? shq : shl).has(i), nl = (dir > 0 ? nlq : nll).has(i); g.strokeStyle = sh ? shc : nl ? warn : set.has(i) ? acc : mut; g.lineWidth = set.has(i) ? 2 : 1.3;
      g.beginPath(); g.moveTo(X(m), mid); g.lineTo(X(m), mid - dir * hh); g.stroke();
      if (it[i] > 0.08) { g.fillStyle = sh ? shc : nl ? warn : set.has(i) ? acc : mut; g.textAlign = "center"; g.fillText(m.toFixed(r.prec > 0 && q.hr ? 4 : 2), X(m), dir > 0 ? mid - hh - 4 : mid + hh + 13); }
    });
    bars(q.mz, q.it, common.q, 1); bars(r.mz, r.it, common.l, -1);
    g.strokeStyle = line; g.lineWidth = 1; g.beginPath(); g.moveTo(ml, mid); g.lineTo(W - mr, mid); g.stroke();
    g.fillStyle = mut; g.textAlign = "center"; g.beginPath(); g.strokeStyle = mut; g.moveTo(ml, H - mb + 6); g.lineTo(W - mr, H - mb + 6); g.stroke();
    const step = [1, 2, 5, 10, 20, 25, 50, 100, 200].find(v => (hi - lo) / v <= 9) || 500;
    for (let v = Math.ceil(lo / step) * step; v <= hi; v += step) { g.beginPath(); g.moveTo(X(v), H - mb + 6); g.lineTo(X(v), H - mb + 10); g.stroke(); g.fillText(String(v), X(v), H - mb + 22); }
    g.fillStyle = ink; g.textAlign = "left"; g.fillText(I18N.t("lib.plot.exp"), ml + 4, mt + 10); g.fillText(r.name || I18N.t("lib.plot.lib"), ml + 4, H - mb - 6);
    g.save(); g.fillStyle = mut; g.textAlign = "center"; g.translate(14, mid); g.rotate(-Math.PI / 2); g.fillText(I18N.t("lib.plot.rel"), 0, 0); g.restore();
    g.fillStyle = acc; g.textAlign = "right"; g.fillText(I18N.t("lib.plot.shared", { n: r.shared }), W - mr - 4, mt + 10);
    // geometry and peaks of the last drawing: the hover of libreria-finestra.js finds the peak under the pointer
    /** @type {Bag} */ (cv)._mg = { X, mid, half, q, r, shq, shl, nlq, nll };
  }
  // confidence levels: livelli.js (rules and thresholds in the data file livelli-1.js, the user's thresholds in qqq.lib.soglie). The candidates carry ik, ent, cos, shared, dprec (ppm), nlib
  const assess = (/** @type {Bag[]} */ cands, /** @type {Bag} */ o = {}) => LIVELLI.assess({ cands, analogs: o.analogs, nScans: o.nScans, std: o.std || null, formulaUnique: !!o.formulaUnique });
  // the voice of the menu exists only for high-resolution spectra (low resolution gives no answers)
  const isHr = (/** @type {Bag} */ p) => { const q = queryOf(p); return !!(q && profOf(q.f).hr); };
  // cosine on the visible peaks (cos_vis of Attributio): the reference peaks below the lower limit of the MS2 scan window are removed; null when nothing is cut
  function cosVis(/** @type {Bag} */ q, /** @type {Bag} */ r, /** @type {number} */ lo) {
    if (!(lo > 0) || !r.mz.some((/** @type {number} */ m) => m < lo)) return null;
    let dot = 0, na = 0, nb = 0; for (const v of q.it) na += v;
    r.mz.forEach((/** @type {number} */ m, /** @type {number} */ j) => { if (m >= lo) nb += r.it[j]; });
    for (const pr of r.pairs || []) if (r.mz[pr[1]] >= lo) dot += Math.sqrt(q.it[pr[0]] * r.it[pr[1]]);
    return na > 0 && nb > 0 ? dot / Math.sqrt(na * nb) : 0;
  }
  // «Identify all the MS2»: the scans of one precursor (10 ppm) and one peak (RT within 0.2 min) are grouped
  function groupRows(/** @type {Bag[]} */ rows, ppm = 10, dRt = 0.2) {
    const R = rows.filter((/** @type {Bag} */ r) => r.prec).sort((/** @type {Bag} */ a, /** @type {Bag} */ b) => a.prec - b.prec), byPrec = [];
    for (const r of R) { const g = byPrec[byPrec.length - 1]; if (g && Math.abs(r.prec - g[g.length - 1].prec) <= r.prec * ppm * 1e-6) g.push(r); else byPrec.push([r]); }
    /** @type {Bag[]} */ const out = [];
    for (const g of byPrec) {
      g.sort((a, b) => a.rt - b.rt); let cur = [g[0]];
      const flush = () => {
        const named = cur.filter(r => r.name && r.ent != null), best = (named.length ? named : cur).reduce((x, y) => ((y.ent ?? -1) > (x.ent ?? -1) ? y : x));
        out.push({ rt: best.rt, prec: cur.reduce((x, r) => x + r.prec, 0) / cur.length, n: cur.length, best, rows: cur });
      };
      for (let i = 1; i < g.length; i++) { if (g[i].rt - g[i - 1].rt <= dRt) cur.push(g[i]); else { flush(); cur = [g[i]]; } }
      flush();
    }
    return out.sort((a, b) => a.rt - b.rt);
  }
  // the level of a group of scans of one precursor: its best candidate, the best one of ANOTHER structure (`alt`, from the worker) and the number of scans
  function groupAssess(/** @type {Bag} */ g) {
    const b = g.best && g.best.name ? g.best : null;
    return assess(b ? [b, b.alt].filter(Boolean) : [], { nScans: g.n });
  }
  // the body loads once, on the first member that needs it; the worker is created later still, on the first call
  /** @type {Promise<Bag> | null} */ let loading = null;
  const body = () => loading || (loading = carica("libreria-corpo.js").then(() => /** @type {Bag} */ (window).LIBCORE({ queryOf, profOf, mirror, whatIs })).catch((/** @type {unknown} */ e) => { loading = null; throw e; }));
  /** @type {Bag} */ let core = /** @type {any} */ (null);
  const fwd = (/** @type {string} */ name) => async (/** @type {unknown[]} */ ...a) => { core = core || await body(); return core[name](...a); };
  const searchFrom = fwd("searchFrom");
  // «What is it?» and «Identify all the MS2»: the master/detail window of libreria-finestra.js, loaded on first use
  const win = async () => { await carica("m:libreria-finestra.js"); return /** @type {Bag} */ (window).LIBWIN; };
  const whatIs = async (/** @type {Pannello} */ p) => (await win()).whatIs(p);
  const identifyAll = async (/** @type {number} */ k) => (await win()).identifyAll(k);
  // the action of the spectrum menu (registry azioni/1): «What is it?» (high resolution) or «Search the libraries» of an MS2 spectrum
  if (typeof AZIONI !== "undefined") AZIONI.registra({ id: "spec.lib", gruppo: "lib", ordine: 1, viste: ["spec"], visibile: (/** @type {Bag} */ c) => c.p.level === 2,
    nome: (/** @type {Bag} */ c) => I18N.t(isHr(c.p) ? "ctx.libWhat" : "ctx.libSearch"), esegui: (/** @type {Bag} */ c) => isHr(c.p) ? whatIs(c.p) : searchFrom(c.p) });
  // the chromatogram menu: the MS2 spectra of a high-resolution file against the libraries
  if (typeof AZIONI !== "undefined") AZIONI.registra({ id: "chrom.identAll", gruppo: "lib", viste: ["chrom"], nome: () => I18N.t("ctx.chrom.identAll"),
    visibile: (/** @type {Bag} */ c) => !!window.HR && HR.isHr(E.files[c.k], 2) && E.files.some((/** @type {Bag} */ x) => x.file.split("#")[0] === E.files[c.k].file.split("#")[0] && x.kind === ESP.ms2), esegui: (/** @type {Bag} */ c) => identifyAll(c.k) });
  // «MS2 at this RT…» on a total trace of a DDA file: the MS2 scan nearest in time goes to the MS2 panel (hidden when the file has no MS2)
  if (typeof AZIONI !== "undefined") AZIONI.registra({ id: "chrom.ms2AtRt", gruppo: "extract", ordine: 2, viste: ["chrom"], voci: (/** @type {Bag} */ c) => {
    const f = E.files[c.k]; if (!f || !f.dda || !window.DDA || !DDA.ms2Item(f) || c.x == null) return [];
    return [{ nome: I18N.t("ctx.chrom.ms2AtRt"), tip: I18N.t("ctx.chrom.ms2AtRt.tip"), esegui: async () => {
      DDA.ensure(c.p); const s1 = E.panels.find((/** @type {Bag} */ q) => q.type === "spec" && q.link === c.p.id && q.el), s2 = s1 && E.panels.find((/** @type {Bag} */ q) => q.id === s1.duo && q.el); if (!s2) return;
      const D = await DDA.get(f.k), i = DDA.nearest(D.rt, c.x); if (i < 0) return;
      await DDA.selectMs2(s2, D.sid[i]); s2.el.scrollIntoView({ block: "nearest", behavior: "smooth" });
    } }];
  } });
  let urlJs = false; try { urlJs = new URLSearchParams(location.search).get("libjs") === "1"; } catch (e) { /* no location */ }
  // for the tests: which engine answers, and a way to kill it (the flag `?libjs=1` is known before the body)
  const engine = { info: async () => { core = core || await body(); return core.engine.info(); }, crash: async () => { core = core || await body(); return core.engine.crash(); }, get fallback() { return core ? core.engine.fallback : urlJs; } };
  const ensureCss = (/** @type {unknown[]} */ ...a) => { if (core) return core.ensureCss(...a); return body().then((/** @type {Bag} */ c) => { core = c; return c.ensureCss(...a); }); };
  return { open: fwd("open"), addFiles: fwd("addFiles"), queryOf, profOf, searchFrom, whatIs, isHr, mirror, cosVis, call: fwd("call"), identifyAll, groupRows, groupAssess, assess, scansOf: fwd("scansOf"), engine, ensureCss };
})();
window.LIB = LIB;
