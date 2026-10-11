// TPMINE-PRIVATE  Untargeted match: every feature with an MS2 goes through the library search (WebAssembly engine in libreria-worker.js, LIB.call("searchAll")) and the level module (LIVELLI);
// a feature without MS2 stays at 4 or 5. The programme NEVER assigns 2b: only the chemist's «decided level» (LIVELLI.decide). The external annotations of the tables (mzMine, GNPS2, MS-DIAL) are shown as
// evidence with their source and score and never become a level in the profile «mzlab». Pure but for LIB/LIVELLI/UNT, which are globals of the page (node tests inject them).
(() => {
  "use strict";
  const G = typeof window !== "undefined" ? window : globalThis;
  const BATCH = 1000, MIN_TOL_PPM = 1;

  /** a formula given by an external source that explains the m/z of the feature (5 ppm) and is the only one → level 4 («unique formula»); never invented here */
  function formulaOf(f, tolPpm = 5) {
    const set = new Map();
    for (const a of f.ann || []) {
      if (!a.formula || a.structure === "unreliable") continue;
      const c = G.UNT.checkStructure(a.formula, f.mz, a.adduct, f.pol, tolPpm);
      if (c.ok === true) set.set(a.formula, a.source);
    }
    return set.size === 1 ? { formula: [...set.keys()][0], source: [...set.values()][0] } : null;
  }
  const extLine = a => {
    const sc = a.score == null ? "" : a.type === "MQScore" || a.type === "cosine" ? ` ${a.type} ${a.score.toFixed(2)}` : ` ${a.type} ${a.score}`, sh = a.shared == null ? "" : `, ${a.shared} shared`;
    return `${a.source}: ${a.cleanName || a.name}${sc}${sh}${a.analog ? " (analogue)" : ""}`;
  };
  const externals = f => (f.ann || []).filter(a => !a.propagated);

  /** result of ONE feature from the row of the search (best candidate + `alt`, the best one of another structure) */
  function assessRow(f, row, opt = {}) {
    const L = G.LIVELLI, hasMs2 = !!(f.ms2 && f.ms2.mz.length), nScans = f.nScans || 1, fu = formulaOf(f);
    const best = row && row.name ? row : null, cands = best ? [best, best.alt].filter(Boolean) : [];
    const lv = L.assess({ cands, analogs: opt.analogs, nScans, std: null, formulaUnique: !!fu }, opt.profile);
    const ext = externals(f).map(extLine);
    const ev = (lv.lines || []).concat(lv.decisive ? [lv.decisive] : []);
    if (fu && lv.proposed === "4") ev.push(`${fu.source}: formula ${fu.formula} explains the m/z within 5 ppm`);
    return { key: f.key, lv, level: lv.proposed, decided: null, hasMs2, nScans, best, ent: best ? best.ent : null, cos: best ? best.cos : null, shared: best ? best.shared : null, dppm: best ? best.dprec : null,
      name: best ? best.name : "", lib: best ? best.lib : "", ik: best ? best.ik : "", formula: best ? best.formula : fu ? fu.formula : "", alt: best && best.alt ? best.alt.name : "", evidence: ev, external: ext };
  }
  const shown = r => G.LIVELLI.shown(r.lv);
  function decide(r, lv) { const o = G.LIVELLI.decide(r.lv, lv); r.decided = r.lv.decided; return o; }

  /** table → Map key → result. opt: {tolPpm, frag, profile, onProgress(i, n), isCancelled()} */
  async function run(table, opt = {}) {
    const tol = Math.max(MIN_TOL_PPM, +opt.tolPpm || 10), frag = opt.frag || 0.01, out = new Map(), queries = [], idx = [];
    table.features.forEach((f, i) => {
      if (f.ms2 && f.ms2.mz.length && f.mz > 0) { queries.push({ key: i, rt: f.rt, prec: f.ms2.prec > 0 ? f.ms2.prec : f.mz, pol: f.pol || table.pol || 0, peaks: f.ms2.mz.map((m, j) => [m, f.ms2.it[j]]) }); idx.push(i); }
    });
    const rows = new Map(); let done = 0, t0 = Date.now();
    for (let a = 0; a < queries.length; a += BATCH) {
      if (opt.isCancelled && opt.isCancelled()) break;
      const part = queries.slice(a, a + BATCH);
      const r = await G.LIB.call("searchAll", { queries: part, tol: { v: tol, unit: "ppm" }, frag }, m => opt.onProgress && opt.onProgress(a + (m.read || 0), queries.length));
      for (const row of r.rows) rows.set(row.key, row);
      done += part.length; if (opt.onProgress) opt.onProgress(done, queries.length);
    }
    for (let i = 0; i < table.features.length; i++) { const f = table.features[i]; out.set(f.key, assessRow(f, rows.get(i) || null, opt)); }
    out.stats = { queries: queries.length, ms: Date.now() - t0 };
    return out;
  }

  /** the pseudo panel read by LIB.queryOf / LIB.whatIs (a DDA panel in the Data tab has the same shape) */
  function panelOf(f, table) {
    const ms2 = f.ms2, top = ms2 ? Math.max(...ms2.it) || 1 : 1;
    return { _a: { data: [{ d: { mz: ms2.mz, y: ms2.it.map(v => v / top), prec: ms2.prec > 0 ? ms2.prec : f.mz }, f: { polarity: (f.pol || table.pol) > 0 ? "positive" : (f.pol || table.pol) < 0 ? "negative" : "", label: `${table.file.name || "table"} · feature ${f.id}`, instrument: "Orbitrap" } }] },
      r0: f.rt, r1: f.rt, prec: ms2.prec > 0 ? ms2.prec : f.mz, avg: f.nScans > 1 ? new Array(f.nScans).fill(0) : null };
  }

  G.UNTM = { run, assessRow, shown, decide, formulaOf, panelOf, externals, extLine };
  if (typeof module !== "undefined") module.exports = G.UNTM;
})();
