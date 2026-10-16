//! The feature of a row as the MS2 rules see it: its m/z (the one read in the table), the rows a precursor is compatible with, the start and the end of
//! its peak in the time of an MS2 file, and the time it is expected at.

use super::exp::Exp;
use super::rules::median;
use crate::legacy::lr::ms2assoc;
use crate::legacy::lr::{Entry, LrDig};

/// The m/z of the feature of a row: the one read in the table (the centre of the peak), else the extraction m/z. The precursor is compared with it
/// and with nothing else (not the calculated m/z, not the m/z corrected by the offset), so a feature never takes a precursor beyond the window.
pub(super) fn feature_mz(r: &Entry) -> f64 {
    r.observed.unwrap_or(r.mz_x)
}

/// The distance of the precursor `p` to the m/z of the feature of the row.
pub(super) fn dmz(r: &Entry, p: f64) -> f64 {
    ms2assoc::delta(p, feature_mz(r)).0
}

/// Rule 2: the rows whose feature m/z is within the tolerance of the precursor of `e` (`ms2assoc::tolerance`: the isolation window of the file, else
/// `PREC_TOL`). A row that is the isotope peak of another is not one.
pub(super) fn compatible<'a>(d: &'a LrDig, e: &Exp) -> Vec<&'a Entry> {
    let tol = e.tol().da;
    d.entries
        .iter()
        .filter(|r| {
            r.ref_rt.is_some()
                && (r.isotope_of.is_none() || r.ms2_target)
                && dmz(r, e.precursor) <= tol
        })
        .collect()
}

/// The peak of the feature of the row, minutes, in the time of the MS2 file: the peak of the file with the same sampling time as the experiment
/// when the row has one there, else the median start and end of its peaks in the series (one wide peak in another file does not widen it), else the reference time of the row +- `rt_tol_min`; moved by the drift.
pub(super) fn bounds(d: &LrDig, r: &Entry, e: &Exp) -> (f64, f64) {
    let ok = |c: &&crate::legacy::lr::labels::Cell| c.detected && c.hi > c.lo;
    let same = e.time.and_then(|t| {
        d.lc.iter()
            .position(|f| f.time.is_some_and(|x| (x - t).abs() < 1e-6))
    });
    let (lo, hi) = match same.and_then(|k| r.cells.get(k)).filter(|c| ok(&c)) {
        Some(c) => (c.lo, c.hi),
        None => {
            let cs: Vec<_> = r.cells.iter().filter(ok).collect();
            if cs.is_empty() {
                let at = r.ref_rt.unwrap_or(0.0);
                (at - d.settings.rt_tol_min, at + d.settings.rt_tol_min)
            } else {
                (
                    median(&mut cs.iter().map(|c| c.lo).collect::<Vec<_>>()),
                    median(&mut cs.iter().map(|c| c.hi).collect::<Vec<_>>()),
                )
            }
        }
    };
    (lo + e.drift, hi + e.drift)
}

/// Time of the row in the full-scan file with the sampling time of the experiment (its apex there), else its reference RT.
pub(super) fn expected(d: &LrDig, r: &Entry, e: &Exp) -> f64 {
    let at = e.time.and_then(|t| {
        d.lc.iter()
            .position(|f| f.time.is_some_and(|x| (x - t).abs() < 1e-6))
    });
    at.and_then(|k| r.cells.get(k))
        .filter(|c| c.detected)
        .and_then(|c| c.apex_rt)
        .or(r.ref_rt)
        .unwrap_or(0.0)
}
