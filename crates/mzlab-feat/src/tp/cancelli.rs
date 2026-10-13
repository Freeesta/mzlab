//! The gates of the view of the products (see `tp`): formed, not in source, explained. Each one writes the status and the notes of the rows it touches.

use mzlab_core::error::{Error, Result};

use super::*;
use crate::aligned::CELL_ABSENT;
use crate::evid::match_hypotheses;
use crate::hypo::{SRC_CLEAVAGE, SRC_IMPORTED, SRC_NETWORK, SRC_RULE};

/// Neutral losses by which an in-source fragment can differ from an accepted product (H2O, NH3, CO, 2 H2O, CO2, CH3OH, C2H4, HCOOH), Da.
const LOSSES: [f64; 8] = [
    18.010565, 17.026549, 27.994915, 36.02113, 43.989829, 32.026215, 28.0313, 46.005479,
];

/// Window of the parent in the table: 5 ppm and 0.3 min, the nearest in m/z.
pub(super) fn parent_row(inp: &TpInputs) -> Option<usize> {
    let (mz, rt) = inp.parent;
    let t = inp.aligned;
    (0..t.len())
        .filter(|&r| (t.mz[r] - mz).abs() <= mz * 5e-6 && (t.rt[r] - rt).abs() <= 0.3)
        .min_by(|&a, &b| (t.mz[a] - mz).abs().total_cmp(&(t.mz[b] - mz).abs()))
}

/// The area of the cell when the file has a peak there (not absent, finite, above zero).
fn measured(t: &Aligned, row: usize, file: usize) -> Option<f64> {
    let c = row * t.n_files + file;
    (t.state[c] != CELL_ABSENT && t.area[c].is_finite() && t.area[c] > 0.0).then_some(t.area[c])
}

/// Largest area of a row over some files (0 where none has a peak).
fn peak(t: &Aligned, row: usize, files: &[usize]) -> f64 {
    files
        .iter()
        .filter_map(|&k| measured(t, row, k))
        .fold(0.0, f64::max)
}

/// Gate 1: the treated samples beat the controls by `ratio`. A row that is already in the t0 and passes anyway is a product that also forms: it stays, noted.
pub(super) fn formed(inp: &TpInputs, parent: usize, b: &mut TpBlock) -> Result<()> {
    let m = inp.minutes;
    let treated: Vec<usize> = (0..m.len()).filter(|&k| m[k] > 0.0).collect();
    let t0: Vec<usize> = (0..m.len()).filter(|&k| m[k] == 0.0).collect();
    let others: Vec<usize> = (0..m.len()).filter(|&k| m[k].is_nan()).collect();
    if treated.is_empty() || t0.len() + others.len() == 0 || m.len() != inp.aligned.n_files {
        return Err(Error::new("error.tpNoControls"));
    }
    for r in 0..inp.aligned.len() {
        if r == parent {
            b.status[r] = ST_PARENT;
            continue;
        }
        let (high, from_t0, from_others) = (
            peak(inp.aligned, r, &treated),
            peak(inp.aligned, r, &t0),
            peak(inp.aligned, r, &others),
        );
        if high <= 0.0 || high < inp.params.ratio * from_t0.max(from_others) {
            b.status[r] = ST_CONTROL;
        } else if from_t0 > 0.0 {
            b.notes[r] |= NOTE_T0_PRESENT;
        }
    }
    Ok(())
}

/// Times (files, in time order) at which both rows have a peak, with the ratio of their areas `a / p`.
fn ratios(inp: &TpInputs, a: usize, p: usize) -> Vec<f64> {
    let mut files: Vec<usize> = (0..inp.minutes.len())
        .filter(|&k| inp.minutes[k] > 0.0)
        .collect();
    crate::sortc::by(&mut files, |&x, &y| {
        inp.minutes[x].total_cmp(&inp.minutes[y])
    });
    files
        .into_iter()
        .filter_map(|k| Some(measured(inp.aligned, a, k)? / measured(inp.aligned, p, k)?))
        .collect()
}

