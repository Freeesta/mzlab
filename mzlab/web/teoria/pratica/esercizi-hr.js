/* mzLab - Pratica: data and generators of the HR levels of the formula page (game "hr-nt"; classic script, also loadable by node).
   ALL DATA ARE SYNTHETIC: formulas drawn at random for teaching, never a compound of a laboratory experience, never a measurement.
   - H6 "nt" (formula from a non-target HR ion): accurate [M+H]+ and M, M+1, M+2 -> candidate formulas within N ppm -> which fail which test
     (nitrogen rule / integer DBE, DBE >= 0, H/C, isotopes), which formula is kept.
   - H8 "fine" (fine isotopic structure): resolved peaks around M+1 and M+2 -> which isotopologue each one is (13C, 15N, 2H, 33S; 34S, 13C2, 18O)
     -> number of S and N.
   Every mass and every abundance is computed from ELEMENTS through PAL (motore.js): nothing is copied from a table. The unit-resolution pattern
   of PAL.isoPattern is the reference the fine structure must add up to. Generators are pure functions of the seed (PAL.rng). */
"use strict";
const HRBASE = (() => {
  const P = typeof PAL !== "undefined" ? PAL : require("./motore.js");
  const EL = typeof ELEMENTS !== "undefined" ? ELEMENTS : global.ELEMENTS;
  const ELECTRON = 0.000548579909;
  const mono = f => P.exact(f, EL);
  const SHIFT_H = mono({ H: 1 }) - ELECTRON;                // [M+H]+ = M + SHIFT_H
  const ISO_TOL = 2.0;                                      // isotopes "do not fit" above this gap (percentage points of M)
  const HC_MIN = 0.4, HC_MAX = 2.4;                         // H/C range of the test
  const TESTS = { parita: "regola dell'azoto (DBE non intero)", dbe: "DBE negativo", hc: "H/C fuori intervallo", iso: "isotopi non compatibili" };
  const fx = (v, d) => v.toFixed(d).replace(".", ",");
  const pick = (r, a) => a[Math.floor(r() * a.length)];
  const ri = (r, a, b) => a + Math.floor(r() * (b - a + 1));
  const pct = (r, v, rel, abs) => Math.max(0, v * (1 + (2 * r() - 1) * rel) + (2 * r() - 1) * abs);

  // ---------------------------------------------------------------- fine isotopic structure (first three nominal shifts, from ELEMENTS)
  /** isotopes of an element relative to the most abundant one: [{A, shift, dm, r}] (r = abundance ratio, dm = mass difference) */
  function isoOf(sym) {
    const el = EL.find(x => x.s === sym), top = el.iso.slice().sort((a, b) => b[2] - a[2])[0];
    return { top, subs: el.iso.filter(i => i !== top && i[0] > top[0] && i[0] - top[0] <= 2).map(i => ({ sym, A: i[0], shift: i[0] - top[0], dm: i[1] - top[1], r: i[2] / top[2], label: i[0] + sym })) };
  }
  /** all isotopologues of f with nominal shift 1 and 2 (substitutions of one or two atoms): [{shift, dm, rel, label}] with rel = intensity / M */
  function fineStructure(f) {
    let comb = [{ shift: 0, dm: 0, rel: 1, parts: [] }];
    Object.entries(f).forEach(([sym, k]) => {
      const { subs } = isoOf(sym), opts = [{ shift: 0, dm: 0, rel: 1, parts: [] }];
      subs.forEach(s => { opts.push({ shift: s.shift, dm: s.dm, rel: k * s.r, parts: [[s.label, 1]] }); });
      subs.filter(s => s.shift === 1).forEach((a, i, arr) => arr.forEach((b, j) => {
        if (j < i || k < 2) return;
        opts.push({ shift: 2, dm: a.dm + b.dm, rel: i === j ? k * (k - 1) / 2 * a.r * a.r : k * (k - 1) * a.r * b.r, parts: i === j ? [[a.label, 2]] : [[a.label, 1], [b.label, 1]] });
      }));
      const next = [];
      comb.forEach(c => opts.forEach(o => { if (c.shift + o.shift <= 2) next.push({ shift: c.shift + o.shift, dm: c.dm + o.dm, rel: c.rel * o.rel, parts: c.parts.concat(o.parts) }); }));
      comb = next;
    });
    const merged = new Map();
    comb.filter(c => c.shift > 0).forEach(c => {
      const label = c.parts.map(([l, n]) => n > 1 ? l + "x" + n : l).sort().join("+"), key = c.shift + ":" + label, cur = merged.get(key);
      if (cur) cur.rel += c.rel; else merged.set(key, { shift: c.shift, dm: c.dm, rel: c.rel, label });
    });
    return [...merged.values()].sort((a, b) => a.shift - b.shift || a.dm - b.dm);
  }
  /** sum of the fine structure per nominal shift, in percent of M: the same numbers as PAL.isoPattern (checked by the tests) */
  const fineSums = f => { const s = [100, 0, 0]; fineStructure(f).forEach(p => { s[p.shift] += p.rel * 100; }); return s; };

  /** abundance per atom (percent of the main isotopologue) of a substitution, from ELEMENTS */
  const perAtom = sym => isoOf(sym).subs.map(s => ({ label: s.label, shift: s.shift, dm: s.dm, pct: s.r * 100 }));
  const subOf = label => { const m = label.match(/^(\d+)([A-Z][a-z]?)$/); return m ? perAtom(m[2]).find(s => s.label === label) : null; };
  return { P, EL, ELECTRON, mono, SHIFT_H, ISO_TOL, HC_MIN, HC_MAX, TESTS, fx, pick, ri, pct, isoOf, fineStructure, fineSums, perAtom, subOf };
})();

