//! Quality measures of an integrated peak and a second way of finding peaks ("prominence + local minimum"), next to the CWT of `peaks`.
//!
//! Reference: MassCube (Yu et al., Nat. Commun. 2025, doi 10.1038/s41467-025-60640-5): peaks are the local maxima of a slightly smoothed
//! trace that stand out by their prominence, the limits sit at the valleys between neighbours, areas are summed on the raw signal. A CWT
//! with ridge lines tends to merge two peaks that overlap; this one separates them wherever the trace has a valley. The measures are
//! numbers to discuss the quality of an integration: they never say "good" or "bad".
//! Everything is O(n) in the points of the peak (the detector is O(n log n) at most in the trace).

use crate::peaks::{noise_level, parabola_height, savgol, Peak};

/// Measures of one peak between the limits `lo..=hi` of the raw trace.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quality {
    /// Pearson correlation (-1..1) between the peak and a Gaussian with its apex and its full width at half maximum.
    pub gaussian: f64,
    /// Width of the right half over the width of the left half, both measured at 10 % of the height (1 = symmetric). `None` when the trace
    /// does not come down to 10 % inside the limits.
    pub asymmetry: Option<f64>,
    /// Turning points beyond the apex (changes of direction of the steps larger than the noise) over the points of the peak: 0 = a clean
    /// bell, towards 1 = the trace zigzags.
    pub noise_score: f64,
}

/// The straight baseline between the first and the last point of the limits, at index `i`.
fn base_at(rt: &[f64], y: &[f64], lo: usize, hi: usize, i: usize) -> f64 {
    let span = rt[hi] - rt[lo];
    if span <= 0.0 {
        return y[lo];
    }
    y[lo] + (y[hi] - y[lo]) * (rt[i] - rt[lo]) / span
}

/// Time at which `c` (the baseline-corrected trace) crosses `level` going from `from` towards `to` (indices, either direction), or `None`.
fn crossing(rt: &[f64], c: &[f64], level: f64, from: usize, to: usize) -> Option<f64> {
    let mut i = from;
    while i != to {
        let j = if to > from { i + 1 } else { i - 1 };
        if c[j] <= level {
            let (a, b) = (c[i] - level, c[i] - c[j]);
            let f = if b > 0.0 { a / b } else { 0.0 };
            return Some(rt[i] + (rt[j] - rt[i]) * f);
        }
        i = j;
    }
    None
}

/// Quality measures of the peak between `lo` and `hi` (inclusive) of the raw trace `y` at times `rt`.
/// `noise` is the noise of the whole trace (`peaks::noise_level`); 0 = take it from the points of the peak.
pub fn quality(rt: &[f64], y: &[f64], lo: usize, hi: usize, noise: f64) -> Quality {
    let n = y.len().min(rt.len());
    if n < 3 || lo >= hi || hi >= n {
        return Quality { gaussian: 0.0, asymmetry: None, noise_score: 0.0 };
    }
    let c: Vec<f64> = (lo..=hi).map(|i| y[i] - base_at(rt, y, lo, hi, i)).collect();
    let t = &rt[lo..=hi];
    let m = c.len();
    let mut k = 0;
    for j in 0..m {
        if c[j] > c[k] {
            k = j;
        }
    }
    let h = c[k];
    if h <= 0.0 {
        return Quality { gaussian: 0.0, asymmetry: None, noise_score: 0.0 };
    }
    // full width at half maximum; a side that does not come down inside the limits counts as its mirror image
    let (l, r) = (crossing(t, &c, h / 2.0, k, 0), crossing(t, &c, h / 2.0, k, m - 1));
    let fwhm = match (l, r) {
        (Some(a), Some(b)) => b - a,
        (Some(a), None) => 2.0 * (t[k] - a),
        (None, Some(b)) => 2.0 * (b - t[k]),
        (None, None) => t[m - 1] - t[0],
    };
    let sigma = (fwhm / 2.354_820_045).max((t[m - 1] - t[0]) * 1e-6).max(1e-12);
    let g: Vec<f64> = t.iter().map(|x| h * (-((x - t[k]).powi(2)) / (2.0 * sigma * sigma)).exp()).collect();
    let (mc, mg) = (c.iter().sum::<f64>() / m as f64, g.iter().sum::<f64>() / m as f64);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for j in 0..m {
        let (a, b) = (c[j] - mc, g[j] - mg);
        sxy += a * b;
        sxx += a * a;
        syy += b * b;
    }
    let gaussian = if sxx > 0.0 && syy > 0.0 { (sxy / (sxx * syy).sqrt()).clamp(-1.0, 1.0) } else { 0.0 };
    let asymmetry = match (crossing(t, &c, 0.1 * h, k, 0), crossing(t, &c, 0.1 * h, k, m - 1)) {
        (Some(a), Some(b)) if t[k] - a > 0.0 => Some((b - t[k]) / (t[k] - a)),
        _ => None,
    };
    // turning points: steps smaller than the noise do not change the direction
    let sigma_n = if noise > 0.0 { noise } else { noise_level(&y[lo..=hi]) };
    let (mut dir, mut turns) = (0i8, 0usize);
    for j in 1..m {
        let d = y[lo + j] - y[lo + j - 1];
        if d.abs() <= sigma_n {
            continue;
        }
        let s = if d > 0.0 { 1 } else { -1 };
        if dir != 0 && s != dir {
            turns += 1;
        }
        dir = s;
    }
    let noise_score = (turns.saturating_sub(1)) as f64 / (m - 2).max(1) as f64;
    Quality { gaussian, asymmetry, noise_score }
}