/// Whether the ratios are constant: enough of them and a coefficient of variation not above `isf_cv`.
fn constant(inp: &TpInputs, r: &[f64]) -> bool {
    if r.len() < inp.params.isf_points {
        return false;
    }
    let m = r.iter().sum::<f64>() / r.len() as f64;
    (r.iter().map(|x| (x - m).powi(2)).sum::<f64>() / r.len() as f64).sqrt() / m
        <= inp.params.isf_cv
}

/// The m/z is among the fragments of the MS2 of the parent (true when the parent has none: it is not contradicted).
fn in_ms2(inp: &TpInputs, mz: f64) -> bool {
    inp.fragments.is_none_or(|f| {
        f.iter()
            .any(|&x| (x - mz).abs() <= mz * inp.params.ppm * 1e-6)
    })
}

/// Gate 2, against the parent: a lighter ion that coelutes with it, whose ratio to it is constant along the series and that is in its MS2, is an
/// in-source fragment. With a ratio that is not constant the ion survives the parent or forms later: it stays, noted when the MS2 of the parent holds it.
/// Then, against the accepted products, the same for the ions that differ from one by a usual neutral loss.
pub(super) fn not_in_source(inp: &TpInputs, parent: usize, b: &mut TpBlock) {
    let t = inp.aligned;
    let near = |a: usize, p: usize| (t.rt[a] - t.rt[p]).abs() <= inp.params.isf_rt;
    for r in 0..t.len() {
        if b.status[r] != ST_TP || t.mz[r] >= t.mz[parent] - 1.0 || !near(r, parent) {
            continue;
        }
        let rat = ratios(inp, r, parent);
        if constant(inp, &rat) && in_ms2(inp, t.mz[r]) {
            b.status[r] = ST_ISF;
        } else if inp.fragments.is_some()
            && rat.len() >= inp.params.isf_points
            && in_ms2(inp, t.mz[r])
        {
            b.notes[r] |= NOTE_ISF_COELUTES;
        }
    }
    let mut accepted: Vec<usize> = (0..t.len()).filter(|&r| b.status[r] == ST_TP).collect();
    crate::sortc::by(&mut accepted, |&x, &y| t.mz[x].total_cmp(&t.mz[y]));
    let mut isf: Vec<usize> = Vec::new();
    for &r in &accepted {
        let found = LOSSES.iter().any(|&loss| {
            let want = t.mz[r] + loss;
            let tol = want * inp.params.ppm * 1e-6;
            let from = accepted.partition_point(|&p| t.mz[p] < want - tol);
            accepted[from..]
                .iter()
                .take_while(|&&p| t.mz[p] <= want + tol)
                .any(|&p| near(r, p) && constant(inp, &ratios(inp, r, p)))
        });
        if found {
            isf.push(r);
        }
    }
    for r in isf {
        b.status[r] = ST_ISF;
        b.notes[r] |= NOTE_ISF_OF_PRODUCT;
    }
}

/// Gate 3: a rule, a cleavage or an imported hypothesis covers the ion, or its MS2 is related to the parent's. Notes the MS2 of low purity.
pub(super) fn explained(inp: &TpInputs, b: &mut TpBlock) {
    let matched = match_hypotheses(inp.hyps, inp.aligned);
    for r in 0..inp.aligned.len() {
        if let Some(h) = matched[r] {
            let s = inp.hyps[h].sources;
            b.via[r] |= [
                (SRC_RULE, VIA_RULE),
                (SRC_CLEAVAGE, VIA_CLEAVAGE),
                (SRC_IMPORTED, VIA_IMPORTED),
                (SRC_NETWORK, VIA_NETWORK),
            ]
            .iter()
            .filter(|(src, _)| s & src != 0)
            .fold(0, |v, (_, bit)| v | bit);
        }
        if crate::evid::rowproofs::sign_of(&inp.signs[r], crate::evid::Kind::Network)
            == crate::evid::rowproofs::FOR
        {
            b.via[r] |= VIA_MS2;
        }
        let purity =
            (!inp.aligned.meta.is_empty()).then(|| f64::from(inp.aligned.meta.ms2_purity[r]));
        if purity.is_some_and(|p| (0.0..inp.params.purity_min).contains(&p)) {
            b.notes[r] |= NOTE_CHIMERIC_MS2;
        }
        if b.status[r] == ST_TP && b.via[r] == 0 {
            b.status[r] = ST_UNEXPLAINED;
        }
    }
}
