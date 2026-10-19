//! arPLS baseline correction of a chromatographic trace (feature `untargeted`).
//!
//! Rewritten from scratch after the algorithm of mzmine 4.10.90 `featdet_baselinecorrection/arpls/` (`ArplsBaseline.java`, `PentadiagonalSolver.java`,
//! `ArplsBaselineCorrector.java`, with `AbstractResolverBaselineCorrector.java` and `BaselineDataBuffer.java` for the option «Exclude peaks»; MIT,
//! (c) 2004-2022 The mzmine Development Team); no code copied. Paper: Baek S.-J. et al., Analyst 140:250 (2015), doi:10.1039/C4AN01061B.
//! Used by the untargeted pipeline before the resolver when the recipe has a `"baseline"` object (off by default); the independent reference is `tools/arpls_riferimento.py`.
//!
//! The baseline `z` solves `(W + lambda D2'D2) z = W y` with the weights `W` of the previous round (logistic in the residuals below the baseline);
//! the system is pentadiagonal and symmetric, so one LDL' factorisation per round is enough. [`correct`] returns `max(y - z, 0)`.

use super::resolve::{regions, MinParams};

/// Stop criterion on the relative change of the weights (fixed in mzmine).
const RATIO: f64 = 1e-6;

/// LDL' factorisation of a symmetric pentadiagonal matrix (diagonal `a`, first off-diagonal `b`, second `c`), solved in place into `out`.
fn solve_penta(a: &[f64], b: &[f64], c: &[f64], rhs: &[f64], out: &mut [f64]) {
    let n = a.len();
    let (mut d, mut al, mut be) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    for i in 0..n {
        let dm1 = if i >= 1 { d[i - 1] } else { 0.0 };
        let dm2 = if i >= 2 { d[i - 2] } else { 0.0 };
        be[i] = if i >= 2 { c[i - 2] / dm2 } else { 0.0 };
        if i >= 1 {
            let corr = if i >= 2 { be[i] * al[i - 1] * dm2 } else { 0.0 };
            al[i] = (b[i - 1] - corr) / dm1;
        }
        let mut di = a[i];
        if i >= 1 {
            di -= al[i] * al[i] * dm1;
        }
        if i >= 2 {
            di -= be[i] * be[i] * dm2;
        }
        d[i] = di;
    }
    for i in 0..n {
        let mut v = rhs[i];
        if i >= 1 {
            v -= al[i] * out[i - 1];
        }
        if i >= 2 {
            v -= be[i] * out[i - 2];
        }
        out[i] = v;
    }
    for i in 0..n {
        out[i] /= d[i];
    }
    for i in (0..n).rev() {
        let mut v = out[i];
        if i + 1 < n {
            v -= al[i + 1] * out[i + 1];
        }
        if i + 2 < n {
            v -= be[i + 2] * out[i + 2];
        }
        out[i] = v;
    }
}

/// The band of `D2'D2` (stencil [1, -2, 1]) for `n >= 3`: diagonal, first and second off-diagonal.
fn penalty(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let (mut d0, mut d1, mut d2) = (vec![0.0; n], vec![0.0; n - 1], vec![0.0; n - 2]);
    for k in 0..n - 2 {
        d0[k] += 1.0;
        d0[k + 1] += 4.0;
        d0[k + 2] += 1.0;
        d1[k] -= 2.0;
        d1[k + 1] -= 2.0;
        d2[k] += 1.0;
    }
    (d0, d1, d2)
}

/// Mean and POPULATION standard deviation (divisor: the number of values) of the residuals under the baseline; `None` when there are none or the
/// spread is 0 (the loop then stops and keeps the baseline).
fn negative_stats(y: &[f64], z: &[f64]) -> Option<(f64, f64)> {
    let neg: Vec<f64> = y
        .iter()
        .zip(z)
        .map(|(a, b)| a - b)
        .filter(|d| *d < 0.0)
        .collect();
    if neg.is_empty() {
        return None;
    }
    let mean = neg.iter().sum::<f64>() / neg.len() as f64;
    let sq: f64 = neg.iter().map(|d| (d - mean) * (d - mean)).sum();
    let std = (sq / neg.len() as f64).sqrt();
    (std != 0.0).then_some((mean, std))
}

