// @ts-check
// PFAS evidence for the feature table (high resolution): mass defect per carbon against mass per carbon (MD/C-m/C) and the search of diagnostic
// fragment ions and neutral losses in an MS2 spectrum. Pure functions, no page: used by pfas.js and tested under node (tests/test_pfas.py).
// Everything here is EVIDENCE for a CANDIDATE (a count of matched fragments and losses with their ppm error): nothing says what an ion is.

const ELECTRON = 0.00054857990946;
/** @type {Record<string, number>} exact monoisotopic masses (C = 12 by definition) */
const M = { C: 12, H: 1.00782503223, N: 14.00307400443, O: 15.99491461957, F: 18.99840316273, S: 31.9720711744 };
/** @param {Record<string, number>} parts */
const massOf = parts => Object.entries(parts).reduce((a, [e, n]) => a + M[e] * n, 0);

/** Fragment ions often seen in the MS2 of perfluoroalkyl substances (pol = polarity of the ion). Hints, not a classification. */
const FRAGMENTS = [
  { id: "CF3", label: "CF3", parts: { C: 1, F: 3 }, pol: -1 }, { id: "C2F5", label: "C2F5", parts: { C: 2, F: 5 }, pol: -1 },
  { id: "C3F7", label: "C3F7", parts: { C: 3, F: 7 }, pol: -1 }, { id: "C4F9", label: "C4F9", parts: { C: 4, F: 9 }, pol: -1 },
  { id: "FSO3", label: "FSO3", parts: { F: 1, S: 1, O: 3 }, pol: -1 }, { id: "SO3", label: "SO3", parts: { S: 1, O: 3 }, pol: -1 },
  { id: "CF3", label: "CF3", parts: { C: 1, F: 3 }, pol: 1 }, { id: "C2F5", label: "C2F5", parts: { C: 2, F: 5 }, pol: 1 },
  { id: "C3F7", label: "C3F7", parts: { C: 3, F: 7 }, pol: 1 }, { id: "C4F9", label: "C4F9", parts: { C: 4, F: 9 }, pol: 1 },
];
/** Neutral losses (Δm between the precursor and a product ion) often seen for PFAS. */
const LOSSES = [
  { id: "HF", label: "HF", parts: { H: 1, F: 1 } }, { id: "2HF", label: "2 HF", parts: { H: 2, F: 2 } }, { id: "CO2", label: "CO2", parts: { C: 1, O: 2 } },
  { id: "HFCO2", label: "HF + CO2", parts: { H: 1, F: 1, C: 1, O: 2 } }, { id: "CF2", label: "CF2", parts: { C: 1, F: 2 } },
  { id: "C2F4", label: "C2F4", parts: { C: 2, F: 4 } }, { id: "C3F6", label: "C3F6", parts: { C: 3, F: 6 } }, { id: "SO3", label: "SO3", parts: { S: 1, O: 3 } },
];
const CF2 = massOf({ C: 1, F: 2 });

/** m/z of a fragment ion with its polarity (the electron is added to an anion and taken from a cation). */
const fragmentMz = (/** @type {{parts: Record<string, number>, pol: number}} */ f) => massOf(f.parts) - f.pol * ELECTRON;
const lossMass = (/** @type {{parts: Record<string, number>}} */ l) => massOf(l.parts);

/** Number of carbons of a formula written like C8F15O2 (0 when there is none). */
function carbons(formula) {
  const m = /C(\d*)(?![a-z])/.exec(formula || "");
  return m ? (m[1] === "" ? 1 : +m[1]) : 0;
}
/** Number of atoms of one element of a formula (F in C8F15O2 = 15). */
function atoms(formula, el) {
  const m = new RegExp(el + "(\\d*)(?![a-z])").exec(formula || "");
  return m ? (m[1] === "" ? 1 : +m[1]) : 0;
}
/** The candidate formula of an ion among the results of /api/composition: the one with the smallest error, among those with at least minF fluorines.
 *  @param {{formula: string, delta_ppm: number}[]} results  @param {number} [minF] */
function pickFormula(results, minF = 3) {
  const ok = (results || []).filter(r => atoms(r.formula, "F") >= minF && carbons(r.formula) > 0);
  ok.sort((a, b) => Math.abs(a.delta_ppm) - Math.abs(b.delta_ppm));
  return ok.length ? { formula: ok[0].formula, ppm: ok[0].delta_ppm, n: ok.length } : null;
}

/** Mass defect per carbon (MD/C) and mass per carbon (m/C) of an ion with c carbons; MD = m/z - round(m/z). null without carbons. */
function mdc(mz, c) {
  if (!(c > 0) || !(mz > 0)) return null;
  const md = mz - Math.round(mz);
  return { mc: mz / c, mdc: md / c, md };
}

/** Diagnostic fragment ions and neutral losses of one MS2 spectrum.
 *  @param {{mz: number[], y: number[]}} sp  centroids of the MS2
 *  @param {number} prec  m/z of the precursor
 *  @param {number} pol   +1 or -1
 *  @param {{tolPpm?: number, minRel?: number}} [o]  tolPpm: tolerance on the product ion; minRel: smallest peak, in % of the base peak */
function search(sp, prec, pol, o) {
  const tolPpm = o && o.tolPpm > 0 ? o.tolPpm : 5, minRel = o && o.minRel >= 0 ? o.minRel : 1;
  const top = sp.y.reduce((a, v) => Math.max(a, v), 0);
  /** @type {{mz: number, rel: number}[]} */
  const pk = [];
  sp.mz.forEach((mz, i) => { const rel = top > 0 ? 100 * sp.y[i] / top : 0; if (rel >= minRel) pk.push({ mz, rel }); });
  const near = (/** @type {number} */ target) => {
    let best = null;
    for (const p of pk) { const d = p.mz - target; if (Math.abs(d) <= tolPpm * 1e-6 * target && (!best || Math.abs(d) < Math.abs(best.mz - target))) best = p; }
    return best;
  };
  const frags = [], losses = [];
  for (const f of FRAGMENTS.filter(x => x.pol === pol)) {
    const mz = fragmentMz(f), p = near(mz);
    if (p) frags.push({ id: f.id, label: f.label, mz, obs: p.mz, ppm: (p.mz - mz) / mz * 1e6, rel: p.rel });
  }
  for (const l of LOSSES) {
    const dm = lossMass(l), p = near(prec - dm);
    if (p && dm > 1) losses.push({ id: l.id, label: l.label, dm, obs: p.mz, ppm: (p.mz - (prec - dm)) / (prec - dm) * 1e6, rel: p.rel });
  }
  // pairs of peaks that differ by CF2: a ladder of perfluoroalkyl fragments
  let cf2 = 0;
  for (let i = 0; i < pk.length; i++) for (let j = i + 1; j < pk.length; j++) { if (Math.abs(pk[j].mz - pk[i].mz - CF2) <= tolPpm * 1e-6 * pk[j].mz) cf2++; }
  return { frags, losses, cf2, n: frags.length + losses.length + (cf2 > 0 ? 1 : 0), peaks: pk.length };
}

export { FRAGMENTS, LOSSES, CF2, fragmentMz, lossMass, carbons, atoms, pickFormula, mdc, search };
