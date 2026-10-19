// Ported from Attributio plugins/metfrag.py (the in-silico fragmentation ranking of MetFrag, without Java or PubChem).
//! In-silico ranking of candidate structures for a feature without a library match. The candidates come from a LOCAL list (PubChemLite-style CSV in the campaign's
//! libraries folder): those whose neutral mass is within `db_ppm` of the precursor are cleaved bond by bond (`review::fragmenter`) and scored like MetFrag's
//! fragmenter score: the weight `mz^0.421 · intensity^0.1307` of the explained peaks over the weight of all the peaks. The bond-energy term of MetFrag and its
//! tolerance `0.005 Da + 10 ppm` are not reproduced (a plain 10 ppm window is used). The result is a hypothesis with its proofs, never an identity.

use crate::attributio::frame::Frame;
use crate::attributio::review::fragmenter::fragment_molecule;
use crate::chem::grafo::Mol;
use mzlab_core::elements;

const PROTON: f64 = 1.00727646677;
const W_MASS: f64 = 0.421;
const W_INT: f64 = 0.1307;

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub id: String,
    pub name: String,
    pub smiles: String,
    pub formula: String,
    /// Monoisotopic neutral mass; from the formula or the SMILES when the list gives none.
    pub mass: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InSilicoOpts {
    pub db_ppm: f64,
    pub frag_ppm: f64,
    pub max_bonds: usize,
    pub top: usize,
}

impl Default for InSilicoOpts {
    fn default() -> Self {
        InSilicoOpts {
            db_ppm: 5.0,
            frag_ppm: 10.0,
            max_bonds: 200,
            top: 5,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ranked {
    pub candidate: Candidate,
    pub score: f64,
    pub explained: Vec<f64>,
    pub mass_ppm: f64,
}

/// Candidates of a CSV list (`,` or `;`): `Identifier`, `CompoundName`/`Name`, `SMILES`, `MolecularFormula`/`Formula`, `MonoisotopicMass`/`ExactMass`.
/// Rows without a SMILES are left out.
pub fn read_candidates(text: &str) -> Vec<Candidate> {
    let sep = if text.lines().next().is_some_and(|l| l.contains(';')) {
        ';'
    } else {
        ','
    };
    let Ok(f) = Frame::parse(text, sep) else {
        return Vec::new();
    };
    let pick = |names: &[&str]| names.iter().find_map(|n| f.col(n));
    let (c_id, c_name, c_smi, c_for, c_mass) = (
        pick(&["Identifier", "id"]),
        pick(&["CompoundName", "Name", "name"]),
        pick(&["SMILES", "smiles"]),
        pick(&["MolecularFormula", "Formula", "formula"]),
        pick(&["MonoisotopicMass", "ExactMass", "mass"]),
    );
    let Some(c_smi) = c_smi else {
        return Vec::new();
    };
    let text_of = |r: usize, c: Option<usize>| {
        c.map(|k| f.cell(r, k).trim().to_string())
            .unwrap_or_default()
    };
    let mut out = Vec::new();
    for r in 0..f.len() {
        let smiles = text_of(r, Some(c_smi));
        if smiles.is_empty() {
            continue;
        }
        let formula = text_of(r, c_for);
        let mass = c_mass
            .and_then(|k| f.num(r, k))
            .or_else(|| {
                elements::parse_formula(&formula)
                    .ok()
                    .map(|x| elements::mass(&x))
            })
            .or_else(|| smiles_mass(&smiles));
        let Some(mass) = mass else { continue };
        out.push(Candidate {
            id: text_of(r, c_id),
            name: text_of(r, c_name),
            smiles,
            formula,
            mass,
        });
    }
    out
}

fn smiles_mass(smiles: &str) -> Option<f64> {
    let m = Mol::parse(smiles.split('.').max_by_key(|p| p.len())?).ok()?;
    Some(elements::mass(&m.formula()))
}

/// MetFrag's fragmenter score of the explained peaks: `Σ w(explained) / Σ w(all)`.
pub fn fragmenter_score(peaks: &[(f64, f64)], explained: &[f64]) -> f64 {
    let w = |mz: f64, it: f64| mz.powf(W_MASS) * it.powf(W_INT);
    let all: f64 = peaks.iter().map(|&(m, i)| w(m, i)).sum();
    let got: f64 = peaks
        .iter()
        .filter(|&&(m, _)| explained.contains(&m))
        .map(|&(m, i)| w(m, i))
        .sum();
    if all > 0.0 {
        got / all
    } else {
        0.0
    }
}

/// Candidates within `db_ppm` of the neutral mass `precursor ∓ proton`, ranked by the fragmenter score (best first, ties by the candidate id); `peaks` are
/// `(m/z, intensity)` of the experimental MS/MS.
pub fn rank(
    precursor_mz: f64,
    pol: &str,
    peaks: &[(f64, f64)],
    cands: &[Candidate],
    o: &InSilicoOpts,
) -> Vec<Ranked> {
    let neutral = if pol == "POS" {
        precursor_mz - PROTON
    } else {
        precursor_mz + PROTON
    };
    let mz: Vec<f64> = peaks.iter().map(|p| p.0).collect();
    let mut out: Vec<Ranked> = Vec::new();
    for c in cands {
        let ppm = (c.mass - neutral) / neutral * 1e6;
        if ppm.abs() > o.db_ppm {
            continue;
        }
        let Ok(fr) = fragment_molecule(&c.smiles, pol, &mz, o.frag_ppm, o.max_bonds) else {
            continue;
        };
        let explained: Vec<f64> = fr.explained.iter().map(|e| e.peak).collect();
        out.push(Ranked {
            candidate: c.clone(),
            score: fragmenter_score(peaks, &explained),
            explained,
            mass_ppm: ppm,
        });
    }
    out.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then(a.candidate.id.cmp(&b.candidate.id))
    });
    out.truncate(o.top.max(1));
    out
}
