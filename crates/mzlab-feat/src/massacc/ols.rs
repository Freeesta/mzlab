//! Weighted least squares on the terms of a model (3x3 normal equations), the plain fit of `robust = none` and the step inside `huber` and `ransac`; with the median and the robust scale they share.

use super::fit::{Fit, Model, Obs};

/// A scale below this (ppm) is the noise of the arithmetic, not a measured spread: it is the floor of the cut-offs.
pub(super) const SIGMA_FLOOR: f64 = 0.05;

/// Median of the values (sorted by `total_cmp`; 0 for none).
pub(super) fn median(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s = v.to_vec();
    crate::sortc::by(&mut s, f64::total_cmp);
    let n = s.len();
    if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    }
}

/// The standard deviation a normal sample would have given this median absolute deviation about its median.
pub(super) fn mad_scale(r: &[f64]) -> f64 {
    let m = median(r);
    1.4826 * median(&r.iter().map(|x| (x - m).abs()).collect::<Vec<_>>())
}

/// Coefficients of `model` on `obs` with the weights `w` (one per observation); `None` when the terms are not independent (all the same m/z for a line).
pub(super) fn wls(model: Model, obs: &[Obs], w: &[f64]) -> Option<[f64; 3]> {
    let p = model.n_params();
    if p == 0 || obs.len() < p {
        return None;
    }
    let (mut a, mut b) = ([[0.0f64; 3]; 3], [0.0f64; 3]);
    for (o, &wi) in obs.iter().zip(w) {
        let x = model.basis(o.mz, o.rt);
        for i in 0..p {
            for j in 0..p {
                a[i][j] += wi * x[i] * x[j];
            }
            b[i] += wi * x[i] * o.err;
        }
    }
    // Gauss elimination with partial pivoting; a pivot that is small against the diagonal means dependent terms
    let scale: f64 = (0..p).map(|i| a[i][i]).fold(0.0, f64::max);
    for c in 0..p {
        let pivot = (c..p).max_by(|&i, &j| a[i][c].abs().total_cmp(&a[j][c].abs()))?;
        if a[pivot][c].abs() <= 1e-12 * scale {
            return None;
        }
        a.swap(c, pivot);
        b.swap(c, pivot);
        for r in c + 1..p {
            let f = a[r][c] / a[c][c];
            let top = a[c];
            for (cell, above) in a[r][c..p].iter_mut().zip(&top[c..p]) {
                *cell -= f * above;
            }
            b[r] -= f * b[c];
        }
    }
    let mut x = [0.0f64; 3];
    for i in (0..p).rev() {
        let s: f64 = (i + 1..p).map(|k| a[i][k] * x[k]).sum();
        x[i] = (b[i] - s) / a[i][i];
    }
    Some(x)
}

/// The residuals of `obs` under `coef`.
pub(super) fn residuals(model: Model, coef: &[f64; 3], obs: &[Obs]) -> Vec<f64> {
    obs.iter()
        .map(|o| o.err - model.predict(coef, o.mz, o.rt))
        .collect()
}

/// `robust = none`: the weighted least squares fit of every observation; the scale is the sample standard deviation of the residuals.
pub(super) fn fit(model: Model, obs: &[Obs]) -> Option<Fit> {
    let w: Vec<f64> = obs.iter().map(|o| o.w).collect();
    let coef = wls(model, obs, &w)?;
    let r = residuals(model, &coef, obs);
    let (sw, sr): (f64, f64) = obs
        .iter()
        .zip(&r)
        .fold((0.0, 0.0), |(a, b), (o, x)| (a + o.w, b + o.w * x * x));
    let dof = obs.len() as f64 - model.n_params() as f64;
    let scale = if dof > 0.0 && sw > 0.0 {
        (sr / sw * obs.len() as f64 / dof).sqrt()
    } else {
        0.0
    };
    Some(Fit {
        coef,
        inlier: vec![true; obs.len()],
        scale,
        model,
    })
}