const HRNTNT = (() => {
  const { P, SHIFT_H, mono, fx, pick, ri, pct, ISO_TOL, HC_MIN, HC_MAX, TESTS } = HRBASE;
  // ================================================================ H6: formula from a non-target ion
  const NT_ELEMENTS = { N: [0, 6], O: [0, 10], S: [0, 2] }, dbeOf = f => P.rdb(f);

  /** tests a student can do on a neutral candidate formula (never "right": the list of the tests it fails) */
  function checks(f, obs) {
    const dbe = dbeOf(f), hc = (f.H || 0) / (f.C || 1), pat = P.isoPattern({ ...f, H: (f.H || 0) + 1 }, 4);
    const gap = Math.max(Math.abs(pat[1] - obs[1]), Math.abs(pat[2] - obs[2]));
    const fails = [];
    if (!Number.isInteger(dbe)) fails.push("parita");
    if (dbe < 0) fails.push("dbe");
    if (hc < HC_MIN || hc > HC_MAX) fails.push("hc");
    if (gap > ISO_TOL) fails.push("iso");
    return { dbe, hc, expM1: pat[1], expM2: pat[2], gap, fails };
  }

  /** every neutral formula CHNOS whose [M+H]+ is within +-tolPpm of mz */
  function enumerate(mz, tolPpm) {
    const out = [], w = mz * tolPpm * 1e-6, M = mz - SHIFT_H, mC = 12, mHh = mono({ H: 1 }), mN = mono({ N: 1 }), mO = mono({ O: 1 }), mS = mono({ S: 1 });
    for (let c = 4; c <= 40; c++) for (let n = NT_ELEMENTS.N[0]; n <= NT_ELEMENTS.N[1]; n++) for (let o = 0; o <= NT_ELEMENTS.O[1]; o++) for (let s = 0; s <= NT_ELEMENTS.S[1]; s++) {
      const rest = c * mC + n * mN + o * mO + s * mS;
      const h = Math.round((M - rest) / mHh);
      if (h < 0 || h > 3 * c + n) continue;
      const f = { C: c, H: h }; if (n) f.N = n; if (o) f.O = o; if (s) f.S = s;
      const t = rest + h * mHh + SHIFT_H;
      if (Math.abs(t - mz) <= w) out.push({ f, mzT: t });
    }
    return out;
  }

  function genNt(level, seed) {
    const tolPpm = level >= 2 ? 3 : 5;
    for (let attempt = 0; attempt < 200; attempt++) {
      const r = P.rng((seed * 40503 + attempt * 2654435761) >>> 0);
      const C = ri(r, 9, 24), N = r() < 0.25 ? 0 : ri(r, 1, 4), O = ri(r, 0, 6), S = r() < 0.7 ? 0 : ri(r, 1, 2);
      const dbe0 = ri(r, 1, 10), H = 2 * C + 2 + N - 2 * dbe0;
      const truth = { C, H }; if (N) truth.N = N; if (O) truth.O = O; if (S) truth.S = S;
      if (H < 2 || H / C < 0.55 || H / C > 2.2) continue;
      const M = mono(truth); if (M < 200 || M > 450) continue;
      const mzT = M + SHIFT_H, errPpm = (2 * r() - 1) * tolPpm * 0.7, mz = Math.round(mzT * (1 + errPpm * 1e-6) * 1e4) / 1e4;
      const ion = { ...truth, H: H + 1 }, pat = P.isoPattern(ion, 4);
      const obs = [100, Math.round(pct(r, pat[1], 0.04, 0.15) * 10) / 10, Math.round(pct(r, pat[2], 0.06, 0.06) * 10) / 10];
      const all = enumerate(mz, tolPpm).map(c => ({ ...c, ...checks(c.f, obs) }));
      const key = f => P.fstr(f), tk = key(truth);
      const tr = all.find(c => key(c.f) === tk);
      if (!tr || tr.fails.length) continue;                                   // the generating formula must pass every test
      const others = all.filter(c => c !== tr && c.fails.length);
      // diversity: one candidate per failed test first, then the closest in mass
      const chosen = [], used = new Set();
      P.shuffle(others, r).forEach(c => { const t = c.fails.find(x => !used.has(x)); if (t && chosen.length < 6) { used.add(t); chosen.push(c); } });
      P.shuffle(others.filter(c => !chosen.includes(c)), r).forEach(c => { if (chosen.length < 6) chosen.push(c); });
      if (chosen.length < 5 || used.size < 3) continue;
      const cands = [tr, ...chosen].sort((a, b) => a.mzT - b.mzT).map((c, i) => ({
        id: "ABCDEFG"[i], f: c.f, fs: P.fstr(c.f), mzT: Math.round(c.mzT * 1e4) / 1e4, ppm: (mz - c.mzT) / c.mzT * 1e6, dbe: c.dbe, hc: c.hc,
        expM1: c.expM1, expM2: c.expM2, gap: c.gap, fails: c.fails, truth: c === tr, nC: c.f.C, nN: c.f.N || 0, nS: c.f.S || 0, nO: c.f.O || 0,
      }));
      const nC1 = obs[1] / 1.0818;                                              // upper bound of C from M+1 (13C/12C of ELEMENTS: about 1.08 %)
      const e = { game: "hr-nt", part: "nt", level, seed, tolPpm, mz, obs, cands, truthId: cands.find(c => c.truth).id, truthFormula: tk,
        cRange: [Math.max(1, Math.floor((obs[1] - 3.5) / 1.0818)), Math.ceil(nC1) + 1], id: `NT${level}-${tk}`, d0: level >= 2 ? 0.9 : 0.5 };
      e.help = helpNt(e);
      return e;
    }
    throw new Error("genNt: no exercise for seed " + seed);
  }

  function helpNt(e) {
    const hp = fx(SHIFT_H, 4);
    return [
      `Parti dal calcolo, senza guardare la tabella: massa neutra M = m/z − ${hp} = ${fx(e.mz - SHIFT_H, 4)}. Per ogni candidato tre numeri si calcolano dalla formula: il DBE = C − H/2 + N/2 + 1, il rapporto H/C e la distribuzione isotopica attesa (M+1 e M+2 in % di M).`,
      `Quattro controlli, indipendenti. (1) Regola dell'azoto: il DBE di una molecola neutra è un numero intero; se non lo è, la parità di H + N è sbagliata. (2) DBE non negativo. (3) H/C tra ${HC_MIN} e ${HC_MAX}. (4) Isotopi: M+1 cresce di circa 1,1 punti per atomo di C, M+2 di circa 4,5 per atomo di S (più ossigeno e carbonio).`,
      `Zona. M+1 = ${fx(e.obs[1], 1)}% porta a un numero di atomi di C tra ${e.cRange[0]} e ${e.cRange[1]} (N, H e S contribuiscono poco a M+1); M+2 = ${fx(e.obs[2], 1)}% dice se S può esserci. Confrontate con la colonna dei candidati e controllate gli altri tre test sulle formule rimaste.`,
    ];
  }

  /** was the answer "reason" a correct statement about candidate c? ("ok" is correct only for a candidate with no failed test) */
  const judgeReason = (c, reason) => reason === "ok" ? c.fails.length === 0 : c.fails.includes(reason);

  return { checks, enumerate, genNt, judgeReason };
})();

