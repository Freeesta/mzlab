//! Peak picking on one trace: a line-by-line port of `ionfamily.detect_peaks` of the Python engine
//! (AsLS/SNIP baseline, Savitzky-Golay, CWT with Ricker wavelets, ridge lines, limits at 5% of the apex).
//! The Python engine is the reference: any difference in the limits is a difference of logic, not of tolerance.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Peak {
    pub apex_i: usize,
    /// Height of the parabola through the apex (on the smoothed trace).
    pub height: f64,
    /// Area of the baseline-corrected trace between the limits (trapezoid, RT in minutes).
    pub area: f64,
    pub lo: usize,
    pub hi: usize,
    pub n_scans: usize,
    pub snr: f64,
}

pub const MIN_SCANS_RELIABLE: usize = 6;

use crate::signal::{asls_baseline, cwt_ricker, median, snip_baseline, solve};
pub use crate::signal::{noise_level, savgol};


/// Height of the quadratic fit around index `i` (the second moment is not needed here), as `_parabola_apex`.
pub(crate) fn parabola_height(rt: &[f64], y: &[f64], i: usize) -> f64 {
    let n = y.len();
    let half = 2usize;
    let a = i.saturating_sub(half);
    let b = (i + half + 1).min(n);
    if b - a < 3 {
        return y[i];
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
    let coef = match solve(ata, aty, 3) {
        Some(c) => c,
        None => return y[i],
    };
    let (c0, c1, c2) = (coef[0], coef[1], coef[2]);
    if c2 >= 0.0 {
        return y[i];
    }
    let (lo, hi) = (xs[0], xs[xs.len() - 1]);
    let xa = (-c1 / (2.0 * c2)).clamp(lo, hi);
    c0 + c1 * xa + c2 * xa * xa
}

fn trapz(y: &[f64], x: &[f64]) -> f64 {
    (1..y.len())
        .map(|i| (y[i] + y[i - 1]) / 2.0 * (x[i] - x[i - 1]))
        .sum()
}

struct Ridge {
    pos: Vec<usize>,
    val: Vec<f64>,
    scale: Vec<usize>,
    last_scale: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Params {
    pub snr: f64,
    pub min_scans: usize,
    pub frac: f64,
    pub smooth: usize,
    /// true = AsLS baseline (default), false = SNIP.
    pub asls: bool,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            snr: 5.0,
            min_scans: 3,
            frac: 0.05,
            smooth: 5,
            asls: true,
        }
    }
}

/// Result of the peak picking with the intermediate traces (`ionfamily.detect_peaks` returns them too).
#[derive(Debug, Clone, Default)]
pub struct Detection {
    /// Baseline-corrected trace (not clipped).
    pub yc: Vec<f64>,
    /// Smoothed, non-negative corrected trace.
    pub ys: Vec<f64>,
    pub noise: f64,
    pub peaks: Vec<Peak>,
}

/// Peaks of the trace (rt in minutes, y), sorted by decreasing height like the Python engine.
pub fn detect_peaks(rt: &[f64], y: &[f64], params: Params) -> Vec<Peak> {
    detect_full(rt, y, params).peaks
}

/// `detect_peaks` with the corrected and smoothed traces and the noise level.
/// `unique(round(geomspace(1, max(3, min(n/6, 12)), 7), 2))`: the widths of the wavelet scales.
fn scale_widths(n: usize) -> Vec<f64> {
    let stop = (n as f64 / 6.0).min(12.0).max(3.0);
    let mut widths: Vec<f64> = (0..7)
        .map(|i| {
            let g = if i == 0 {
                1.0
            } else if i == 6 {
                stop
            } else {
                (stop.ln() * i as f64 / 6.0).exp()
            };
            (g * 100.0).round_ties_even() / 100.0
        })
        .collect();
    widths.dedup();
    widths
}

/// Ridge lines over the scales: the local maxima of each scale joined when they stay close, with the value over the noise of their scale.
fn ridge_lines(c: &[Vec<f64>], widths: &[f64]) -> Vec<Ridge> {
    let (nw, n) = (widths.len(), c[0].len());
    let peaks_per_scale: Vec<Vec<usize>> = (0..nw)
        .map(|i| {
            (1..n - 1)
                .filter(|&j| c[i][j] > c[i][j - 1] && c[i][j] >= c[i][j + 1] && c[i][j] > 0.0)
                .collect()
        })
        .collect();
    let scale_noise: Vec<f64> = (0..nw)
        .map(|i| {
            let m = median(&c[i]);
            let dev: Vec<f64> = c[i].iter().map(|v| (v - m).abs()).collect();
            (median(&dev) * 1.4826).max(1e-12)
        })
        .collect();
    let mut ridges: Vec<Ridge> = Vec::new();
    for (i, mx) in peaks_per_scale.iter().enumerate() {
        for &pos in mx {
            let mut joined = false;
            for r in ridges.iter_mut() {
                if r.last_scale + 1 == i || r.last_scale + 2 == i {
                    let last = *r.pos.last().unwrap() as f64;
                    if (last - pos as f64).abs() <= (widths[i] / 2.0).max(1.0) {
                        r.pos.push(pos);
                        r.val.push(c[i][pos] / scale_noise[i]);
                        r.scale.push(i);
                        r.last_scale = i;
                        joined = true;
                        break;
                    }
                }
            }
            if !joined {
                ridges.push(Ridge {
                    pos: vec![pos],
                    val: vec![c[i][pos] / scale_noise[i]],
                    scale: vec![i],
                    last_scale: i,
                });
            }
        }
    }
    ridges
}

