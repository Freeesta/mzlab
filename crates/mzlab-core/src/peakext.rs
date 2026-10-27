//! Apex time with its uncertainty, FWHM and reliability of a detected peak: the part of `ionfamily.detect_peaks` and
//! `ionfamily._parabola_apex` that `peaks::Peak` does not carry. The Python engine is the reference.

use crate::peaks::{quad_fit, Fit, Peak, MIN_SCANS_RELIABLE};
use crate::signal::{median, noise_level, solve3};

/// What `detect_peaks` of Python adds to the peak of `peaks::Peak`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extras {
    pub apex_rt: f64,
    pub apex_sigma: f64,
    pub fwhm_min: f64,
    pub fwhm_scans: f64,
    pub reliable: bool,
}

fn median_dt(rt: &[f64]) -> f64 {
    let d: Vec<f64> = rt.windows(2).map(|w| w[1] - w[0]).collect();
    median(&d)
}

/// What the peaks of one trace share: the median spacing of the scans and the noise of the smoothed trace, computed once per trace.
#[derive(Debug, Clone, Copy)]
pub struct TraceStats {
    pub dt: f64,
    pub noise: f64,
}

impl TraceStats {
    pub fn new(rt: &[f64], ys: &[f64]) -> Self {
        TraceStats {
            dt: median_dt(rt),
            noise: noise_level(ys),
        }
    }
}

/// `_parabola_apex(rt, y, i)`: `(t_apex, sigma_t, height)` of the quadratic fit over `i ± 2` (delta method); `st` is `TraceStats::new(rt, y)`.
pub fn parabola_apex(rt: &[f64], y: &[f64], i: usize, st: &TraceStats) -> (f64, f64, f64) {
    let n = y.len();
    let q = match quad_fit(rt, y, i) {
        Fit::Short => {
            let mean_dt = if n > 1 {
                (rt[n - 1] - rt[0]) / (n - 1) as f64
            } else {
                0.0
            };
            return (rt[i], mean_dt, y[i]);
        }
        Fit::Flat => return (rt[i], st.dt / 2.0, y[i]),
        Fit::Peak(q) => q,
    };
    let [c0, c1, c2] = q.c;
    let a = i.saturating_sub(2);
    let rss: f64 = q.xs[..q.len]
        .iter()
        .zip(&y[a..])
        .map(|(x, &yy)| (yy - (c0 + c1 * x + c2 * x * x)).powi(2))
        .sum();
    let s2 = (rss / (q.len as f64 - 3.0).max(1.0))
        .max(st.noise.powi(2))
        .max(1e-12);
    let g = [0.0, -1.0 / (2.0 * c2), c1 / (2.0 * c2 * c2)];
    // var = g' (s2 (A'A)^-1) g: solve (A'A) v = g, then g . v
    let var = match solve3(q.ata, g) {
        Some(v) => s2 * (0..3).map(|j| g[j] * v[j]).sum::<f64>(),
        None => 0.0,
    };
    (rt[i] + q.xa, var.max(0.0).sqrt(), q.height)
}

/// FWHM (minutes) by linear interpolation at half of the apex of the smoothed trace `ys`.
fn fwhm(rt: &[f64], ys: &[f64], apex: usize) -> f64 {
    let n = ys.len();
    let half = 0.5 * ys[apex];
    let mut li = apex;
    while li > 0 && ys[li] > half {
        li -= 1;
    }
    let mut ri = apex;
    while ri < n - 1 && ys[ri] > half {
        ri += 1;
    }
    let tl = if li < apex && ys[li] <= half {
        rt[li] + (half - ys[li]) / (ys[li + 1] - ys[li]).max(1e-12) * (rt[li + 1] - rt[li])
    } else {
        rt[li]
    };
    let tr = if ri > apex && ys[ri] <= half {
        rt[ri] - (half - ys[ri]) / (ys[ri - 1] - ys[ri]).max(1e-12) * (rt[ri] - rt[ri - 1])
    } else {
        rt[ri]
    };
    (tr - tl).max(0.0)
}

/// Apex time and sigma, FWHM and reliability of `pk`, found on the smoothed trace `ys` (`Detection::ys`); `st` is `TraceStats::new(rt, ys)`, shared by the
/// peaks of the trace.
pub fn extras(rt: &[f64], ys: &[f64], pk: &Peak, st: &TraceStats) -> Extras {
    let w = fwhm(rt, ys, pk.apex_i);
    let scans = if st.dt > 0.0 { w / st.dt } else { 0.0 };
    let (apex_rt, apex_sigma, _) = parabola_apex(rt, ys, pk.apex_i, st);
    Extras {
        apex_rt,
        apex_sigma,
        fwhm_min: w,
        fwhm_scans: scans,
        reliable: pk.n_scans >= MIN_SCANS_RELIABLE && scans >= 3.0,
    }
}

#[cfg(test)]
mod tests;