const HRNT = (() => {
  const { P, ELECTRON, mono, fx, ri, pct, fineStructure, fineSums, subOf, isoOf } = HRBASE;
  // ================================================================ H8: fine isotopic structure
  const FINE_LABELS = ["13C", "15N", "2H", "33S", "34S", "13Cx2", "18O"];
  const THRESHOLD = 0.25;                                   // peaks below this (percent of M) are not listed

  function genFine(level, seed) {
    for (let attempt = 0; attempt < 200; attempt++) {
      const r = P.rng((seed * 69069 + attempt * 1013904223) >>> 0);
      const C = ri(r, 12, 26), N = ri(r, 0, 4), S = ri(r, 0, 2), O = ri(r, 0, 6);
      if (N + S === 0) continue;
      const dbe0 = ri(r, 2, 10), H = 2 * C + 2 + N - 2 * dbe0;
      if (H < 4 || H / C < 0.6 || H / C > 2.2) continue;
      const f = { C, H }; if (N) f.N = N; if (O) f.O = O; if (S) f.S = S;
      const ion = { ...f, H: H + 1 }, m0 = Math.round((mono(ion) - ELECTRON) * 1e4) / 1e4;
      const peaks = fineStructure(ion).map(p => ({ shift: p.shift, label: p.label, dm: p.dm, rel: p.rel * 100 })).filter(p => p.rel >= THRESHOLD);
      if (peaks.some(p => !FINE_LABELS.includes(p.label))) continue;
      const need = { 15: N >= 1, S34: S >= 1 };
      if (!peaks.some(p => p.label === "13C")) continue;
      if (need[15] && !peaks.some(p => p.label === "15N")) continue;
      const shown = peaks.map(p => ({ shift: p.shift, label: p.label, trueRel: p.rel, dm: p.dm,
        mz: Math.round((m0 + p.dm) * (1 + (2 * r() - 1) * 0.6e-6) * 1e5) / 1e5, rel: Math.round(pct(r, p.rel, 0.04, 0.01) * 100) / 100 })).sort((a, b) => a.mz - b.mz);
      shown.forEach((p, i) => { p.id = "p" + (i + 1); });
      const e = { game: "hr-nt", part: "fine", level, seed, formula: f, fs: P.fstr(f), ion, mz0: m0, peaks: shown, labels: FINE_LABELS, nS: S, nN: N,
        sums: fineSums(ion), unit: P.isoPattern(ion, 3), id: `FI-${P.fstr(f)}`, d0: 0.9 };
      e.help = helpFine(e);
      return e;
    }
    throw new Error("genFine: no exercise for seed " + seed);
  }


  function helpFine(e) {
    const d = l => fx(subOf(l).dm, 5), p = l => fx(subOf(l).pct, 3);
    return [
      `Parti dalle differenze: Δm = m/z del picco − m/z di M (${fx(e.mz0, 4)}). Nello stesso raggruppamento nominale (M+1 o M+2) atomi diversi danno differenze diverse alla terza o quarta cifra decimale: è così che l'alta risoluzione li separa.`,
      `Il raggruppamento M+1 può contenere <sup>13</sup>C, <sup>15</sup>N, <sup>2</sup>H e <sup>33</sup>S; il raggruppamento M+2 <sup>34</sup>S, due <sup>13</sup>C insieme e <sup>18</sup>O. Per ciascuna sostituzione la Δm si ricava dalle masse degli isotopi (tavola periodica del programma).`,
      `Zona. Le differenze attese: <sup>15</sup>N ${d("15N")}, <sup>33</sup>S ${d("33S")}, <sup>13</sup>C ${d("13C")}, <sup>2</sup>H ${d("2H")} (M+1); <sup>34</sup>S ${d("34S")}, <sup>18</sup>O ${d("18O")}. Per contare gli atomi dividete l'intensità del picco per quella che dà un solo atomo (per esempio <sup>15</sup>N ${p("15N")} %, <sup>34</sup>S ${p("34S")} % di M per atomo), tenendo conto del rumore.`,
    ];
  }

  /** numbers for one peak after the student has labelled it (never the verdict by itself) */
  function coherenceFine(e, id) {
    const p = e.peaks.find(x => x.id === id), dm = p.mz - e.mz0, sub = p.label.indexOf("x") < 0 ? subOf(p.label) : null;
    return { id, dm, label: p.label, rel: p.rel, perAtomPct: sub ? sub.pct : null, atoms: sub ? p.rel / sub.pct : null };
  }

  return { ...HRBASE, ...HRNTNT, FINE_LABELS, THRESHOLD, genFine, coherenceFine };
})();
if (typeof module !== "undefined") module.exports = HRNT;
