/* mzLab - Pratica: data and generators of the LC-MS exercises (games "lc-dm" and "lc-esi"; classic script, also loadable by node).
   ALL DATA ARE CONSTRUCTED FOR TEACHING: generic scaffolds with invented formulas, never real compounds of a laboratory experience.
   - H1 "dm" (delta m of a transformation product): precursor [M+H]+ and product m/z -> which transformation (or pair of transformations)?
   - H2 "esi" (ESI+ and ESI-): lists of m/z of generic compounds -> neutral mass M, nitrogen rule, pairing of + and - signals, adducts.
   Every mass is computed from ELEMENTS through PAL (motore.js): nothing is copied from a table. Every generator is a pure function of
   (level, seed): the same seed gives the same exercise (PAL.rng). The page (pratica-tp.html) only shows these objects. */
"use strict";
const LCMS = (() => {
  const P = typeof PAL !== "undefined" ? PAL : require("./motore.js");
  const sub = s => String(s).replace(/(\d+)/g, "<sub>$1</sub>");
  const ex = d => P.exact(d);                               // exact monoisotopic mass of a (signed) element count
  const ELECTRON = 0.000548579909;
  const mH = ex({ H: 1 });
  // ---------------------------------------------------------------- adducts: shift of m/z from the neutral mass M
  const SHIFT = {
    "[M+H]+": mH - ELECTRON,
    "[M+NH4]+": ex({ N: 1, H: 4 }) - ELECTRON,
    "[M+Na]+": ex({ Na: 1 }) - ELECTRON,
    "[M-H]-": -mH + ELECTRON,
    "[M+HCOO]-": ex({ C: 1, H: 1, O: 2 }) + ELECTRON,
  };
  const ADDUCT_LABEL = { "[M+H]+": "[M+H]<sup>+</sup>", "[M+NH4]+": "[M+NH<sub>4</sub>]<sup>+</sup>", "[M+Na]+": "[M+Na]<sup>+</sup>", "[M-H]-": "[M−H]<sup>−</sup>", "[M+HCOO]-": "[M+HCOO]<sup>−</sup>" };
  const C13 = 13.00335483507 - 12;                          // 13C - 12C: the isotopic trap of NH2 -> OH

  // ---------------------------------------------------------------- scaffolds (invented formulas; groups = what a transformation needs)
  // Me: aryl or alkyl CH3; NMe: N- or O-methyl; Et: N- or O-ethyl; NH2: primary amine; Cl: chlorine on carbon; COOH: carboxylic acid; OH: alcohol; CO: carbonyl
  const SCAFFOLDS = [
    { id: "S01", f: "C10H13NO2", desc: "estere aromatico con un metile", groups: ["Me", "CO"] },
    { id: "S02", f: "C9H12N2O", desc: "ammide di un'anilina primaria", groups: ["Me", "NH2", "CO"] },
    { id: "S03", f: "C13H19ClN2O", desc: "ammide clorurata con una ammina terziaria", groups: ["Me", "NMe", "Cl", "CO"] },
    { id: "S04", f: "C11H14Cl2O2", desc: "estere di un acido aromatico diclorurato", groups: ["Me", "Cl", "Et", "CO"] },
    { id: "S05", f: "C14H21NO3", desc: "ammina terziaria con un etere", groups: ["Me", "NMe", "Et"] },
    { id: "S06", f: "C8H11N3O2S", desc: "eterociclo azotato con un gruppo solforato", groups: ["Me", "NH2"] },
    { id: "S07", f: "C15H17ClN2O2", desc: "diarile clorurato con due azoti", groups: ["Me", "Cl", "NMe", "CO"] },
    { id: "S08", f: "C10H11NO3", desc: "acido aromatico con un'ammide", groups: ["Me", "COOH", "CO", "OH"] },
    { id: "S09", f: "C12H16N4O2", desc: "eterociclo con quattro azoti", groups: ["Me", "NH2", "NMe", "CO"] },
    { id: "S10", f: "C9H10ClNO", desc: "ammide aromatica monoclorurata", groups: ["Me", "Cl", "CO"] },
    { id: "S11", f: "C16H25NO4", desc: "estere con una ammina secondaria", groups: ["Me", "Et", "CO", "OH"] },
    { id: "S12", f: "C13H12N2O3", desc: "fenolo con un'ammide", groups: ["Me", "NH2", "OH", "CO"] },
  ].map(s => ({ ...s, p: P.parse(s.f), nom: P.nominal(P.parse(s.f)), M: ex(P.parse(s.f)) }));

  // ---------------------------------------------------------------- transformations (delta = element counts, may be negative)
  const T = (id, label, delta, needs = []) => ({ id, label, delta, needs, dm: ex(delta), nom: Math.round(ex(delta)) });
  const TRANS = [
    T("O", "+O", { O: 1 }), T("O-2H", "+O−2H", { O: 1, H: -2 }), T("2O", "+2O", { O: 2 }), T("3O", "+3O", { O: 3 }),
    T("-2H", "−2H", { H: -2 }), T("+H2", "+H<sub>2</sub>", { H: 2 }), T("+H2O", "+H<sub>2</sub>O", { H: 2, O: 1 }), T("-H2O", "−H<sub>2</sub>O", { H: -2, O: -1 }, ["OH"]),
    T("+CH2", "+CH<sub>2</sub>", { C: 1, H: 2 }), T("-CH2", "−CH<sub>2</sub>", { C: -1, H: -2 }, ["NMe"]), T("-C2H4", "−C<sub>2</sub>H<sub>4</sub>", { C: -2, H: -4 }, ["Et"]),
    T("-CO", "−CO", { C: -1, O: -1 }, ["CO"]), T("-CO2", "−CO<sub>2</sub>", { C: -1, O: -2 }, ["COOH"]),
    T("CH3>COOH", "CH<sub>3</sub>→COOH", { H: -2, O: 2 }, ["Me"]), T("NH2>OH", "NH<sub>2</sub>→OH", { N: -1, H: -1, O: 1 }, ["NH2"]),
    T("-Cl+OH", "−Cl+OH", { Cl: -1, O: 1, H: 1 }, ["Cl"]), T("-Cl+H", "−Cl+H", { Cl: -1, H: 1 }, ["Cl"]),
  ];
  const ISO13C = { id: "13C", label: "picco isotopico <sup>13</sup>C (non è una trasformazione)", delta: null, dm: C13, nom: 1, iso: true };

  const apply = (f, d) => { const o = { ...f }; for (const e in d) { o[e] = (o[e] || 0) + d[e]; if (o[e] < 0) return null; if (!o[e]) delete o[e]; } return o; };
  const valid = f => !!f && Number.isInteger(P.rdb(f)) && P.rdb(f) >= 0;       // DBE of a real structure: integer and not negative
  const sumDelta = ts => { const d = {}; ts.forEach(t => { for (const e in t.delta) d[e] = (d[e] || 0) + t.delta[e]; }); return d; };
  const key = d => Object.keys(d).sort().filter(e => d[e]).map(e => e + d[e]).join(",");
  const applicable = (sc, t) => t.needs.every(g => sc.groups.includes(g));
  const mzOf = (M, add) => M + SHIFT[add];
  const fx = (v, d) => v.toFixed(d).replace(".", ",");
  const sgn = (v, d) => (v >= 0 ? "+" : "−") + fx(Math.abs(v), d);
  const err = (r, ppm) => (2 * r() - 1) * ppm;                                 // uniform in [-ppm, +ppm]
  const noisy = (mz, r, ppm) => Math.round(mz * (1 + err(r, ppm) * 1e-6) * 1e4) / 1e4;
  const pick = (r, a) => a[Math.floor(r() * a.length)];
  const shuf = (r, a) => P.shuffle(a, r);

  // ================================================================ H1: delta m of a transformation product
  const LEVELS_DM = { 1: { ppm: 2, pairs: false, near: false }, 2: { ppm: 2, pairs: false, near: true }, 3: { ppm: 5, pairs: true, near: true } };

  /** candidate answers for a scaffold: lists of transformations, with the product formula valid and a unique delta */
  function candidates(sc, pairs) {
    const single = TRANS.filter(t => applicable(sc, t));
    const lists = single.map(t => [t]);
    if (pairs) { const simple = single.filter(t => t.id !== "O-2H");                  // +O−2H is already a pair of simple steps
      for (let i = 0; i < simple.length; i++) for (let j = i + 1; j < simple.length; j++) lists.push([simple[i], simple[j]]); }
    const seen = new Map();
    lists.forEach(l => {
      const d = sumDelta(l), k = key(d), prod = apply(sc.p, d);
      if (!k || !valid(prod)) return;
      if (!seen.has(k) || seen.get(k).length > l.length) seen.set(k, l);        // same delta from two routes: keep the shortest
    });
    return [...seen.values()].map(l => { const d = sumDelta(l); return { ts: l, delta: d, dm: ex(d), id: l.map(t => t.id).join("+"), label: l.map(t => t.label).join(" e "), iso: false }; });
  }

  function genDm(level, seed) {
    const cfg = LEVELS_DM[level] || LEVELS_DM[1];
    for (let attempt = 0; attempt < 60; attempt++) {
      const r = P.rng((seed * 7919 + attempt * 104729) >>> 0);
      const sc = pick(r, SCAFFOLDS), all = candidates(sc, cfg.pairs);
      const pool = cfg.pairs ? all.filter(c => c.ts.length === 2) : all.filter(c => c.ts.length === 1);
      if (!pool.length) continue;
      const truth = pick(r, pool);
      const Mp = sc.M, mzP = Math.round(mzOf(Mp, "[M+H]+") * 1e4) / 1e4;
      const obs = noisy(mzOf(Mp + truth.dm, "[M+H]+"), r, cfg.ppm);
      const tolDa = cfg.ppm * obs / 1e6, minGap = 4 * tolDa;
      const others = all.filter(c => c !== truth && (cfg.pairs ? c.ts.length === 2 : c.ts.length === 1) && Math.abs(c.dm - truth.dm) >= minGap);
      const pool13 = cfg.near && Math.abs(C13 - truth.dm) >= minGap ? [ISO13C] : [];
      let dist = [];
      if (cfg.near) {                                                          // quasi isobaric traps first (within 0.1 u), then the rest
        const close = shuf(r, others.filter(c => Math.abs(c.dm - truth.dm) < 0.1)).slice(0, 2);
        dist.push(...close);
        if (pool13.length && Math.abs(truth.dm - C13) < 0.1) dist.push(ISO13C);
      }
      const gapL1 = cfg.near ? 0 : 0.5;                                        // level 1: well separated options
      const rest = shuf(r, others.filter(c => !dist.includes(c) && Math.abs(c.dm - truth.dm) >= gapL1));
      while (dist.length < 4 && rest.length) dist.push(rest.pop());
      if (dist.length < 4) continue;
      dist = dist.slice(0, 4);
      const options = shuf(r, [truth, ...dist]).map(o => ({ id: o.id, label: o.label, dm: o.dm, nom: Math.round(o.dm), iso: !!o.iso, delta: o.delta }));
      const prod = apply(sc.p, truth.delta);
      const dmObs = obs - mzP;
      const nomObs = Math.round(obs), nomPrec = Math.round(mzP);
      const ex_ = {
        game: "lc-dm", level, seed, ppm: cfg.ppm, tolMDa: tolDa * 1000, scaffold: sc.id, desc: sc.desc,
        precursor: { f: sc.f, formula: sc.p, mz: mzP, nominalMz: nomPrec },
        obs, dmObs, nomObs, options, truthId: truth.id, truthLabel: truth.label,
        product: { formula: prod, f: P.fstr(prod), dbe: P.rdb(prod), nN: prod.N || 0, nO: prod.O || 0, nParityOdd: (prod.N || 0) % 2 === 1 },
        id: `L${level}-${sc.id}-${truth.id}`, d0: level === 1 ? 0 : level === 2 ? 0.6 : 1.2,
      };
      ex_.help = helpDm(ex_);
      return ex_;
    }
    throw new Error("genDm: no exercise for seed " + seed);
  }

  function helpDm(e) {
    const nm = Math.round(e.dmObs);
    return [
      `Parti dalla differenza: m/z del candidato − m/z del precursore = ${fx(e.obs, 4)} − ${fx(e.precursor.mz, 4)} = ${sgn(e.dmObs, 4)} u. Quella è la massa esatta che la trasformazione ha aggiunto o tolto (le cariche sono uguali e si semplificano).`,
      `La differenza nominale è ${nm > 0 ? "+" : "−"}${Math.abs(nm)}: le opzioni compatibili devono avere la stessa massa intera. Tra quelle rimaste guardate i decimali: il segno del difetto di massa dipende dagli atomi coinvolti (l'ossigeno e il cloro lo abbassano, l'idrogeno lo alza).`,
      `Con questa risoluzione l'errore atteso è ±${e.ppm} ppm, cioè ±${fx(e.tolMDa, 1)} mDa a m/z ${fx(e.obs, 1)}. Calcolate, per ogni opzione, la differenza tra il suo Δm teorico e il Δm misurato: tenete solo quelle che stanno dentro l'errore e poi controllate che la formula del prodotto abbia DBE intero e non negativo.`,
    ];
  }

  /** Coherence of a choice (never says "right": gives the numbers). optId: id of the option chosen. */
  function coherenceDm(e, optId) {
    const o = e.options.find(x => x.id === optId);
    const diff = e.dmObs - o.dm, ppm = diff / e.obs * 1e6;
    let prod = null, dbe = null, nN = null;
    if (!o.iso) { prod = apply(e.precursor.formula, o.delta); if (prod) { dbe = P.rdb(prod); nN = prod.N || 0; } }
    return { option: o, dmTheory: o.dm, diffMDa: diff * 1000, ppm, within: Math.abs(diff) * 1000 <= e.tolMDa * 1.0001, iso: !!o.iso,
      productFormula: prod ? P.fstr(prod) : null, dbe, dbeOk: dbe !== null && Number.isInteger(dbe) && dbe >= 0, nN };
  }

  // ================================================================ H2: ESI+ and ESI-
  const LEVELS_ESI = { 1: { ppm: 3, n: 1 }, 2: { ppm: 3, n: 3 }, 3: { ppm: 5, n: 1 } };

  function genEsi(level, seed) {
    const cfg = LEVELS_ESI[level] || LEVELS_ESI[1];
    for (let attempt = 0; attempt < 60; attempt++) {
      const r = P.rng((seed * 6151 + attempt * 99991) >>> 0);
      const scs = shuf(r, SCAFFOLDS).slice(0, cfg.n);
      // different compounds must differ by more than 0.05 u in M, so that no pairing is ambiguous
      let bad = false;
      for (let i = 0; i < scs.length; i++) for (let j = i + 1; j < scs.length; j++) if (Math.abs(scs[i].M - scs[j].M) < 0.5) bad = true;
      if (bad) continue;
      const compounds = scs.map((s, i) => ({ id: "C" + (i + 1), scaffold: s.id, f: s.f, M: s.M, nN: s.p.N || 0, nOdd: (s.p.N || 0) % 2 === 1 }));
      const signals = [];
      compounds.forEach(c => {
        const adds = ["[M+H]+", "[M-H]-"];
        if (level === 3) adds.push(r() < 0.5 ? "[M+Na]+" : "[M+NH4]+", "[M+HCOO]-");
        adds.forEach(a => signals.push({ comp: c.id, adduct: a, mode: a.endsWith("+") ? "+" : "-", mz: noisy(mzOf(c.M, a), r, cfg.ppm) }));
      });
      const pos = signals.filter(s => s.mode === "+").sort((a, b) => a.mz - b.mz), neg = signals.filter(s => s.mode === "-").sort((a, b) => a.mz - b.mz);
      pos.forEach((s, i) => { s.id = "p" + (i + 1); }); neg.forEach((s, i) => { s.id = "n" + (i + 1); });
      const e = {
        game: "lc-esi", level, seed, ppm: cfg.ppm, compounds, pos, neg, id: `L${level}-${scs.map(s => s.id).join("")}`, d0: level === 1 ? 0 : level === 2 ? 0.6 : 1.2,
        task: level === 1 ? "M" : level === 2 ? "pair" : "adduct",
        adductChoices: Object.keys(SHIFT),
      };
      e.help = helpEsi(e);
      return e;
    }
    throw new Error("genEsi: no exercise for seed " + seed);
  }

  function helpEsi(e) {
    const hp = fx(SHIFT["[M+H]+"], 4), hm = fx(-SHIFT["[M-H]-"], 4);
    if (e.task === "M") return [
      `In ESI+ lo ione più comune è [M+H]<sup>+</sup>, in ESI− è [M−H]<sup>−</sup>: dallo stesso composto i due segnali distano circa due volte la massa del protone. Controllate che sia così per i due m/z dati.`,
      `Per ricavare M togliete ${hp} u dal segnale positivo (aggiungete ${hm} u a quello negativo); i due risultati devono coincidere entro l'errore. Poi guardate la massa nominale di M, non quella dello ione.`,
      `Regola dell'azoto, nella forma per gli ioni: [M+H]<sup>+</sup> e [M−H]<sup>−</sup> hanno massa nominale <i>pari</i> quando N è <i>dispari</i> (e viceversa), perché lo ione ha un'unità in più o in meno di M. Applicatela al m/z nominale di uno dei due segnali.`,
    ];
    if (e.task === "pair") return [
      `Due segnali dello stesso composto, uno positivo e uno negativo, dovrebbero distare circa ${fx(2 * SHIFT["[M+H]+"], 3)} u (due volte la massa del protone), se sono [M+H]<sup>+</sup> e [M−H]<sup>−</sup>.`,
      `Fate la differenza tra ogni m/z positivo e ogni m/z negativo: ne cercate una vicina a quel valore, con l'errore dato. Le altre combinazioni distano di molto di più o di molto di meno.`,
      `Per la parità di N, usate la regola per gli ioni: m/z nominale pari → N dispari; m/z nominale dispari → N pari. Vale sia per [M+H]<sup>+</sup> sia per [M−H]<sup>−</sup>, quindi i due segnali accoppiati devono dare la stessa risposta.`,
    ];
    return [
      `Nello stesso composto, due segnali dello stesso segno che distano 21,982 u (Na al posto di H) o 17,027 u (NH<sub>4</sub> al posto di H) sono lo ione protonato e il suo addotto.`,
      `Il segnale più leggero di ciascun segno è quasi sempre lo ione [M+H]<sup>+</sup> o [M−H]<sup>−</sup>. In negativo, due segnali distanti 46,005 u indicano il formiato: [M+HCOO]<sup>−</sup> = [M−H]<sup>−</sup> + 46,005.`,
      `Prima di rispondere, ricavate M da [M+H]<sup>+</sup> e da [M−H]<sup>−</sup> e verificate che coincidano; poi calcolate M anche dagli addotti scelti: se il valore non torna, l'attribuzione è sbagliata. Strumenti del programma: la tabella degli addotti e la calcolatrice.`,
    ];
  }

  /** nitrogen rule for the ions [M+H]+ and [M-H]-: even nominal m/z <-> odd number of N. Returns true if N is odd. */
  const nOddFromIon = mz => Math.round(mz) % 2 === 0;

  return { SHIFT, ADDUCT_LABEL, SCAFFOLDS, TRANS, ISO13C, C13, LEVELS_DM, LEVELS_ESI, candidates, apply, valid, genDm, coherenceDm, genEsi, nOddFromIon, sub, fx, sgn };
})();
if (typeof module !== "undefined") module.exports = LCMS;