/// The arPLS baseline of `y`. `lambda` is the smoothness (mzmine: 1e5, range 1..1e12), `max_iter` the cap on the rounds (50, range 1..500).
/// Fewer than 3 points give a baseline of zeros.
pub fn baseline(y: &[f64], lambda: f64, max_iter: usize) -> Vec<f64> {
    let n = y.len();
    let mut z = vec![0.0; n];
    if n < 3 {
        return z;
    }
    let (p0, p1, p2) = penalty(n);
    let off1: Vec<f64> = p1.iter().map(|v| lambda * v).collect();
    let off2: Vec<f64> = p2.iter().map(|v| lambda * v).collect();
    let mut w = vec![1.0; n];
    for _ in 0..max_iter {
        let diag: Vec<f64> = (0..n).map(|i| w[i] + lambda * p0[i]).collect();
        let rhs: Vec<f64> = (0..n).map(|i| w[i] * y[i]).collect();
        solve_penta(&diag, &off1, &off2, &rhs, &mut z);
        let Some((mean, std)) = negative_stats(y, &z) else {
            return z;
        };
        let cutoff = 2.0 * std - mean;
        let (mut num, mut den) = (0.0, 0.0);
        for i in 0..n {
            let wt = 1.0 / (1.0 + (2.0 * (y[i] - z[i] - cutoff) / std).exp());
            num += (wt - w[i]) * (wt - w[i]);
            den += w[i] * w[i];
            w[i] = wt;
        }
        let change = if den > 0.0 {
            num.sqrt() / den.sqrt()
        } else {
            0.0
        };
        if change < RATIO {
            break;
        }
    }
    z
}

/// `max(y - baseline, 0)`: the corrected trace (the baseline may dip below zero, the result may not).
pub fn correct(y: &[f64], lambda: f64, max_iter: usize) -> Vec<f64> {
    let z = baseline(y, lambda, max_iter);
    y.iter().zip(&z).map(|(v, b)| (v - b).max(0.0)).collect()
}

/// Parameters of the resolver that finds the peaks to bridge for «Exclude peaks» (fixed in mzmine 4.10: threshold 0.5, search 0.04 min, relative
/// height 0.005, absolute height 1, ratio 2.5, duration 0-50 min, 5 points).
fn bridge_params() -> MinParams {
    MinParams {
        chrom_threshold: 0.5,
        search_rt: 0.04,
        min_ratio: 2.5,
        min_abs_height: 1.0,
        min_rel_height: 0.005,
        min_points: 5,
        duration: (0.0, 50.0),
        // the widening is done by this module on the regions (`bridge`)
        adjust_edges: false,
    }
}

/// Replaces `[s, e]` by the straight line between the points just outside it (flat at an edge, unchanged when there is no anchor at all).
fn bridge(x: &[f64], y: &mut [f64], orig: &[f64], s: usize, e: usize) {
    let n = x.len();
    match (s.checked_sub(1), (e + 1 < n).then_some(e + 1)) {
        (Some(l), Some(r)) => {
            let slope = (orig[r] - orig[l]) / (x[r] - x[l]);
            for i in l + 1..r {
                y[i] = orig[l] + slope * (x[i] - x[l]);
            }
        }
        (None, Some(r)) => y[..r].fill(orig[r]),
        (Some(l), None) => y[l + 1..].fill(orig[l]),
        (None, None) => {}
    }
}

/// Option «Exclude peaks»: the peaks are found on a first corrected copy, bridged by lines in the original, the baseline is fitted on that bridged
/// trace and subtracted from the ORIGINAL `y`. `x` is the RT axis in minutes (strictly increasing). The resolver is that of `resolve.rs`; the borders
/// are widened by one scan up to a zero next to them as `adjustStartAndEnd` of mzmine does for each resolved range.
pub fn correct_excluding_peaks(x: &[f64], y: &[f64], lambda: f64, max_iter: usize) -> Vec<f64> {
    let mut first = correct(y, lambda, max_iter);
    let found = regions(x, &mut first, &bridge_params());
    let last = y.len().saturating_sub(1);
    let mut bridged = y.to_vec();
    for (s, e) in found {
        let s2 = if first[s] != 0.0 && first[s.saturating_sub(1)] == 0.0 {
            s.saturating_sub(1)
        } else {
            s
        };
        let e2 = if first[e] != 0.0 && first[(e + 1).min(last)] == 0.0 {
            e + 1
        } else {
            e
        };
        bridge(x, &mut bridged, y, s2, e2);
    }
    let z = baseline(&bridged, lambda, max_iter);
    y.iter().zip(&z).map(|(v, b)| (v - b).max(0.0)).collect()
}
