//! What the graph is built from, read from a file: the tree of the precursors of its scans (`msnalbero::albero`) and the consensus of
//! the MS1 and of each node of the tree (the parameters of `msntree`). Used by the command line and by the route of the page.

use super::Spettro;
use crate::error::Result;
use crate::idxread::Scan;
use crate::msnalbero::{albero, consenso, Albero, Raggruppa};
use crate::sorgente::Scansioni;

/// The consensus of a set of scans: peaks within `CONSENSO_PPM`, present in at least half of the scans.
const CONSENSO_PPM: f64 = 5.0;
const CONSENSO_FRAZIONE: f64 = 0.5;

fn consenso_di<M: Scansioni + ?Sized>(
    scans: &[Scan],
    quali: &[usize],
    m: &mut M,
) -> Result<Spettro> {
    let mut sp = Vec::with_capacity(quali.len());
    for &i in quali {
        sp.push(m.spettro(&scans[i])?);
    }
    let (mz, rel, _) = consenso(&sp, CONSENSO_PPM, CONSENSO_FRAZIONE, 0.0);
    Ok(Spettro { mz, rel })
}

/// What the graph is built from.
pub struct Letto {
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
pub fn leggi<M: Scansioni + ?Sized>(scans: &[Scan], m: &mut M, r: &Raggruppa) -> Result<Letto> {
    let a = albero(scans, r);
    let ms1 = if a.ms1.is_empty() {
        None
    } else {
        Some(consenso_di(scans, &a.ms1, m)?)
    };
    let (mut spettri, mut larghezze) = (Vec::with_capacity(a.nodi.len()), vec![]);
    for n in &a.nodi {
        spettri.push(consenso_di(scans, &n.scansioni, m)?);
        larghezze.push(
            n.scansioni
                .iter()
                .filter_map(|&i| scans[i].iso.map(|(lo, hi)| hi - lo))
                .reduce(f64::max),
        );
    }
    let neg = scans.iter().filter(|s| s.polarity < 0).count();
    let pol = if neg * 2 > scans.len() { -1 } else { 1 };
    Ok(Letto {
        albero: a,
        ms1,
        spettri,
        larghezze,
        polarita: pol,
    })
}
