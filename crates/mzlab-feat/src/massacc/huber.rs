//! `robust = huber`: Huber's M-estimate by iteratively reweighted least squares. A residual beyond `K` robust standard deviations counts as `K` of them, so a reference ion
//! overlapped by an isobaric analyte pulls the fit by a bounded amount. The scale is the median absolute deviation, re-estimated at each step.

use super::fit::{Fit, Model, Obs};
use super::ols::{mad_scale, residuals, wls, SIGMA_FLOOR};

/// Tuning constant of the Huber weight, in robust standard deviations (95 % efficiency on normal data).
const K: f64 = 1.345;
/// A residual beyond this many robust standard deviations marks its observation as an outlier.
const OUTLIER: f64 = 3.0;
const MAX_ITER: usize = 100;
const CONVERGED: f64 = 1e-10;

pub(super) fn fit(model: Model, obs: &[Obs]) -> Option<Fit> {
    let mut w: Vec<f64> = obs.iter().map(|o| o.w).collect();
    let mut coef = wls(model, obs, &w)?;
    for _ in 0..MAX_ITER {
        let r = residuals(model, &coef, obs);
        let cut = K * mad_scale(&r).max(SIGMA_FLOOR);
        for ((wi, o), x) in w.iter_mut().zip(obs).zip(&r) {
            *wi = o.w * if x.abs() > cut { cut / x.abs() } else { 1.0 };
        }
        let next = wls(model, obs, &w)?;
        let step = (0..3)
            .map(|i| (next[i] - coef[i]).abs())
            .fold(0.0, f64::max);
        coef = next;
        if step < CONVERGED {
            break;
        }
    }
    let r = residuals(model, &coef, obs);
    let scale = mad_scale(&r);
    let limit = OUTLIER * scale.max(SIGMA_FLOOR);
    Some(Fit {
        coef,
        inlier: r.iter().map(|x| x.abs() <= limit).collect(),
        scale,
        model,
    })
}
