//! `FragmentLibrary` and `build_library` of mzFinder: the formulas of the fragments of the parent seen in the MSn tree (the precursors of its
//! nodes) and in the DDA MS2 of the parent, each with the substructures that can give it.

use super::space::candidates;
use super::{Cand, MsnTree};
use crate::legacy::fragtree::{fmt, vec_of};
use crate::sortc::SortC;

/// One formula of the library.
#[derive(Debug, Clone)]
pub struct Entry {
    pub vec: Vec<i64>,
    pub mz: f64,
    /// `msn`, `dda` (a set: each at most once, in the order added).
    pub source: Vec<&'static str>,
    pub rel: f64,
    pub cand: Option<Cand>,
}

/// `FragmentLibrary`: the entries in the order added.
#[derive(Debug, Clone, Default)]
pub struct Library {
    pub entries: Vec<(String, Entry)>,
}

impl Library {
    pub fn get(&self, formula: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.0 == formula).map(|e| &e.1)
    }

    /// `FragmentLibrary.add`: a formula seen twice keeps the atom sets that satisfy both constraints (or the smaller set when none does).
    pub fn add(
        &mut self,
        formula: &str,
        vec: Vec<i64>,
        mz: f64,
        source: &'static str,
        rel: f64,
        cand: Option<Cand>,
    ) {
        let Some(e) = self
            .entries
            .iter_mut()
            .find(|e| e.0 == formula)
            .map(|e| &mut e.1)
        else {
            self.entries.push((
                formula.to_string(),
                Entry {
                    vec,
                    mz,
                    source: vec![source],
                    rel,
                    cand,
                },
            ));
            return;
        };
        if !e.source.contains(&source) {
            e.source.push(source);
        }
        e.rel = e.rel.max(rel);
        let Some((idx, w)) = cand else {
            return;
        };
        let Some((ei, ew)) = e.cand.as_ref() else {
            e.cand = Some((idx, w));
            return;
        };
        let mut both: Vec<usize> = ei.iter().copied().filter(|i| idx.contains(i)).collect();
        both.sortc();
        both.dedup();
        if !both.is_empty() {
            let ww: Vec<f64> = both
                .iter()
                .map(|i| ew[ei.iter().rposition(|x| x == i).unwrap_or(0)])
                .collect();
            let s = ww.iter().fold(0.0, |a, b| a + b);
            e.cand = Some((both, ww.iter().map(|x| x / s).collect()));
        } else if idx.len() < ei.len() {
            e.cand = Some((idx, w));
        }
    }
}

/// `build_library(tree, dda_peaks, min_rel=1.0)` with the default `node_peak_rel=None`: the precursors of the nodes, then the DDA peaks
/// (m/z, relative intensity in %) of at least 1 % that are a sub-formula of the parent ion.
pub fn build(tree: &MsnTree, dda: Option<(&[f64], &[f64])>) -> Library {
    let mut lib = Library::default();
    for n in tree.nodes.iter().filter(|n| n.id != "MS1") {
        let (Some(f), Some(v)) = (
            n.formula.as_deref(),
            n.formula.as_deref().and_then(|f| vec_of(f, &tree.els)),
        ) else {
            continue;
        };
        let cand = tree
            .subs
            .as_ref()
            .and_then(|_| tree.cand.get(&n.id).cloned());
        lib.add(f, v, n.prec_mz.unwrap_or(f64::NAN), "msn", 100.0, cand);
    }
    let Some((mz, rel)) = dda else {
        return lib;
    };
    let top = tree.nodes[0]
        .formula
        .as_deref()
        .and_then(|f| vec_of(f, &tree.els))
        .unwrap_or_default();
    for (&m, &r) in mz.iter().zip(rel) {
        if r < 1.0 {
            continue;
        }
        let Some(&c) = candidates(&tree.space, m, 5.0, 0.002, Some(&top)).first() else {
            continue;
        };
        let v = tree.space.g[c].clone();
        let cand = tree.subs.as_ref().map(|s| s.candidates(&v));
        lib.add(&fmt(&v, &tree.els), v, m, "dda", r, cand);
    }
    lib
}
