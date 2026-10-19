// @ts-check
// @ts-strict
"use strict";
// Neutral losses (window «Perdite neutre», tables.js): the list and the search for combinations. Pure data and functions, also run by node in tests/test_perdite.py.
// The exact masses are NOT written here: tables.js computes them from the formulas with the element data (elements.js), the test with mzlab/chem/elements.py.
// Sources (primary literature): Levsen et al., J. Mass Spectrom. 42 (2007) 1024-1044; Holcapek, Jirasko, Lisa, J. Chromatogr. A 1217 (2010) 3908-3921;
// Demarque et al., Nat. Prod. Rep. 33 (2016) 432-455; De Vijlder et al., Mass Spectrom. Rev. 37 (2018) 607-629. The «seen in» column and the mechanisms are
// textbook summaries of what those reviews describe; no percentages (none was checked against the full texts). Peptide / lipid specific losses are left out on purpose.
// f = neutral formula (mass computed), short = 2-4 word label for the compact line, pol = typical ionisation polarity ("+", "-" or "±"), rad = the lost species is a radical (odd-electron loss)
const LOSSES = [
  { f: "CH3", name: "loss.CH3.name", short: "loss.CH3.short", rad: true, pol: "±", seen: "loss.CH3.seen", mech: "loss.CH3.mech" },
  { f: "NH3", name: "loss.NH3.name", short: "loss.NH3.short", pol: "+", seen: "loss.NH3.seen", mech: "loss.NH3.mech" },
  { f: "H2O", name: "loss.H2O.name", short: "loss.H2O.short", pol: "±", seen: "loss.H2O.seen", mech: "loss.H2O.mech" },
  { f: "HF", name: "loss.HF.name", short: "loss.HF.short", pol: "±", seen: "loss.HF.seen", mech: "loss.HF.mech" },
  { f: "HCN", name: "loss.HCN.name", short: "loss.HCN.short", pol: "+", seen: "loss.HCN.seen", mech: "loss.HCN.mech" },
  { f: "CO", name: "loss.CO.name", short: "loss.CO.short", pol: "±", seen: "loss.CO.seen", mech: "loss.CO.mech" },
  { f: "C2H4", name: "loss.C2H4.name", short: "loss.C2H4.short", pol: "+", seen: "loss.C2H4.seen", mech: "loss.C2H4.mech" },
  { f: "CH2O", name: "loss.CH2O.name", short: "loss.CH2O.short", pol: "+", seen: "loss.CH2O.seen", mech: "loss.CH2O.mech" },
  { f: "CH4O", name: "loss.CH4O.name", short: "loss.CH4O.short", pol: "±", seen: "loss.CH4O.seen", mech: "loss.CH4O.mech" },
  { f: "H2S", name: "loss.H2S.name", short: "loss.H2S.short", pol: "±", seen: "loss.H2S.seen", mech: "loss.H2S.mech" },
  { f: "HCl", name: "loss.HCl.name", short: "loss.HCl.short", pol: "±", seen: "loss.HCl.seen", mech: "loss.HCl.mech" },
  { f: "Cl", name: "loss.Cl.name", short: "loss.Cl.short", rad: true, pol: "±", seen: "loss.Cl.seen", mech: "loss.Cl.mech" },
  { f: "C2H2O", name: "loss.C2H2O.name", short: "loss.C2H2O.short", pol: "±", seen: "loss.C2H2O.seen", mech: "loss.C2H2O.mech" },
  { f: "C3H6", name: "loss.C3H6.name", short: "loss.C3H6.short", pol: "+", seen: "loss.C3H6.seen", mech: "loss.C3H6.mech" },
  { f: "CO2", name: "loss.CO2.name", short: "loss.CO2.short", pol: "±", seen: "loss.CO2.seen", mech: "loss.CO2.mech" },
  { f: "CH2O2", name: "loss.CH2O2.name", short: "loss.CH2O2.short", pol: "±", seen: "loss.CH2O2.seen", mech: "loss.CH2O2.mech" },
  { f: "NO2", name: "loss.NO2.name", short: "loss.NO2.short", rad: true, pol: "-", seen: "loss.NO2.seen", mech: "loss.NO2.mech" },
  { f: "C4H8", name: "loss.C4H8.name", short: "loss.C4H8.short", pol: "+", seen: "loss.C4H8.seen", mech: "loss.C4H8.mech" },
  { f: "C2H3NO", name: "loss.C2H3NO.name", short: "loss.C2H3NO.short", pol: "+", seen: "loss.C2H3NO.seen", mech: "loss.C2H3NO.mech" },
  { f: "SO2", name: "loss.SO2.name", short: "loss.SO2.short", pol: "±", seen: "loss.SO2.seen", mech: "loss.SO2.mech" },
  { f: "C6H6", name: "loss.C6H6.name", short: "loss.C6H6.short", pol: "+", seen: "loss.C6H6.seen", mech: "loss.C6H6.mech" },
  { f: "HBr", name: "loss.HBr.name", short: "loss.HBr.short", pol: "±", seen: "loss.HBr.seen", mech: "loss.HBr.mech" },
  { f: "SO3", name: "loss.SO3.name", short: "loss.SO3.short", pol: "-", seen: "loss.SO3.seen", mech: "loss.SO3.mech" },
];
const LOSS_REFS = "Levsen 2007; Holčapek 2010";
// catalog key: the texts of this list are in lang/it.js and en.js (keys loss.<formula>.name / seen / mech)
const LOSS_RAD = "loss.rad";
// the candidates for a mass difference: single losses within the tolerance, pairs that add up to it, and repetitions (n x the same loss).
// mass(f) = exact mass of a formula (given by the caller); tol in Da.
function lossCombos(target, mass, tol = 0.5, list = LOSSES) {
  const m = list.map(l => ({ l, m: mass(l.f) }));
  const single = m.filter(x => Math.abs(x.m - target) <= tol).map(x => [x.l]);
  const pairs = [], reps = [];
  for (let i = 0; i < m.length; i++) {
    for (let j = i + 1; j < m.length; j++) if (Math.abs(m[i].m + m[j].m - target) <= tol) pairs.push([m[i].l, m[j].l]);
    for (let n = 2; n <= 4; n++) if (Math.abs(n * m[i].m - target) <= tol) reps.push({ n, l: m[i].l });
  }
  return { single, pairs, reps };
}
if (typeof module !== "undefined") module.exports = { LOSSES, LOSS_REFS, LOSS_RAD, lossCombos };
