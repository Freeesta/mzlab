//! Apex time with its uncertainty, FWHM and reliability of a detected peak: the part of `ionfamily.detect_peaks` and
//! `ionfamily._parabola_apex` that `peaks::Peak` does not carry. The Python engine is the reference.

use crate::peaks::{Peak, MIN_SCANS_RELIABLE};
use crate::signal::{median, noise_level, solve};

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

/// `_parabola_apex(rt, y, i)`: `(t_apex, sigma_t, height)` of the quadratic fit over `i ± 2` (delta method).
pub fn parabola_apex(rt: &[f64], y: &[f64], i: usize) -> (f64, f64, f64) {
    let n = y.len();
    let (a, b) = (i.saturating_sub(2), (i + 3).min(n));
    if b - a < 3 {
        let mean_dt = if n > 1 {
            (rt[n - 1] - rt[0]) / (n - 1) as f64
        } else {
            0.0
        };
        return (rt[i], mean_dt, y[i]);
    }
    let xs: Vec<f64> = rt[a..b].iter().map(|t| t - rt[i]).collect();
    let mut ata = vec![0.0; 9];
    let mut aty = vec![0.0; 3];
    for (x, &yy) in xs.iter().zip(&y[a..b]) {
        for j in 0..3 {
            aty[j] += x.powi(j as i32) * yy;
            for k in 0..3 {
                ata[j * 3 + k] += x.powi((j + k) as i32);
            }
        }
    }
    let dt = median_dt(rt);
    let coef = match solve(ata.clone(), aty, 3) {
        Some(c) => c,
        None => return (rt[i], dt / 2.0, y[i]),
    };
    let (c0, c1, c2) = (coef[0], coef[1], coef[2]);
    if c2 >= 0.0 {
        return (rt[i], dt / 2.0, y[i]);
    }
    let xa = (-c1 / (2.0 * c2)).clamp(xs[0], xs[xs.len() - 1]);
    let h = c0 + c1 * xa + c2 * xa * xa;
    let rss: f64 = xs
        .iter()
        .zip(&y[a..b])
        .map(|(x, &yy)| (yy - (c0 + c1 * x + c2 * x * x)).powi(2))
        .sum();
    let s2 = (rss / (xs.len() as f64 - 3.0).max(1.0))
        .max(noise_level(y).powi(2))
        .max(1e-12);
    let g = [0.0, -1.0 / (2.0 * c2), c1 / (2.0 * c2 * c2)];
    // var = g' (s2 (A'A)^-1) g: solve (A'A) v = g, then g . v
    let var = match solve(ata, g.to_vec(), 3) {
        Some(v) => s2 * (0..3).map(|j| g[j] * v[j]).sum::<f64>(),
        None => 0.0,
    };
    (rt[i] + xa, var.max(0.0).sqrt(), h)
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

/// Apex time and sigma, FWHM and reliability of `pk`, found on the smoothed trace `ys` (`Detection::ys`).
pub fn extras(rt: &[f64], ys: &[f64], pk: &Peak) -> Extras {
    let w = fwhm(rt, ys, pk.apex_i);
    let dt = median_dt(rt);
    let scans = if dt > 0.0 { w / dt } else { 0.0 };
    let (apex_rt, apex_sigma, _) = parabola_apex(rt, ys, pk.apex_i);
    Extras {
        apex_rt,
        apex_sigma,
        fwhm_min: w,
        fwhm_scans: scans,
        reliable: pk.n_scans >= MIN_SCANS_RELIABLE && scans >= 3.0,
    }
}
