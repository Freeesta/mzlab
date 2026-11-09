//! The cleavages of the structure of the user that explain the m/z of one node of an `msngrafo/1`: the fragmenter of mzLab (`fragmenter`) run on the
//! node's m/z and on the peaks of its own spectrum, the cuts whose piece gives the m/z of the node grouped by that m/z. Used by `mzlab-cli msn-graph
//! --strutture` and the `.wasm` `msngrafo` (`strutture_testo`, the same bytes); the types of the answer are in `mzlab_core::msngrafo`. Candidates, never identifications.

use super::fragmenter::{fragment_molecule, FragMatch, FragmentSide};
use super::grafo::Mol;
use crate::sortc::SortC;
use mzlab_core::error::{Error, Result as CoreResult};
use mzlab_core::msngrafo::{
    with_all_structures, with_structures, Cut, ExplainedPeak, Graph, StructureCandidate,
    Structures, StructuresInput,
};

/// Cleavable bonds taken at most (the fragmenter cuts the first ones in the order of the SMILES).
const MAX_BONDS: usize = 400;
/// Two theoretical m/z are the same composition within this distance (Da); the electron mass is the same on both sides.
const SAME_MASS: f64 = 2e-4;

fn cut(
    ing: &StructuresInput,
    bond: (usize, (usize, usize), &str),
    lato: u8,
    (side, other): (&FragmentSide, &FragmentSide),
    m: &FragMatch,
) -> Cut {
    let picchi = side
        .matches
        .iter()
        .filter(|x| x.peak != ing.mz)
        .map(|x| ExplainedPeak {
            mz: x.peak,
            rel: ing
                .picchi
                .iter()
                .find(|p| p.0 == x.peak)
                .map_or(0.0, |p| p.1),
            ppm: x.ppm,
            formula: x.formula.clone(),
            loss: x.loss.clone(),
        })
        .collect();
    Cut {
        legame: bond.0,
        atoms: bond.1,
        label: bond.2.to_string(),
        lato,
        piece: side.atoms.clone(),
        rest: other.atoms.clone(),
        formula: side.formula.clone(),
        variant: m.var.to_string(),
        perdita: m.loss.clone(),
        theoretical_mz: m.mz_th,
        ppm: m.ppm,
        peaks: picchi,
    }
}

/// The graph text `j` of `g` with the key `strutture` of node `nodo` (`nK` or `K`): the cleavages of `smiles` that give its m/z within `ppm`.
pub fn strutture_testo(
    g: &Graph,
    j: &str,
    smiles: Option<&str>,
    (nodo, ppm): (&str, f64),
) -> CoreResult<String> {
    let smiles = smiles.filter(|s| !s.trim().is_empty());
    let smiles = smiles.ok_or_else(|| Error::new("err.msngrafo.noSmiles"))?;
    let bad = || Error::new("err.msngrafo.badNode").with("node", nodo);
    if nodo == "tutti" || nodo == "all" {
        let mut all_items = vec![];
        for (k, _) in g.nodi.iter().enumerate().filter(|(_, n)| n.livello >= 2) {
            let ing = g.structures_input(k).ok_or_else(bad)?;
            let s = structures(smiles, &ing, ppm)
                .map_err(|e| Error::new("err.msngrafo.badSmiles").with("detail", e))?;
            if !s.candidates.is_empty() {
                all_items.push(s);
            }
        }
        return Ok(with_all_structures(j, &all_items));
    }
    let k: usize = nodo.trim_start_matches('n').parse().map_err(|_| bad())?;
    let ing = g.structures_input(k).ok_or_else(bad)?;
    let s = structures(smiles, &ing, ppm)
        .map_err(|e| Error::new("err.msngrafo.badSmiles").with("detail", e))?;
    Ok(with_structures(j, &s))
}

/// The cleavages of `smiles` that give the m/z of the node `ing` within `ppm`. `Err` = the SMILES cannot be read.
pub fn structures(smiles: &str, ing: &StructuresInput, ppm: f64) -> Result<Structures, String> {
    let mut peaks = vec![ing.mz];
    peaks.extend(ing.picchi.iter().map(|p| p.0).filter(|&mz| mz != ing.mz));
    let pol = if ing.polarita < 0 { "NEG" } else { "POS" };
    let frag = fragment_molecule(smiles, pol, &peaks, ppm, MAX_BONDS)?;
    let mixture = smiles.split('.').filter(|p| !p.trim().is_empty()).count() > 1;
    let mut out = Structures {
        node: ing.nodo,
        smiles: smiles.to_string(),
        mz: ing.mz,
        ppm_tol: ppm,
        atoms: if mixture {
            0
        } else {
            Mol::parse(smiles).map_or(0, |m| m.atoms.len())
        },
        bonds: frag.bonds.len(),
        candidates: vec![],
        warnings: if mixture {
            vec!["warn.msngrafo.structMixture"]
        } else {
            vec![]
        },
    };
    for b in &frag.bonds {
        let (Some(f1), Some(f2)) = (&b.frag1, &b.frag2) else {
            continue;
        };
        let sides = [(1u8, (f1, f2)), (2u8, (f2, f1))];
        for (lato, pair) in sides {
            for m in pair.0.matches.iter().filter(|m| m.peak == ing.mz) {
                let t = cut(ing, (b.bond_idx, b.atoms, &b.label), lato, pair, m);
                match out
                    .candidates
                    .iter_mut()
                    .find(|c| (c.theoretical_mz - t.theoretical_mz).abs() < SAME_MASS)
                {
                    Some(c) => c.cuts.push(t),
                    None => out.candidates.push(StructureCandidate {
                        formula: ing
                            .formule
                            .iter()
                            .find(|f| (f.1 - t.theoretical_mz).abs() < SAME_MASS)
                            .map(|f| f.0.clone()),
                        theoretical_mz: t.theoretical_mz,
                        ppm: t.ppm,
                        cuts: vec![t],
                    }),
                }
            }
        }
    }
    for c in &mut out.candidates {
        c.cuts
            .sortc_by_key(|t| (t.legame, t.lato, t.variant.clone()));
        c.ppm = c
            .cuts
            .iter()
            .map(|t| t.ppm)
            .min_by(|a, b| a.abs().total_cmp(&b.abs()))
            .unwrap_or(c.ppm);
    }
    out.candidates.sortc_by(|a, b| {
        a.ppm
            .abs()
            .total_cmp(&b.ppm.abs())
            .then(a.theoretical_mz.total_cmp(&b.theoretical_mz))
    });
    Ok(out)
}