/// The apexes `(index, ridge SNR, width)` that the ridges support, in order of position.
fn ridge_apexes(ridges: &[Ridge], widths: &[f64], ys: &[f64], noise: f64, snr: f64) -> Vec<(usize, f64, f64)> {
    let n = ys.len();
    let mut found: Vec<(usize, f64, f64)> = Vec::new();
    let mut taken: Vec<usize> = Vec::new();
    for r in ridges {
        if r.pos.len() < 3 {
            continue;
        }
        let mut k = 0;
        for (j, v) in r.val.iter().enumerate() {
            if *v > r.val[k] {
                k = j;
            }
        }
        if r.val[k] < snr {
            continue;
        }
        let w = widths[r.scale[k]];
        let c0 = r.pos[k] as f64;
        let a = ((c0 - w).trunc().max(0.0)) as usize;
        let b = (((c0 + w).trunc() + 1.0).max(0.0) as usize).min(n);
        let mut apex = a;
        for j in a..b {
            if ys[j] > ys[apex] {
                apex = j;
            }
        }
        if ys[apex] < snr * noise {
            continue;
        }
        if taken
            .iter()
            .any(|&t| (apex as f64 - t as f64).abs() <= (w / 2.0).max(1.0))
        {
            continue;
        }
        taken.push(apex);
        found.push((apex, r.val[k], w));
    }
    crate::sortc::by(&mut found, |a, b| {
        a.0.cmp(&b.0)
            .then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    });
    found
}

/// Borders, area and height of each apex; the peaks with fewer scans than `min_scans` are dropped; by decreasing height.
fn peaks_of(found: Vec<(usize, f64, f64)>, rt: &[f64], yc: &[f64], ys: &[f64], params: &Params) -> Vec<Peak> {
    let n = ys.len();
    let mut peaks = Vec::new();
    for (apex, rsn, _w) in found {
        let h = ys[apex];
        let mut lo = apex;
        while lo > 0 && ys[lo - 1] > params.frac * h && ys[lo - 1] <= ys[lo] + 1e-12 {
            lo -= 1;
        }
        let mut hi = apex;
        while hi < n - 1 && ys[hi + 1] > params.frac * h && ys[hi + 1] <= ys[hi] + 1e-12 {
            hi += 1;
        }
        let lo2 = lo.saturating_sub(1);
        let hi2 = (hi + 1).min(n - 1);
        let hh = parabola_height(rt, ys, apex);
        let seg: Vec<f64> = yc[lo2..=hi2].iter().map(|v| v.max(0.0)).collect();
        let area = trapz(&seg, &rt[lo2..=hi2]);
        let ns = hi2 - lo2 + 1;
        if ns < params.min_scans {
            continue;
        }
        peaks.push(Peak {
            apex_i: apex,
            height: hh,
            area,
            lo: lo2,
            hi: hi2,
            n_scans: ns,
            snr: rsn,
        });
    }
    crate::sortc::by(&mut peaks, |a, b| {
        b.height
            .partial_cmp(&a.height)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    peaks
}

pub fn detect_full(rt: &[f64], y: &[f64], params: Params) -> Detection {
    let n = y.len();
    if n < 7 || !y.iter().any(|&v| v > 0.0) {
        return Detection::default();
    }
    let base = if params.asls {
        asls_baseline(y, 1e5, 0.01, 10, 160)
    } else {
        snip_baseline(y, 20)
    };
    let yc: Vec<f64> = (0..n).map(|i| y[i] - base[i]).collect();
    let ys: Vec<f64> = savgol(&yc, params.smooth, 2)
        .into_iter()
        .map(|v| v.max(0.0))
        .collect();
    let noise = noise_level(&yc).max(1e-9);
    let widths = scale_widths(n);
    let c = cwt_ricker(&ys, &widths);
    let ridges = ridge_lines(&c, &widths);
    let found = ridge_apexes(&ridges, &widths, &ys, noise, params.snr);
    let peaks = peaks_of(found, rt, &yc, &ys, &params);
    Detection { yc, ys, noise, peaks }
}
