//! What the graph is built from, read from a file: the tree of the precursors of its scans (`msntree_build::albero`) and the consensus of
//! the MS1 and of each node of the tree (the parameters of `msntree`). Used by the command line and by the route of the page.

use super::Spettro;
use crate::budget::refusal;
use crate::error::Result;
use crate::idxread::Scan;
use crate::msntree_build::{albero, consenso, Albero, Grouping};
use crate::source::Scansioni;

/// The consensus of a set of scans: peaks within `CONSENSUS_PPM`, present in at least half of the scans.
const CONSENSUS_PPM: f64 = 5.0;
const CONSENSUS_FRACTION: f64 = 0.5;

/// Bytes a point of a spectrum costs while its consensus is made: the pair (m/z, intensity) and its entry among the peaks of `consenso`.
pub const POINT_BYTES: usize = 40;
/// Spectra read before the estimate extrapolates to all the scans of the consensus.
const SAMPLE: usize = 8;

/// Every spectrum of a consensus is held until it is made (an infusion can have thousands of MS1 scans): the bytes held so far, extrapolated to
/// all of `which` once `SAMPLE` are read, end in the calm refusal (phase `msnGraph`) as soon as they pass `budget`.
fn consensus_of<M: Scansioni + ?Sized>(
    scans: &[Scan],
    which: &[usize],
    m: &mut M,
    budget: usize,
) -> Result<Spettro> {
    let mut sp = Vec::with_capacity(which.len());
    let mut held = 0usize;
    for &i in which {
        let s = m.spettro(&scans[i])?;
        held = held.saturating_add(s.0.len().saturating_mul(POINT_BYTES));
        sp.push(s);
        let est = if sp.len() >= SAMPLE.min(which.len()) {
            (held as u128 * which.len() as u128 / sp.len() as u128).min(usize::MAX as u128) as usize
        } else {
            held
        };
        if est > budget {
            return Err(refusal("msnGraph", est, budget));
        }
    }
    let (mz, rel, _) = consenso(&sp, CONSENSUS_PPM, CONSENSUS_FRACTION, 0.0);
    Ok(Spettro { mz, rel })
}

/// What the graph is built from.
pub struct ReadDone {
    pub albero: Albero,
    /// The consensus of the MS1, `None` without MS1 scans.
    pub ms1: Option<Spettro>,
    /// The consensus of each node of the tree.
    pub spettri: Vec<Spettro>,
    /// The widest isolation window (Da) among the scans of each node of the tree.
    pub larghezze: Vec<Option<f64>>,
    /// The polarity of most scans.
    pub polarita: i8,
}

/// The tree of the scans and the consensus spectra of its nodes.
/// `budget`: the bytes the spectra of one consensus may hold (`Options::budget`).
pub fn read<M: Scansioni + ?Sized>(
    scans: &[Scan],
    m: &mut M,
    r: &Grouping,
    budget: usize,
) -> Result<ReadDone> {
    let a = albero(scans, r);
    let ms1 = if a.ms1.is_empty() {
        None
    } else {
        Some(consensus_of(scans, &a.ms1, m, budget)?)
    };
    let (mut spettri, mut widths) = (Vec::with_capacity(a.nodi.len()), vec![]);
    for n in &a.nodi {
        spettri.push(consensus_of(scans, &n.scansioni, m, budget)?);
        widths.push(
            n.scansioni
                .iter()
                .filter_map(|&i| scans[i].iso.map(|(lo, hi)| hi - lo))
                .reduce(f64::max),
        );
    }
    let neg = scans.iter().filter(|s| s.polarity < 0).count();
    let pol = if neg * 2 > scans.len() { -1 } else { 1 };
    Ok(ReadDone {
        albero: a,
        ms1,
        spettri,
        larghezze: widths,
        polarita: pol,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readers::Field;

    /// Every scan has the same spectrum of `n` points.
    struct Same(usize);
    impl Scansioni for Same {
        fn spettro(&mut self, _: &Scan) -> Result<(Vec<f64>, Vec<f64>)> {
            Ok((
                (0..self.0).map(|i| 100.0 + i as f64).collect(),
                vec![1.0; self.0],
            ))
        }
        fn scan_header(&mut self, _: &Scan) -> Result<Vec<Field>> {
            Ok(vec![])
        }
    }

    #[test]
    fn a_consensus_over_the_budget_is_a_calm_refusal_with_the_estimate() {
        let ms1 = Scan {
            level: 1,
            polarity: 1,
            ..Scan::default()
        };
        let scans = vec![ms1; 1000];
        // 1000 scans of 1000 points: 40 MB of spectra; refused after the sample with the whole estimate, not after 1000 reads
        let e = read(&scans, &mut Same(1000), &Grouping::default(), 4 << 20)
            .err()
            .unwrap();
        assert_eq!(e.key, "error.memoryBudget");
        let p = |k: &str| {
            e.params
                .iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| v.clone())
        };
        assert_eq!(p("fase").as_deref(), Some("msnGraph"));
        assert_eq!(p("stima_mb").as_deref(), Some("39"));
        assert_eq!(p("budget_mb").as_deref(), Some("4"));
        let a = read(&scans, &mut Same(1000), &Grouping::default(), 64 << 20).unwrap();
        let b = read(&scans, &mut Same(1000), &Grouping::default(), usize::MAX).unwrap();
        assert_eq!(a.ms1.unwrap().mz, b.ms1.unwrap().mz);
    }
}
