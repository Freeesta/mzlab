/* mzLab - Pratica: data and generator of the MSn game ("lc-ms2", H7 "Quale MSn fare"; classic script, also loadable by node).
   ALL DATA ARE SYNTHETIC: a constructed molecule (generic functional groups, invented formula), a constructed MS2 spectrum and a constructed MS3.
   Declared model: every functional group loses its neutral once; an MS2 ion is a precursor that lost one group (or two, the second-generation ions);
   the MS3 of a one-loss ion shows one loss per group still present (a declared subset of the plausible MS2 pathways); an MS3 peak is "new" only
   if its m/z is not already a peak of the MS2 spectrum; ions below MS3_MIN (percent of the base peak) give an MS3 too weak to be useful.
   Masses come from ELEMENTS through PAL. Generator = pure function of (level, seed). The program never says which ion to isolate. */
"use strict";
const LCMSNDATA = (() => {
  const P = typeof PAL !== "undefined" ? PAL : require("./motore.js");
  const ELECTRON = 0.000548579909;
  const MS3_MIN = 10;                                       // % of the base peak
  // group -> neutral lost, name, atoms the group adds to the invented skeleton
  const GROUPS = {
    NH2: { loss: "NH3", name: "ammina primaria", add: { N: 1, H: 2 } },
    OH: { loss: "H2O", name: "idrossile alcolico", add: { O: 1, H: 1 } },
    CO: { loss: "CO", name: "carbonile", add: { C: 1, O: 1 } },
    Et: { loss: "C2H4", name: "gruppo etile", add: { C: 2, H: 5 } },
    Cl: { loss: "HCl", name: "cloro su carbonio", add: { Cl: 1 } },
    OMe: { loss: "CH3OH", name: "estere metilico", add: { C: 2, H: 3, O: 2 } },
    COOH: { loss: "HCOOH", name: "acido carbossilico", add: { C: 1, H: 1, O: 2 } },
  };
  const LOSS_MASS = Object.fromEntries(Object.entries(GROUPS).map(([g, v]) => [g, P.exact(P.parse(v.loss))]));
  const LOSS_NOM = Object.fromEntries(Object.entries(LOSS_MASS).map(([g, m]) => [g, Math.round(m)]));
  const LOSS_LIST = Object.values(GROUPS).map(v => v.loss);
  const sum = (a, b) => { const o = { ...a }; Object.entries(b).forEach(([k, n]) => { o[k] = (o[k] || 0) + n; }); return o; };
  const sub = (a, b) => { const o = { ...a }; Object.entries(b).forEach(([k, n]) => { o[k] = (o[k] || 0) - n; }); return o; };
  const valid = f => Object.values(f).every(n => n >= 0);
  /** all losses (one group, two groups) must have distinct nominal masses: otherwise the unit-resolution spectrum is ambiguous */
  const distinctNominal = gs => {
    const v = gs.map(g => LOSS_NOM[g]), all = v.concat(v.flatMap((a, i) => v.slice(i + 1).map(b => a + b)));
    return new Set(all).size === all.length;
  };
  /** an invented molecule that contains every group; every one- and two-group loss from [M+H]+ must be a valid formula */
  function skeleton(r, gs) {
    for (let k = 0; k < 100; k++) {
      let f = { C: 6 + Math.floor(r() * 7) };
      gs.forEach(g => { f = sum(f, GROUPS[g].add); });
      const dbe = 4 + Math.floor(r() * 3);
      f.H = 2 * f.C + 2 + (f.N || 0) - (f.Cl || 0) - 2 * dbe;
      if (f.H < 6) continue;
      const ion = { ...f, H: f.H + 1 };
      const ok = gs.every(g => valid(sub(ion, P.parse(GROUPS[g].loss)))) && gs.every((g, i) => gs.slice(i + 1).every(h => valid(sub(sub(ion, P.parse(GROUPS[g].loss)), P.parse(GROUPS[h].loss)))));
      if (ok) return f;
    }
    return null;
  }
  return { P, ELECTRON, MS3_MIN, GROUPS, LOSS_MASS, LOSS_NOM, LOSS_LIST, distinctNominal, skeleton };
})();
const LCMSN = (() => {
  const { P, ELECTRON, MS3_MIN, GROUPS, LOSS_MASS, distinctNominal, skeleton } = LCMSNDATA;
  const r1 = v => Math.round(v * 10) / 10;
  const fx = (v, d) => v.toFixed(d).replace(".", ",");
  const ri = (r, a, b) => a + Math.floor(r() * (b - a + 1));

  /** abundances (percent of the base peak): index 0 is the base peak; with `low` the last one is below MS3_MIN */
  function abundances(r, n, low) {
    const out = [100]; const pool = [];
    for (let v = 18; v <= 78; v += 6) pool.push(v + ri(r, 0, 3));
    P.shuffle(pool, r).slice(0, n - 1 - (low ? 1 : 0)).forEach(v => out.push(v));
    if (low) out.push(ri(r, 3, 8));
    return out;
  }
  /** the MS2 ions: one-loss ions (the groups) plus the observed two-loss ions; ids follow the m/z */
  function buildIons(prec, gs, ab, pairs, r) {
    const ions = gs.map((g, i) => ({ kind: "single", groups: [g], mz: r1(prec - LOSS_MASS[g]), abund: ab[i] }));
    pairs.forEach(([a, b]) => ions.push({ kind: "double", groups: [gs[a], gs[b]], mz: r1(prec - LOSS_MASS[gs[a]] - LOSS_MASS[gs[b]]), abund: ri(r, 12, 30) }));
    ions.sort((x, y) => x.mz - y.mz).forEach((x, i) => { x.id = "M" + (i + 1); });
    return ions;
  }
  const seen = (ions, g, h) => ions.some(x => x.kind === "double" && x.groups.includes(g) && x.groups.includes(h));
  /** the MS3 of an ion: one daughter per group still present; "novel" = the double loss is not a peak of the MS2 spectrum */
  function ms3Of(e, ion) {
    if (ion.kind !== "single") return [];
    return e.groups.filter(h => h !== ion.groups[0]).map(h => ({ group: h, loss: GROUPS[h].loss, mz: r1(ion.mz - LOSS_MASS[h]), novel: !seen(e.ions, ion.groups[0], h) }));
  }
  const infoOf = (e, ion) => ms3Of(e, ion).filter(d => d.novel).length;
  const feasible = ion => ion.kind === "single" && ion.abund >= MS3_MIN;

  function genMsn(level, seed) {
    const G = level === 1 ? 4 : 5;
    for (let attempt = 0; attempt < 300; attempt++) {
      const r = P.rng((seed * 48271 + attempt * 2654435761) >>> 0);
      const gs = P.shuffle(Object.keys(GROUPS), r).slice(0, G);
      if (!distinctNominal(gs)) continue;
      const f = skeleton(r, gs); if (!f) continue;
      const ion = { ...f, H: f.H + 1 }, prec = P.exact(ion) - ELECTRON, low = level >= 2;
      const ab = abundances(r, G, low);                     // gs[0] = base peak (hub), gs[G-1] = weak ion when low
      let pairs;
      if (level === 1) { const x = ri(r, 1, G - 1), y = ri(r, 1, G - 1); if (x === y) continue; pairs = [[x, y]]; }
      else pairs = (level === 2 ? [1, 2] : [1, 2, 3]).map(i => [0, i]);
      const e = { game: "lc-ms2", level, seed, groups: gs, formula: f, fs: P.fstr(f), precMz: r1(prec), ions: buildIons(prec, gs, ab, pairs, r), threshold: MS3_MIN };
      e.probeGroup = level === 1 ? gs[pairs[0][0]] : gs[0];
      const singles = e.ions.filter(x => x.kind === "single"), best = Math.max(...singles.filter(feasible).map(x => infoOf(e, x)));
      e.bestIds = singles.filter(x => feasible(x) && infoOf(e, x) === best).map(x => x.id);
      e.probeId = singles.find(x => x.groups[0] === e.probeGroup).id;
      if (!e.bestIds.length || e.bestIds.length === singles.length) continue;
      if (level >= 2 && e.bestIds.includes(singles.find(x => x.abund === 100).id)) continue;
      if (new Set(e.ions.map(x => x.mz)).size !== e.ions.length) continue;
      e.id = `MS${level}-${e.fs}-${gs.join("")}`; e.d0 = level === 1 ? 0.3 : level === 2 ? 0.8 : 1.3;
      e.help = help(e);
      return e;
    }
    throw new Error("genMsn: no exercise for seed " + seed);
  }
  function help(e) {
    return [
      `Parti dalle differenze: ogni picco dista dal precursore (m/z ${fx(e.precMz, 1)}) di una perdita neutra. Le perdite comuni sono NH<sub>3</sub> 17, H<sub>2</sub>O 18, CO e C<sub>2</sub>H<sub>4</sub> 28, CH<sub>3</sub>OH 32, HCl 36, HCOOH 46. Un picco che dista di due perdite insieme è un ione di seconda generazione.`,
      `Nei dati costruiti ogni gruppo funzionale perde il suo neutro una volta sola, e l'MS<sup>3</sup> di uno ione a perdita singola mostra le perdite dei gruppi che ha ancora. Una perdita dà un'informazione nuova solo se il picco che produce non compare già nello spettro MS<sup>2</sup>. Sotto il ${MS3_MIN} % del picco base l'MS<sup>3</sup> è troppo debole.`,
      `Zona. Per ogni ione a perdita singola contate i gruppi rimasti (tutti tranne quello già perso), togliete quelli per cui lo ione a doppia perdita è già nell'MS<sup>2</sup>, e scartate gli ioni sotto la soglia di abbondanza. Poi confrontate i conteggi: l'abbondanza da sola non decide.`,
    ];
  }
  /** numbers shown after an attempt: for every one-loss ion the remaining groups, the already seen double losses, the novel peaks, the feasibility */
  function coherence(e) {
    return e.ions.filter(x => x.kind === "single").map(x => ({ id: x.id, mz: x.mz, abund: x.abund, remaining: e.groups.length - 1, seen: e.groups.length - 1 - infoOf(e, x), novel: infoOf(e, x), feasible: feasible(x) }));
  }
  return { MS3_MIN, GROUPS, genMsn, ms3Of, infoOf, feasible, coherence, fx };
})();
if (typeof module !== "undefined") module.exports = LCMSN;
