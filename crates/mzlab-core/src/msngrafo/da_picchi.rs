//! The graph of ONE spectrum: an MS2 with its precursor (a library record, a single scan) as a tree of one level. Used by `mzlab-cli msn-rules` to
//! judge the rules of `fragrules/1` on public spectra; the graph is the one of an infusion, the root being the ion of the precursor m/z.

use super::{build, Graph, Input, Options, Spettro};
use crate::msnalbero::{albero_di, Grouping};

/// The graph of the MS2 `picchi` (m/z, any intensity scale) of the precursor `precursore`, activated as `att` (`CID` = trap type, anything else beam
/// type) at the collision energy `ce`; `polarita` is 1 or -1.
pub fn graph_from_peaks(
    (precursore, polarita): (f64, i8),
    (att, ce): (&str, f64),
    peaks: &[(f64, f64)],
    o: &Options,
) -> Graph {
    let path = [(precursore, att.to_string(), ce)];
    let al = albero_di(vec![(1, None), (2, Some(&path[..]))], &Grouping::default());
    let base = peaks.iter().fold(0.0f64, |m, p| m.max(p.1));
    let spettro = Spettro {
        mz: peaks.iter().map(|p| p.0).collect(),
        rel: peaks.iter().map(|p| p.1 / base * 100.0).collect(),
    };
    let ms1 = Spettro {
        mz: vec![precursore],
        rel: vec![100.0],
    };
    let ing = Input {
        albero: &al,
        ms1: Some(&ms1),
        spettri: &[spettro],
        widths: &[],
        polarita,
    };
    build(&ing, o)
}