#[derive(Debug, Clone, Copy)]
pub struct ProminenceParams {
    /// A peak must stand out of its valleys by at least `snr` times the noise.
    pub snr: f64,
    /// ... and by at least this fraction of the highest point of the trace.
    pub rel_prominence: f64,
    pub min_scans: usize,
    /// The outer limits stop where the trace falls below this fraction of the apex.
    pub frac: f64,
    /// Window of the light Savitzky-Golay smoothing used to find the maxima (odd).
    pub smooth: usize,
}

impl Default for ProminenceParams {
    fn default() -> Self {
        ProminenceParams { snr: 5.0, rel_prominence: 0.01, min_scans: 3, frac: 0.05, smooth: 5 }
    }
}

/// Peaks of the trace by prominence and local minima, sorted by decreasing height. `Peak.snr` = prominence over the noise; `Peak.area` is
/// summed on the RAW signal above the straight line between the two limits (trapezoid, RT in minutes); a limit shared with the next peak is
/// a valley and counts as a drop line down to the level of the other limit.
pub fn detect_prominence(rt: &[f64], y: &[f64], p: ProminenceParams) -> Vec<Peak> {
    let n = y.len().min(rt.len());
    if n < 7 || !y.iter().any(|&v| v > 0.0) {
        return Vec::new();
    }
    let ys: Vec<f64> = savgol(&y[..n], p.smooth, 2).into_iter().map(|v| v.max(0.0)).collect();
    let noise = noise_level(&y[..n]).max(1e-9);
    let top = ys.iter().cloned().fold(0.0, f64::max);
    let need = (p.snr * noise).max(p.rel_prominence * top);
    // local maxima and their prominence
    let mut cand: Vec<(usize, f64)> = Vec::new();
    for j in 1..n - 1 {
        if !(ys[j] > ys[j - 1] && ys[j] >= ys[j + 1] && ys[j] > 0.0) {
            continue;
        }
        let (mut lm, mut k) = (ys[j], j);
        while k > 0 && ys[k - 1] <= ys[j] {
            k -= 1;
            lm = lm.min(ys[k]);
        }
        let left = lm; // lowest point between the apex and the next higher point on the left (or the start)
        let (mut rm, mut k) = (ys[j], j);
        while k + 1 < n && ys[k + 1] <= ys[j] {
            k += 1;
            rm = rm.min(ys[k]);
        }
        let prom = ys[j] - left.max(rm);
        if prom >= need {
            cand.push((j, prom));
        }
    }
    let mut out = Vec::new();
    for (idx, &(apex, prom)) in cand.iter().enumerate() {
        let h = ys[apex];
        let mut lo = apex;
        let left_stop = if idx > 0 { Some(valley(&ys, cand[idx - 1].0, apex)) } else { None };
        while lo > 0 && ys[lo - 1] > p.frac * h && ys[lo - 1] <= ys[lo] + 1e-12 && left_stop.is_none_or(|v| lo > v) {
            lo -= 1;
        }
        if let Some(v) = left_stop {
            lo = lo.max(v);
        }
        let mut hi = apex;
        let right_stop = if idx + 1 < cand.len() { Some(valley(&ys, apex, cand[idx + 1].0)) } else { None };
        while hi < n - 1 && ys[hi + 1] > p.frac * h && ys[hi + 1] <= ys[hi] + 1e-12 && right_stop.is_none_or(|v| hi < v) {
            hi += 1;
        }
        if let Some(v) = right_stop {
            hi = hi.min(v);
        }
        let (lo2, hi2) = (lo, hi);
        if hi2 - lo2 + 1 < p.min_scans {
            continue;
        }
        // a limit shared with a neighbour sits on a valley, not on the baseline: the baseline there is a vertical drop to the level of the other limit
        let (mut yl, mut yh) = (y[lo2], y[hi2]);
        match (left_stop.is_some(), right_stop.is_some()) {
            (true, false) => yl = yh,
            (false, true) => yh = yl,
            (true, true) => {
                yl = yl.min(yh);
                yh = yl;
            }
            _ => {}
        }
        let span = rt[hi2] - rt[lo2];
        let seg: Vec<f64> = (lo2..=hi2)
            .map(|i| (y[i] - if span > 0.0 { yl + (yh - yl) * (rt[i] - rt[lo2]) / span } else { yl }).max(0.0))
            .collect();
        let area = (1..seg.len()).map(|i| (seg[i] + seg[i - 1]) / 2.0 * (rt[lo2 + i] - rt[lo2 + i - 1])).sum();
        out.push(Peak { apex_i: apex, height: parabola_height(rt, &ys, apex), area, lo: lo2, hi: hi2, n_scans: hi2 - lo2 + 1, snr: prom / noise });
    }
    crate::sortc::by(&mut out, |a, b| b.height.partial_cmp(&a.height).unwrap_or(std::cmp::Ordering::Equal));
    out
}

