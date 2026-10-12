//! `mzlab/differenza.py` (traces): sample minus reference, on the times of the sample. The reference is its local maximum within ± `rttol`
//! minutes, never lower than its interpolation (`np.interp`); where the sample lies outside the time range of the reference the answer is
//! NaN (no data, not 0). Numbers only, never a verdict.

use crate::stats::psum;

/// `np.interp(x, xr, yr)` of a non-empty, ascending `xr`.
fn interp(x: f64, xr: &[f64], yr: &[f64]) -> f64 {
    let n = xr.len();
    if x < xr[0] {
        return yr[0];
    }
    if x >= xr[n - 1] {
        return yr[n - 1];
    }
    let j = (xr.partition_point(|v| *v <= x) - 1).min(n - 2);
    let slope = (yr[j + 1] - yr[j]) / (xr[j + 1] - xr[j]);
    let mut r = slope * (x - xr[j]) + yr[j];
    if r.is_nan() {
        r = slope * (x - xr[j + 1]) + yr[j + 1];
        if r.is_nan() && yr[j] == yr[j + 1] {
            r = yr[j];
        }
    }
    r
}

/// `local_max`: the reference at the times `x`.
pub fn local_max(x: &[f64], xr: &[f64], yr: &[f64], tol: f64) -> Vec<f64> {
    if xr.is_empty() {
        return vec![f64::NAN; x.len()];
    }
    let mut out: Vec<f64> = x.iter().map(|&t| interp(t, xr, yr)).collect();
    if tol > 0.0 && xr.len() > 1 {
        for (i, &t) in x.iter().enumerate() {
            let lo = xr.partition_point(|v| *v < t - tol);
            let hi = xr.partition_point(|v| *v <= t + tol);
            if hi > lo {
                let m = yr[lo..hi].iter().copied().fold(f64::NEG_INFINITY, f64::max);
                if m > out[i] {
                    out[i] = m;
                }
            }
        }
    }
    for (i, &t) in x.iter().enumerate() {
        if t < xr[0] || t > xr[xr.len() - 1] {
            out[i] = f64::NAN;
        }
    }
    out
}

/// `subtract_trace`: `y − factor · reference`; `clip` turns the negatives into 0 (a NaN stays NaN).
pub fn subtract_trace(x: &[f64], y: &[f64], xr: &[f64], yr: &[f64], factor: f64, rttol: f64, clip: bool) -> Vec<f64> {
    let r = local_max(x, xr, yr, rttol);
    y.iter().zip(&r).map(|(a, b)| a - factor * b).map(|d| if clip && d < 0.0 { 0.0 } else { d }).collect()
}

/// `area`: the trapezoid area of a trace (numpy's pairwise sum of the strips).
pub fn area(x: &[f64], y: &[f64]) -> f64 {
    if x.len() < 2 {
        return 0.0;
    }
    let strips: Vec<f64> = (1..x.len()).map(|i| (x[i] - x[i - 1]) * (y[i] + y[i - 1]) / 2.0).collect();
    psum(&strips)
}

/// `norm_factor`: 1 without normalization or when one of the two areas is not positive, else sample / reference.
pub fn norm_factor(normalize: bool, sample: f64, reference: f64) -> f64 {
    if !normalize || !(sample > 0.0 && reference > 0.0) {
        return 1.0;
    }
    sample / reference
}

/// `_diff_cut`: the difference at the times of the sample, without the times that have no reference (NaN), as (rt, y).
pub fn cut(rt: &[f64], d: &[f64]) -> (Vec<f64>, Vec<f64>) {
    rt.iter().zip(d).filter(|(_, v)| !v.is_nan()).map(|(t, v)| (*t, *v)).unzip()
}
