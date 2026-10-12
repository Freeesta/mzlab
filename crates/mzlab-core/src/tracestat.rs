//! Small numeric helpers shared by the ion-origin evidence (`similarity`, `ratio`, `kinetics`): the baseline-corrected copy of a trace,
//! peaks with their apex and FWHM, `np.roll`, `np.percentile`, cosine. Ports of the helpers of `ionfamily.py`.

use crate::peakext::{extras, Extras};
use crate::peaks::{detect_full, Params, Peak};
use crate::signal::{asls_baseline, median, noise_level, savgol};
use crate::stats::psum;

/// `ionfamily._prep`: baseline-corrected trace, its smoothed copy and the noise level of the corrected trace.
pub fn prep(y: &[f64]) -> (Vec<f64>, Vec<f64>, f64) {
    let base = if y.len() >= 7 {
        asls_baseline(y, 1e5, 0.01, 10, 160)
    } else {
        vec![0.0; y.len()]
    };
    let yc: Vec<f64> = y.iter().zip(&base).map(|(a, b)| (a - b).max(0.0)).collect();
    let ys = savgol(&yc, 5, 2).into_iter().map(|v| v.max(0.0)).collect();
    let noise = noise_level(&yc);
    (yc, ys, noise)
}

/// A detected peak with the fields `detect_peaks` of Python reports.
#[derive(Debug, Clone)]
pub struct PeakX {
    pub p: Peak,
    pub e: Extras,
}

/// `ionfamily.detect_peaks(rt, y)["peaks"]`, highest first.
pub fn detect(rt: &[f64], y: &[f64]) -> Vec<PeakX> {
    let d = detect_full(rt, y, Params::default());
    d.peaks
        .iter()
        .map(|p| PeakX {
            p: p.clone(),
            e: extras(rt, &d.ys, p),
        })
        .collect()
}

/// `ionfamily._pick_peak`: the peak closest to `near_i` (within `tol_scans`) or the highest one.
pub fn pick_peak(ps: &[PeakX], near_i: Option<usize>, tol_scans: Option<f64>) -> Option<&PeakX> {
    let first = ps.first()?;
    let near = match near_i {
        None => return Some(first),
        Some(n) => n as f64,
    };
    let best = ps.iter().min_by(|a, b| {
        (a.p.apex_i as f64 - near)
            .abs()
            .total_cmp(&(b.p.apex_i as f64 - near).abs())
    })?;
    match tol_scans {
        Some(t) if (best.p.apex_i as f64 - near).abs() > t => None,
        _ => Some(best),
    }
}

/// `np.roll(a, s)`: element `i` moves to `i + s` (circular).
pub fn roll(a: &[f64], s: i64) -> Vec<f64> {
    let n = a.len() as i64;
    (0..n).map(|i| a[(i - s).rem_euclid(n) as usize]).collect()
}

/// `np.percentile(a, q)` (linear interpolation), `q` in 0..=100.
pub fn percentile(a: &[f64], q: f64) -> f64 {
    let mut s = a.to_vec();
    s.sort_by(|x, y| x.total_cmp(y));
    let pos = (s.len() - 1) as f64 * q / 100.0;
    let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
    s[lo] + (s[hi] - s[lo]) * (pos - lo as f64)
}

pub fn mean(a: &[f64]) -> f64 {
    psum(a) / a.len() as f64
}

/// `np.std` (population).
pub fn std(a: &[f64]) -> f64 {
    let m = mean(a);
    let d: Vec<f64> = a.iter().map(|v| (v - m) * (v - m)).collect();
    (psum(&d) / a.len() as f64).sqrt()
}

pub fn median_of(a: &[f64]) -> f64 {
    median(a)
}

/// `ionfamily.cosine`.
pub fn cosine(x: &[f64], y: &[f64]) -> f64 {
    let nx = psum(&x.iter().map(|v| v * v).collect::<Vec<_>>()).sqrt();
    let ny = psum(&y.iter().map(|v| v * v).collect::<Vec<_>>()).sqrt();
    let d = nx * ny;
    if d > 0.0 {
        psum(&x.iter().zip(y).map(|(a, b)| a * b).collect::<Vec<_>>()) / d
    } else {
        f64::NAN
    }
}

/// `np.linspace(a, b, n)`.
pub fn linspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    if n == 1 {
        return vec![a];
    }
    let step = (b - a) / (n - 1) as f64;
    let mut v: Vec<f64> = (0..n).map(|i| i as f64 * step + a).collect();
    v[n - 1] = b;
    v
}

/// `np.geomspace(a, b, n)` for positive `a`, `b`.
pub fn geomspace(a: f64, b: f64, n: usize) -> Vec<f64> {
    let mut v: Vec<f64> = linspace(a.log10(), b.log10(), n)
        .into_iter()
        .map(|e| 10f64.powf(e))
        .collect();
    v[0] = a;
    v[n - 1] = b;
    v
}

/// `np.argmin` (first minimum; NaN counts as the minimum like numpy).
pub fn argmin(a: &[f64]) -> usize {
    let mut k = 0;
    for (i, v) in a.iter().enumerate() {
        if v.is_nan() {
            return i;
        }
        if *v < a[k] {
            k = i;
        }
    }
    k
}
