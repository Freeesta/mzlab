//! The graph of ONE spectrum: an MS2 with its precursor (a library record, a single scan) as a tree of one level. Used by `mzlab-cli msn-rules` to
//! judge the rules of `fragrules/1` on public spectra; the graph is the one of an infusion, the root being the ion of the precursor m/z.

use super::{costruisci, Grafo, Ingresso, Opzioni, Spettro};
use crate::msnalbero::{albero_di, Raggruppa};

/// The graph of the MS2 `picchi` (m/z, any intensity scale) of the precursor `precursore`, activated as `att` (`CID` = trap type, anything else beam
/// type) at the collision energy `ce`; `polarita` is 1 or -1.
pub fn grafo_da_picchi(
    (precursore, polarita): (f64, i8),
    (att, ce): (&str, f64),
    picchi: &[(f64, f64)],
    o: &Opzioni,
) -> Grafo {
    let percorso = [(precursore, att.to_string(), ce)];
    let al = albero_di(
        vec![(1, None), (2, Some(&percorso[..]))],
        &Raggruppa::default(),
    );
    let base = picchi.iter().fold(0.0f64, |m, p| m.max(p.1));
    let spettro = Spettro {
        mz: picchi.iter().map(|p| p.0).collect(),
        rel: picchi.iter().map(|p| p.1 / base * 100.0).collect(),
    };
    let ms1 = Spettro {
        mz: vec![precursore],
        rel: vec![100.0],
    };
    let ing = Ingresso {
        albero: &al,
        ms1: Some(&ms1),
        spettri: &[spettro],
        larghezze: &[],
        polarita,
    };
    costruisci(&ing, o)
}
