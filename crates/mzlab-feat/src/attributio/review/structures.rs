// Ported from Attributio src/attributio/review/build.py (Structures, structures_of).
//! The candidate structures of a feature and what the review reads from a SMILES: its formula and the first block of its InChIKey. The reference tool does this
//! with RDKit when it is installed (and does without when it is not); here `Structures` has two implementations: `GraphStructures`, the molecular graph of mzLab
//! (formula only: it has no InChIKey, so the identity of a structure comes from the sources), and `NoStructures`, which reads nothing, as the reference tool
//! does without RDKit.

use super::explain::Formula;
use crate::attributio::annotate::Feature;
use crate::chem::grafo::Mol;
use mzlab_core::elements;

/// What the review reads from a SMILES.
pub trait Structures {
    /// The neutral formula of the largest piece of the structure, `None` when the SMILES cannot be read.
    fn formula(&self, smiles: &str) -> Option<Formula>;
    /// The first block of the InChIKey (14 characters), `None` when it cannot be computed.
    fn ik1(&self, smiles: &str) -> Option<String>;
}

/// No structure can be read: every candidate falls back on the formulas of the sources.
pub struct NoStructures;

impl Structures for NoStructures {
    fn formula(&self, _: &str) -> Option<Formula> {
        None
    }
    fn ik1(&self, _: &str) -> Option<String> {
        None
    }
}

/// The molecular graph of mzLab.
pub struct GraphStructures;

impl Structures for GraphStructures {
    fn formula(&self, smiles: &str) -> Option<Formula> {
        if smiles.trim().is_empty() {
            return None;
        }
        let m = smiles
            .split('.')
            .filter_map(|p| Mol::parse(p).ok())
            .max_by_key(|m| m.atoms.len())?;
        let f: elements::Formula = m.formula();
        Some(f.into_iter().map(|(e, n)| (e, n as i64)).collect())
    }
    fn ik1(&self, _: &str) -> Option<String> {
        None
    }
}

/// Candidate structures of a feature: `(label, smiles)` from CSI:FingerID and the libraries, without repeating a SMILES.
pub fn structures_of(f: &Feature) -> Vec<(&'static str, String)> {
    let cols: [(&'static str, Option<&str>); 4] = [
        ("CSI:FingerID", f.csi.smiles.as_deref()),
        ("library", f.mzlib.smiles.as_deref()),
        ("library", f.gnps.smiles.as_deref()),
        ("CSI:FingerID", f.smiles.as_deref()),
    ];
    let mut out: Vec<(&'static str, String)> = Vec::new();
    for (label, s) in cols {
        if let Some(s) = s.filter(|s| !s.trim().is_empty()) {
            if out.iter().all(|x| x.1 != s) {
                out.push((label, s.to_string()));
            }
        }
    }
    out
}
