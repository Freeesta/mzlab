//! What the graph is built from, read from a file: the tree of the precursors of its scans (`msnalbero::albero`) and the consensus of
//! the MS1 and of each node of the tree (the parameters of `msntree`). Used by the command line and by the route of the page.

use super::Spettro;
use crate::error::Result;
use crate::idxread::Scan;
use crate::msnalbero::{albero, consenso, Albero, Grouping};
use crate::sorgente::Scansioni;

/// The consensus of a set of scans: peaks within `CONSENSUS_PPM`, present in at least half of the scans.
const CONSENSUS_PPM: f64 = 5.0;
const CONSENSUS_FRACTION: f64 = 0.5;

fn consensus_of<M: Scansioni + ?Sized>(
    scans: &[Scan],
    which: &[usize],
    m: &mut M,
) -> Result<Spettro> {
    let mut sp = Vec::with_capacity(which.len());
    for &i in which {
        sp.push(m.spettro(&scans[i])?);
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
pub fn read<M: Scansioni + ?Sized>(scans: &[Scan], m: &mut M, r: &Grouping) -> Result<ReadDone> {
    let a = albero(scans, r);
    let ms1 = if a.ms1.is_empty() {
        None
    } else {
        Some(consensus_of(scans, &a.ms1, m)?)
    };
    let (mut spettri, mut widths) = (Vec::with_capacity(a.nodi.len()), vec![]);
    for n in &a.nodi {
        spettri.push(consensus_of(scans, &n.scansioni, m)?);
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
