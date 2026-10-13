//! The resolver of the MRM chromatograms: moving average over seconds, variance-stabilising transform of the counting noise, maxima by prominence in
//! standard deviations and limits at the feet of the peaks.

use mzlab_core::peaks::noise_level;

/// Number of points spanning `s` seconds at the step `dt` seconds (at least 1, odd: the window of a centred smoothing).
pub(super) fn odd_points(s: f64, dt: f64) -> usize {
    if dt > 0.0 && s > 0.0 {
        ((s / dt).round().max(1.0) as usize) | 1
    } else {
        1
    }
}

/// Centred moving average over `w` points (odd), on fewer points at the two ends: it keeps the area and averages the counts.
pub(super) fn boxcar(y: &[f64], w: usize) -> Vec<f64> {
    let h = w / 2;
    let mut c = vec![0.0; y.len() + 1];
    for (i, v) in y.iter().enumerate() {
        c[i + 1] = c[i] + v;
    }
    (0..y.len())
        .map(|i| {
            let (a, b) = (i.saturating_sub(h), (i + h + 1).min(y.len()));
            (c[b] - c[a]) / (b - a) as f64
        })
        .collect()
}

/// Variance-stabilising transform of a chromatogram averaged over `w` points (generalised Anscombe, Poisson plus Gaussian). A counting detector reports
/// multiples of one count (`quantum`, the smallest positive value) whose spread grows as the square root of the level; a continuous noise adds the MAD of
/// the differences, which is 0 on a sparse chromatogram made mostly of zeros. Both shrink with the points averaged. After the transform the noise is
/// about 1 at every level, so a rise is measured in standard deviations at the foot and at the apex of a peak alike.
pub(super) struct Stabilise {
    gain: f64,
    pub sigma: f64,
}

impl Stabilise {
    pub(super) fn new(y: &[f64], w: usize) -> Stabilise {
        let q = y
            .iter()
            .copied()
            .filter(|&v| v > 0.0)
            .fold(f64::INFINITY, f64::min);
        let w = w as f64;
        Stabilise {
            gain: if q.is_finite() { q / w } else { 0.0 },
            sigma: noise_level(y) / w.sqrt(),
        }
    }

    /// The level `x` in standard deviations of the noise, 0 at the level 0.
    pub(super) fn z(&self, x: f64) -> f64 {
        let (a, s2) = (self.gain, self.sigma * self.sigma);
        let f = |x: f64| 2.0 / a * (a * x.max(0.0) + 0.375 * a * a + s2).sqrt();
        if a > 0.0 {
            f(x) - f(0.0)
        } else if s2 > 0.0 {
            x / self.sigma
        } else {
            x
        }
    }
}

/// Area of `y` between the points `lo` and `hi` above the straight line joining them (trapezoid, minutes).
pub(super) fn area_above(t: &[f64], y: &[f64], lo: usize, hi: usize) -> f64 {
    let span = t[hi] - t[lo];
    let seg: Vec<f64> = (lo..=hi)
        .map(|i| {
            let base = if span > 0.0 {
                y[lo] + (y[hi] - y[lo]) * (t[i] - t[lo]) / span
            } else {
                y[lo]
            };
            (y[i] - base).max(0.0)
        })
        .collect();
    (1..seg.len())
        .map(|i| (seg[i] + seg[i - 1]) / 2.0 * (t[lo + i] - t[lo + i - 1]))
        .sum()
}

/// A peak of a stabilised trace: apex, limits (inclusive) and rise over the higher of its two valleys (standard deviations).
pub(super) struct Span {
    pub apex: usize,
    pub lo: usize,
    pub hi: usize,
    pub rise: f64,
}

/// The local maxima of `z` that rise at least `snr` over the higher of their two valleys (the lowest point before a higher one on each side; of two
/// equal maxima the first one is the higher, so a flat top is one peak).
fn maxima(z: &[f64], snr: f64) -> Vec<(usize, f64)> {
    let n = z.len();
    let mut out = Vec::new();
    for j in 1..n.saturating_sub(1) {
        if !(z[j] > z[j - 1] && z[j] >= z[j + 1]) {
            continue;
        }
        let (mut lm, mut k) = (z[j], j);
        while k > 0 && z[k - 1] < z[j] {
            k -= 1;
            lm = lm.min(z[k]);
        }
        let (mut rm, mut k) = (z[j], j);
        while k + 1 < n && z[k + 1] <= z[j] {
            k += 1;
            rm = rm.min(z[k]);
        }
        if z[j] - lm.max(rm) >= snr {
            out.push((j, z[j] - lm.max(rm)));
        }
    }
    out
}

/// The foot of a peak walking from the apex `a` towards `stop` (inclusive): the lowest point reached while a lower one keeps coming within `span`
/// points. A wiggle of the noise on the flank does not stop it; the flat baseline, where the trace stops falling, does.
fn foot(z: &[f64], a: usize, stop: usize, span: usize) -> usize {
    let (mut best, mut i) = (a, a);
    while i != stop {
        i = if stop < a { i - 1 } else { i + 1 };
        if z[i] < z[best] {
            best = i;
        } else if i.abs_diff(best) > span {
            break;
        }
    }
    best
}

/// The peaks of the stabilised trace `z`: maxima that rise `snr` standard deviations, limits at their feet (never past the lowest point between two
/// neighbouring maxima), at least `min_pts` points wide.
pub(super) fn resolve(z: &[f64], snr: f64, min_pts: usize) -> Vec<Span> {
    let m = maxima(z, snr);
    let valley = |a: usize, b: usize| (a..=b).fold(a, |v, i| if z[i] < z[v] { i } else { v });
    let mut out = Vec::new();
    for (idx, &(apex, rise)) in m.iter().enumerate() {
        let left = if idx > 0 {
            valley(m[idx - 1].0, apex)
        } else {
            0
        };
        let right = m
            .get(idx + 1)
            .map_or(z.len() - 1, |&(b, _)| valley(apex, b));
        let (lo, hi) = (foot(z, apex, left, min_pts), foot(z, apex, right, min_pts));
        if hi - lo + 1 >= min_pts {
            out.push(Span { apex, lo, hi, rise });
        }
    }
    out
}
