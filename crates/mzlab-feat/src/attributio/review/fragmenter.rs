// Ported from Attributio src/attributio/review/fragmenter.py.
//! In-silico systematic bond cleavage: every acyclic single bond of a candidate structure is cut, each side takes a hydrogen, and the ion forms and neutral-loss
//! variants of both sides are matched against the experimental MS/MS peaks within a ppm tolerance. Built on the molecular graph of mzLab (`chem::grafo`), no
//! RDKit; the drawing of a fragment is not made here (the page draws the atoms listed in `atoms`).

use super::isotopes::round_to;
use crate::chem::grafo::Mol;
use mzlab_core::elements::{self, Formula};

const ION_H: f64 = 1.00727646677;
const LOSS_H2O: f64 = 18.010564684;
const LOSS_CO: f64 = 27.994914622;
const LOSS_CO2: f64 = 43.989829244;
const LOSS_NH3: f64 = 17.026549101;

/// A peak matched by an ion form of a fragment.
#[derive(Clone, Debug, PartialEq)]
pub struct FragMatch {
    pub peak: f64,
    pub mz_th: f64,
    pub ppm: f64,
    pub formula: String,
    pub loss: String,
    pub var: &'static str,
}

/// One side of a cleaved bond.
#[derive(Clone, Debug, PartialEq)]
pub struct FragmentSide {
    pub formula: String,
    pub mass: f64,
    pub mz_pos: f64,
    pub mz_neg: f64,
    /// Heavy atoms of the fragment (indices of the SMILES, 0-based).
    pub atoms: Vec<usize>,
    pub matches: Vec<FragMatch>,
}

/// A cleaved bond with the two sides.
#[derive(Clone, Debug, PartialEq)]
pub struct BondCut {
    pub bond_idx: usize,
    pub label: String,
    pub atom1: String,
    pub atom2: String,
    pub frag1: Option<FragmentSide>,
    pub frag2: Option<FragmentSide>,
}

/// The best fragment for an experimental peak.
#[derive(Clone, Debug, PartialEq)]
pub struct ExplainedPeak {
    pub peak: f64,
    pub formula: String,
    pub mz_th: f64,
    pub ppm: f64,
    pub loss: String,
    pub bond_label: String,
    pub atoms: Vec<usize>,
}

/// All the cleavages of a structure and the peaks they explain (sorted by m/z).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fragmentation {
    pub bonds: Vec<BondCut>,
    pub explained: Vec<ExplainedPeak>,
}

impl Fragmentation {
    pub fn n_explained(&self) -> usize {
        self.explained.len()
    }
}

fn variants(f_str: &str, mass: f64, ion_delta: f64) -> Vec<(&'static str, String, f64, &'static str)> {
    let mut v: Vec<(&'static str, String, f64, &'static str)> =
        vec![("ion", f_str.to_string(), mass + ion_delta, ""), ("radical", f_str.to_string(), mass, "(radical)")];
    let has = |c: &str| f_str.contains(c);
    if has("O") {
        v.push(("-H2O", format!("{f_str}-H2O"), mass + ion_delta - LOSS_H2O, "-H2O"));
        v.push(("-2H2O", format!("{f_str}-2H2O"), mass + ion_delta - 2.0 * LOSS_H2O, "-2H2O"));
    }
    if has("C") && has("O") {
        v.push(("-CO", format!("{f_str}-CO"), mass + ion_delta - LOSS_CO, "-CO"));
    }
    if f_str.matches('O').count() >= 2 && has("C") {
        v.push(("-CO2", format!("{f_str}-CO2"), mass + ion_delta - LOSS_CO2, "-CO2"));
    }
    if has("N") {
        v.push(("-NH3", format!("{f_str}-NH3"), mass + ion_delta - LOSS_NH3, "-NH3"));
    }
    v
}

/// The side of a cut with the hydrogen that replaces the attachment point.
fn side_formula(m: &Mol, keep: &[bool]) -> Formula {
    let mut f = m.formula_of(keep);
    match f.iter_mut().find(|(e, _)| e == "H") {
        Some(h) => h.1 += 1,
        None => f.push(("H".to_string(), 1)),
    }
    f
}

fn atom_label(m: &Mol, i: usize) -> String {
    format!("{}{}", m.atoms[i].el, i + 1)
}

