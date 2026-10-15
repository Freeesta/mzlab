//! The similarity of the MS2 of a row with the MS2 of the parent at unit resolution: the consensus spectrum of each (the pooled fragments of the
//! scans that `ms2link` gave them, merged within `TOL_LR`) compared with `pair_components_tol`, as the high-resolution Dig does in three components.

use mzlab_core::idxread::{Index, ReadAt};

use super::ms2::{pool, Pool};
use super::ms2link::{small, Links};
use super::{Entry, LrDig};
use crate::legacy::similarity::{pair_components_tol, prepare, Components, Spec, K};
use crate::legacy::Res;

/// Tolerance of the matching of fragments at unit resolution (the Q1/Q3 window), Da.
pub const TOL_LR: f64 = 0.5;

/// The consensus spectrum of a pool: m/z rounded to 0.1, the peaks within `TOL_LR` of a stronger one merged into it (intensity-weighted m/z,
/// summed intensity), then `prepare` (precursor region, weak peaks, `K` strongest, sqrt, unit norm). `None` without fragments.
pub fn consensus(frags: &[(f64, f64)], precursor: f64) -> Option<Spec> {
    let mut left: Peaks = frags
        .iter()
        .map(|f| ((f.0 * 10.0).round() / 10.0, f.1))
        .filter(|f| f.1 > 0.0)
        .collect();
    small(&mut left, |a, b| b.1.total_cmp(&a.1));
    let (mut mz, mut inten) = (Vec::new(), Vec::new());
    while !left.is_empty() {
        let top = left[0].0;
        let (near, rest): (Peaks, Peaks) =
            left.into_iter().partition(|p| (p.0 - top).abs() <= TOL_LR);
        let w: f64 = near.iter().map(|p| p.1).sum();
        mz.push(near.iter().map(|p| p.0 * p.1).sum::<f64>() / w);
        inten.push(w);
        left = rest;
    }
    let s = prepare(&mz, &inten, precursor);
    (!s.0.is_empty()).then_some(s)
}

/// A consensus spectrum with its precursor.
type Consensus = (Spec, f64);
type Peaks = Vec<(f64, f64)>;

/// The consensus spectrum and precursor of the pool of `e` (`None` without scans).
fn spectrum<R: ReadAt>(
    d: &LrDig,
    e: &Entry,
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<Option<Consensus>> {
    let p: Pool = pool(d, e, K, open)?;
    let Some(prec) = p.precursor else {
        return Ok(None);
    };
    let frags: Vec<(f64, f64)> = p.frags.iter().map(|f| (f.mz, f.rel)).collect();
    Ok(consensus(&frags, prec).map(|s| (s, prec)))
}

/// The similarity of every row with MS2 to the parent (the first entry), kept in `Links::sims`; empty when the parent has no MS2.
pub fn compute<R: ReadAt>(
    d: &LrDig,
    l: &Links,
    open: &mut dyn FnMut(usize) -> Res<Index<R>>,
) -> Res<Vec<(i64, Components)>> {
    let mut out = Vec::new();
    let Some(parent) = d.entries.first().filter(|_| l.parent_has_ms2) else {
        return Ok(out);
    };
    let Some((ps, pp)) = spectrum(d, parent, open)? else {
        return Ok(out);
    };
    for e in d
        .entries
        .iter()
        .filter(|e| e.id != parent.id && l.of(e.id).next().is_some())
    {
        if let Some((s, p)) = spectrum(d, e, open)? {
            out.push((e.id, pair_components_tol(&ps, pp, &s, p, TOL_LR)));
        }
    }
    Ok(out)
}
