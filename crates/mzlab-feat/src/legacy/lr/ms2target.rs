//! MS2 targets (`docs/studi/TABUI_MS2_LR.md` §6): a student acquires an MS2 on an ion because the full scan showed it, so a peak of the trace of an
//! experiment that no row of the Dig got (`no_row`, `no_row_at_rt`) is looked for in the aligned table; the strongest feature there at its precursor
//! and time becomes a row (an unexpected ion, `ms2_target`) with the criteria and the score of the other rows but without the filters of the Dig
//! (`max_unexpected`, criteria), and the matches are made again. A peak with no feature in the table is `no_feature`.

use super::cond::GensBy;
use super::ms2link::{Exp, Links};
use super::rows::{best_area, entry, is_treated};
use super::{evaluate, finish_one, isf, Entry, LrDig};
use crate::aligned::Aligned;
use crate::legacy::Res;
use crate::sortc::SortC;
use mzlab_core::idxread::{Index, ReadAt};

/// Id of the first row made from an MS2 (the unexpected ions of the Dig start at 100001).
pub const FIRST_ID: i64 = 200_001;

/// What the table has at the precursor and the time of a peak.
pub enum Near {
    /// No feature there.
    None,
    /// The strongest feature is already a row of the Dig.
    Row,
    /// The strongest feature, row of the table.
    Feature(usize),
}

/// Whether `apex` lies between the start and the end of a detected peak of the feature `r`, in any column.
fn in_peak(t: &Aligned, r: usize, apex: f64) -> bool {
    super::rows::cells(t, r)
        .iter()
        .any(|c| c.detected && c.hi > c.lo && apex >= c.lo && apex <= c.hi)
}

/// The feature of the table whose m/z (the one read in the table, nothing corrected) is within the tolerance of the precursor of `e`
/// (`Exp::tol`: half the isolation window of the file, else `PREC_TOL`) and whose chromatographic peak, in one of the files of the series, contains
/// `apex` (start to end of the peak), after `rt_min`, with the largest area in the files of the series.
pub fn near(d: &LrDig, e: &Exp, apex: f64) -> Near {
    let (t, tol) = (&*d.table, e.tol().da);
    let best = (0..t.len())
        .filter(|&r| {
            t.rt[r] >= d.settings.rt_min
                && (t.mz[r] - e.precursor).abs() <= tol
                && in_peak(t, r, apex)
        })
        .map(|r| (r, best_area(t, &d.lc, r, |_| true)))
        .filter(|x| x.1 > 0.0)
        .max_by(|a, b| a.1.total_cmp(&b.1));
    match best {
        None => Near::None,
        Some((r, _)) if d.entries.iter().any(|e| e.row == Some(r)) => Near::Row,
        Some((r, _)) => Near::Feature(r),
    }
}

/// Removes the rows made by an earlier call (and their in-source verdicts).
pub fn clear(d: &mut LrDig) {
    d.entries.retain(|e| !e.ms2_target);
    d.isf.retain(|v| v.0 < FIRST_ID);
}

/// The unmatched peaks that can become a row: their experiment and what the table has there.
fn open_peaks(d: &LrDig, l: &Links) -> Vec<(usize, Near)> {
    l.unmatched
        .iter()
        .filter(|u| matches!(u.reason, "no_row" | "no_row_at_rt"))
        .filter_map(|u| {
            let x = u.exp?;
            let (e, p) = (l.exps.get(x)?, l.exps.get(x)?.peaks.get(u.peak?)?);
            Some((x, near(d, e, p.apex - e.drift)))
        })
        .collect()
}

/// Makes the rows of the peaks that have a feature in the table (one per feature) and returns `(experiment, id)` of the one that made each.
pub fn make(d: &mut LrDig, l: &Links) -> Vec<(usize, i64)> {
    let mut want: Vec<(usize, usize)> = Vec::new();
    for (x, n) in open_peaks(d, l) {
        if let Near::Feature(r) = n {
            if !want.iter().any(|w| w.1 == r) {
                want.push((x, r));
            }
        }
    }
    want.sortc_by_key(|w| w.1);
    let t = d.table.clone();
    let off = d
        .offset
        .as_ref()
        .filter(|o| o.applied)
        .map_or(0.0, |o| o.offset);
    let mut made = Vec::new();
    for (i, (x, r)) in want.iter().enumerate() {
        let id = FIRST_ID + i as i64;
        let p = l.exps[*x].precursor;
        let name = format!("MS2 target m/z {:.1} @ {:.2} min", p, t.rt[*r]);
        let mut e = entry(id, true, name, t.mz[*r] - off, t.mz[*r], Some(*r));
        e.area = best_area(&t, &d.lc, *r, is_treated);
        e.ms2_target = true;
        evaluate(&mut e, &t, &d.lc, d.settings.persist());
        d.entries.push(e);
        made.push((*x, id));
    }
    finish(d);
    made
}

/// Isotope check and kinetic generation of the new rows.
fn finish(d: &mut LrDig) {
    let parent = d
        .entries
        .first()
        .filter(|p| p.cells.len() == d.lc.len())
        .map(|p| p.cells.as_slice());
    let gens = GensBy::new(&d.lc, parent, d.settings.groups.reference.as_deref());
    let snapshot: Vec<Entry> = d.entries.clone();
    let (lc, s) = (&d.lc, &d.settings);
    for e in d.entries.iter_mut().filter(|e| e.ms2_target) {
        finish_one(e, &snapshot, lc, s, &gens);
    }
}

/// The in-source verdicts of the new rows, when the page asked for them for the others.
pub fn verdicts<R: ReadAt>(d: &mut LrDig, open: &mut dyn FnMut(usize) -> Res<Index<R>>) -> Res<()> {
    d.isf = isf::evaluate(d, open)?;
    Ok(())
}

/// Marks what the new rows came from, and the peaks that have no feature in the table.
pub fn finish_links(d: &LrDig, l: &mut Links, made: &[(usize, i64)]) {
    for k in l.links.iter_mut() {
        if made.contains(&(k.exp, k.row)) {
            k.why += "; row made from this MS2";
        }
    }
    for i in 0..l.unmatched.len() {
        let u = &l.unmatched[i];
        if !matches!(u.reason, "no_row" | "no_row_at_rt") {
            continue;
        }
        let (Some(x), Some(k)) = (u.exp, u.peak) else {
            continue;
        };
        let (e, p) = (&l.exps[x], &l.exps[x].peaks[k]);
        if matches!(near(d, e, p.apex - e.drift), Near::None) {
            let why = format!(
                "no feature in the table at m/z {:.2} and {:.2} min",
                e.precursor, p.apex
            );
            let u = &mut l.unmatched[i];
            (u.reason, u.why) = ("no_feature", why);
        }
    }
}