/// The largest piece (heavy atoms) of a SMILES with salts or mixtures.
fn main_piece(smiles: &str) -> Option<(Mol, String)> {
    smiles.split('.').filter_map(|p| Mol::parse(p).ok().map(|m| (m, p.to_string()))).max_by_key(|(m, _)| m.atoms.len())
}

struct Ctx<'a> {
    peaks: &'a [f64],
    ppm_tol: f64,
    ion_delta: f64,
}

fn side_of(m: &Mol, keep: &[bool], ctx: &Ctx, label: &str, best: &mut Vec<ExplainedPeak>) -> FragmentSide {
    let f = side_formula(m, keep);
    let f_str = elements::fmt(&f);
    let mass = elements::mass(&f);
    let atoms: Vec<usize> = (0..keep.len()).filter(|&i| keep[i]).collect();
    let mut matches = Vec::new();
    for (var, v_formula, mz_th, loss) in variants(&f_str, mass, ctx.ion_delta) {
        for &p in ctx.peaks {
            let ppm = (p - mz_th) / mz_th * 1e6;
            if ppm.abs() > ctx.ppm_tol {
                continue;
            }
            matches.push(FragMatch {
                peak: p,
                mz_th: round_to(mz_th, 5),
                ppm: round_to(ppm, 2),
                formula: v_formula.clone(),
                loss: loss.to_string(),
                var,
            });
            let cand = ExplainedPeak {
                peak: p,
                formula: v_formula.clone(),
                mz_th: round_to(mz_th, 5),
                ppm: round_to(ppm, 2),
                loss: loss.to_string(),
                bond_label: label.to_string(),
                atoms: atoms.clone(),
            };
            match best.iter_mut().find(|e| e.peak == p) {
                Some(e) if ppm.abs() < e.ppm.abs() => *e = cand,
                Some(_) => {}
                None => best.push(cand),
            }
        }
    }
    FragmentSide { formula: f_str, mass: round_to(mass, 5), mz_pos: round_to(mass + ION_H, 5), mz_neg: round_to(mass - ION_H, 5), atoms, matches }
}

/// Cleaves the acyclic single bonds of `smiles` (the first `max_bonds`), computes the fragments and matches them against `exp_peaks` within `ppm_tol`.
pub fn fragment_molecule(smiles: &str, pol: &str, exp_peaks: &[f64], ppm_tol: f64, max_bonds: usize) -> Result<Fragmentation, String> {
    if smiles.trim().is_empty() {
        return Err("No SMILES provided".into());
    }
    // a mixture (salt, co-crystal) gives three pieces at every cut, which the reference tool skips: no cleavage is made
    if smiles.split('.').filter(|p| !p.trim().is_empty()).count() > 1 {
        main_piece(smiles).ok_or("Invalid SMILES")?;
        return Ok(Fragmentation::default());
    }
    let (m, _) = main_piece(smiles).ok_or("Invalid SMILES")?;
    let on_ring = m.ring_bonds();
    // an aromatic-looking bond outside a ring (a biaryl bond) is single, as in the reference tool
    let breakable: Vec<usize> =
        (0..m.bonds.len()).filter(|&k| !on_ring[k] && (m.bonds[k].order == 1.0 || m.bonds[k].order == 1.5)).take(max_bonds).collect();
    let peaks: Vec<f64> = exp_peaks.iter().cloned().filter(|p| *p > 0.0).collect();
    let ctx = Ctx { peaks: &peaks, ppm_tol, ion_delta: if pol == "POS" { ION_H } else { -ION_H } };
    let mut out = Fragmentation::default();
    for k in breakable {
        let (a, b) = (m.bonds[k].a, m.bonds[k].b);
        let label = format!("{}-{}", atom_label(&m, a), atom_label(&m, b));
        let (sa, sb) = m.sides(k);
        let lowest_first = sa.iter().position(|&x| x) < sb.iter().position(|&x| x);
        let (first, second) = if lowest_first { (&sa, &sb) } else { (&sb, &sa) };
        let f1 = side_of(&m, first, &ctx, &label, &mut out.explained);
        let f2 = side_of(&m, second, &ctx, &label, &mut out.explained);
        out.bonds.push(BondCut { bond_idx: k, label, atom1: atom_label(&m, a), atom2: atom_label(&m, b), frag1: Some(f1), frag2: Some(f2) });
    }
    out.explained.sort_by(|x, y| x.peak.partial_cmp(&y.peak).unwrap());
    Ok(out)
}