/// Index of the lowest point of `ys` strictly between the apexes `a` and `b` (the first one on a tie).
fn valley(ys: &[f64], a: usize, b: usize) -> usize {
    let mut v = a + 1;
    for i in a + 1..b {
        if ys[i] < ys[v] {
            v = i;
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peaks::{detect_peaks, Params};

    /// Deterministic noise in [-1, 1).
    fn lcg(seed: &mut u64) -> f64 {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((*seed >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
    }

    /// 6 minutes at 0.015 min, a sum of Gaussians `(centre, sigma, height)` plus an optional exponential tail and noise.
    fn trace(parts: &[(f64, f64, f64)], tail: f64, noise: f64) -> (Vec<f64>, Vec<f64>) {
        let rt: Vec<f64> = (0..400).map(|i| i as f64 * 0.015).collect();
        let mut seed = 7u64;
        let mut y: Vec<f64> = rt.iter().map(|t| parts.iter().map(|(c, s, h)| h * (-((t - c).powi(2)) / (2.0 * s * s)).exp()).sum::<f64>()).collect();
        if tail > 0.0 {
            let src = y.clone();
            for i in 0..y.len() {
                let mut acc = 0.0;
                for j in 0..=i {
                    acc += src[j] * (-((i - j) as f64) * 0.015 / tail).exp();
                }
                y[i] = acc * 0.015 / tail;
            }
        }
        for v in y.iter_mut() {
            *v += noise * lcg(&mut seed) + 10.0 * noise;
        }
        (rt, y)
    }

    #[test]
    fn single_peak_is_a_clean_bell() {
        let (rt, y) = trace(&[(3.0, 0.06, 1.0e5)], 0.0, 50.0);
        let pk = detect_prominence(&rt, &y, ProminenceParams::default());
        assert_eq!(pk.len(), 1);
        let p = &pk[0];
        assert!((p.apex_i as f64 * 0.015 - 3.0).abs() < 0.03);
        let q = quality(&rt, &y, p.lo, p.hi, noise_level(&y));
        assert!(q.gaussian > 0.99, "{q:?}");
        assert!((q.asymmetry.unwrap() - 1.0).abs() < 0.1, "{q:?}");
        assert!(q.noise_score < 0.15, "{q:?}");
        // true area 1e5 * 0.06 * sqrt(2 pi) = 15040; the limits at 5 % of the apex and the straight baseline between them leave out the tails
        assert!((p.area / 15040.0 - 1.0).abs() < 0.2, "area {}", p.area);
    }

    #[test]
    fn tail_gives_asymmetry_above_one() {
        let (rt, y) = trace(&[(2.0, 0.05, 1.0e5)], 0.12, 50.0);
        let pk = detect_prominence(&rt, &y, ProminenceParams::default());
        assert_eq!(pk.len(), 1);
        let q = quality(&rt, &y, pk[0].lo, pk[0].hi, noise_level(&y));
        assert!(q.asymmetry.unwrap() > 1.5, "{q:?}");
        assert!(q.gaussian < 0.99);
    }

    #[test]
    fn partly_resolved_double_is_split_at_the_valley() {
        // two peaks 3.4 sigma apart, heights 1 : 0.7: the trace has a valley between them
        let (rt, y) = trace(&[(2.5, 0.06, 1.0e5), (2.5 + 3.4 * 0.06, 0.06, 0.7e5)], 0.0, 50.0);
        let two = detect_prominence(&rt, &y, ProminenceParams::default());
        assert_eq!(two.len(), 2, "{two:?}");
        let (a, b) = if two[0].apex_i < two[1].apex_i { (&two[0], &two[1]) } else { (&two[1], &two[0]) };
        assert_eq!(a.hi, b.lo, "the limits meet at the valley");
        let mid = 2.5 + 1.7 * 0.06;
        assert!((a.hi as f64 * 0.015 - mid).abs() < 0.06, "valley at {}", a.hi as f64 * 0.015);
        assert!((a.area / (1.0e5 * 0.06 * (2.0 * std::f64::consts::PI).sqrt()) - 1.0).abs() < 0.35, "area {}", a.area);
        // the CWT finds two peaks here as well, but its limits overlap (the same points are integrated twice); the valley limits of the new mode do not
        let mut c = detect_peaks(&rt, &y, Params::default());
        assert_eq!(c.len(), 2, "{c:?}");
        crate::sortc::slice_by_key(&mut c, |p| p.apex_i);
        assert!(c[0].hi > c[1].lo, "CWT limits overlap: {c:?}");
        assert!(a.hi <= b.lo);
    }

    #[test]
    fn shoulder_stays_one_peak() {
        let (rt, y) = trace(&[(2.5, 0.06, 1.0e5), (2.5 + 1.4 * 0.06, 0.06, 0.4e5)], 0.0, 50.0);
        let pk = detect_prominence(&rt, &y, ProminenceParams::default());
        assert_eq!(pk.len(), 1, "{pk:?}");
    }

    #[test]
    fn noise_alone_has_no_peaks() {
        let (rt, y) = trace(&[], 0.0, 50.0);
        assert!(detect_prominence(&rt, &y, ProminenceParams::default()).is_empty());
    }

    #[test]
    fn zigzag_raises_the_noise_score() {
        let rt: Vec<f64> = (0..21).map(|i| i as f64 * 0.02).collect();
        let clean: Vec<f64> = (0..21).map(|i| 1000.0 * (-((i as f64 - 10.0).powi(2)) / 18.0).exp()).collect();
        let mut jag = clean.clone();
        for (i, v) in jag.iter_mut().enumerate() {
            *v += if i % 2 == 0 { 120.0 } else { -120.0 };
        }
        let a = quality(&rt, &clean, 0, 20, 10.0);
        let b = quality(&rt, &jag, 0, 20, 10.0);
        assert!(b.noise_score > a.noise_score + 0.3, "{a:?} {b:?}");
    }
}

