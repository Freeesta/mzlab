//! Local-maximum mass detector: centroids the profile peaks of ONE spectrum (feature `untargeted`).
//!
//! Rewritten from scratch after `featdet_massdetection/local_max/LocalMaxMassDetector.java` of mzmine 4.10.90 (MIT, (c) 2004-2022 The mzmine
//! Development Team); no code copied. Maxima and valleys are searched on the smoothed series (`smooth.rs`), height, area and centroid always on the
//! raw intensities. Reference: `tools/localmax_riferimento.py` (numpy). Not yet wired into the pipeline: `adap.rs` starts from centroided scans.
//! Where the text of the study is looser than the source the source wins: the gap that splits the runs is measured on the spectrum (`max_mz_gap`)
//! and the two-peak rule compares the valley with the ACTIVE peak (the highest of the merged ones).

use super::smooth::{smooth_run, Method};

const MZ_WEIGHTING_THRESHOLD: f64 = 0.4;
const VALLEY_FACTOR: f64 = 0.7;
const RISE_FACTOR: f64 = 1.0 / 0.7;

/// Settings of the detector (mzmine: noise level 0, «Height», at least 3 data points, no smoothing).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    pub noise: f64,
    /// `true`: the intensity of a peak is its area (sum of the points), `false`: its height.
    pub area: bool,
    pub min_points: usize,
    pub smoothing: Method,
    pub window: usize,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            noise: 0.0,
            area: false,
            min_points: 3,
            smoothing: Method::None,
            window: 5,
        }
    }
}

/// Largest m/z step of the spectrum: the first pair of consecutive non-zero points counted from the top, or from the start if that step is larger;
/// 0.1 when there is no such pair.
pub fn max_mz_gap(mz: &[f64], y: &[f64]) -> f64 {
    let n = mz.len();
    let mut top = n.saturating_sub(1);
    let mut right = -1.0f64;
    while top > 1 {
        if y[top] > 0.0 && y[top - 1] > 0.0 {
            right = (mz[top] - mz[top - 1]).abs();
            break;
        }
        top -= 1;
    }
    let mut left = -1.0f64;
    for i in 1..top {
        if y[i] > 0.0 && y[i - 1] > 0.0 {
            left = (mz[i] - mz[i - 1]).abs();
            break;
        }
    }
    let gap = left.max(right);
    if gap == -1.0 {
        0.1
    } else {
        gap
    }
}

/// Runs `[start, end]` (inclusive) of consecutive points: a run closes where the m/z step is at least `gap`; it is kept when it has at least
/// `min_points` points and one of them is above the noise.
fn consecutive_runs(mz: &[f64], y: &[f64], gap: f64, p: &Params) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut above = false;
    for i in 1..mz.len() {
        above |= y[i] > p.noise;
        if mz[i] - mz[i - 1] >= gap {
            if i - start >= p.min_points && above {
                out.push((start, i - 1));
            }
            start = i;
            above = false;
        }
    }
    if mz.len() - start >= p.min_points && above {
        out.push((start, mz.len() - 1));
    }
    out
}

/// Indices in `[start, end)` that are `>=` the left and `>` the right neighbour (outside the run counts as 0).
fn local_maxima(y: &[f64], start: usize, end: usize) -> Vec<usize> {
    (start..end)
        .filter(|&i| {
            let left = if i == start { 0.0 } else { y[i - 1] };
            let right = if i == end - 1 { 0.0 } else { y[i + 1] };
            y[i] >= left && y[i] > right
        })
        .collect()
}

/// The lowest point strictly between two indices (the first one if equal); `from` itself when nothing lies between.
fn lowest_valley(y: &[f64], from: usize, to: usize) -> usize {
    let mut best = from;
    let mut low = f64::MAX;
    for (i, v) in y.iter().enumerate().take(to).skip(from + 1) {
        if *v < low {
            low = *v;
            best = i;
        }
    }
    best
}

/// Edges `[start, end)` of the peaks of a run: neighbouring maxima stay apart only if the valley falls under 0.7 x the active peak and the next
/// maximum rises over valley / 0.7; otherwise they merge and the active peak is the higher one.
fn peak_edges(d: &[f64], run_start: usize, run_end: usize) -> Vec<(usize, usize)> {
    let cand = local_maxima(d, run_start, run_end + 1);
    let mut out = Vec::new();
    let Some(&first) = cand.first() else {
        return out;
    };
    let (mut active, mut left) = (first, run_start);
    for &next in &cand[1..] {
        let valley = lowest_valley(d, active, next);
        if d[valley] < VALLEY_FACTOR * d[active] && d[next] > RISE_FACTOR * d[valley] {
            out.push((left, valley + 1));
            active = next;
            left = valley;
        } else if d[next] > d[active] {
            active = next;
        }
    }
    out.push((left, run_end + 1));
    out
}

/// Centroid and intensity of the points `[s, e)` of the RAW data, or `None` when the peak is rejected.
fn centroid(
    mz: &[f64],
    y: &[f64],
    s: usize,
    e: usize,
    min_abs: f64,
    p: &Params,
) -> Option<(f64, f64)> {
    if e - s < p.min_points {
        return None;
    }
    let mut apex = s;
    for i in s + 1..e {
        if y[i] > y[apex] {
            apex = i;
        }
    }
    let top = y[apex];
    if top < p.noise.max(2.0 * min_abs) {
        return None;
    }
    let cutoff = top * MZ_WEIGHTING_THRESHOLD;
    let edge = (apex - s).min(e - apex);
    let (lo, hi) = (apex - edge, apex + edge);
    let (mut num, mut den, mut area, mut nonzero) = (0.0, 0.0, 0.0, 0);
    for i in s..e {
        area += y[i];
        if y[i] > cutoff && i >= lo && i <= hi {
            num += mz[i] * y[i];
            den += y[i];
        }
        if y[i] > 0.0 {
            nonzero += 1;
        }
    }
    if den == 0.0 || nonzero < p.min_points {
        return None;
    }
    Some((num / den, if p.area { area } else { top }))
}

/// The smallest intensity above 0 of the spectrum (the first point counts only if positive); 0 when there is none.
fn min_positive(y: &[f64]) -> f64 {
    let low = y
        .iter()
        .copied()
        .filter(|v| *v > 0.0)
        .fold(f64::MAX, f64::min);
    if low == f64::MAX {
        0.0
    } else {
        low
    }
}

/// Peaks of one profile spectrum as `(m/z, intensity)` vectors, in m/z order. `mz` is ascending; `y` has the same length.
pub fn detect(mz: &[f64], y: &[f64], p: &Params) -> (Vec<f64>, Vec<f64>) {
    let (mut out_mz, mut out_y) = (Vec::new(), Vec::new());
    if mz.len() < p.min_points || mz.len() != y.len() || mz.is_empty() {
        return (out_mz, out_y);
    }
    let runs = consecutive_runs(mz, y, max_mz_gap(mz, y), p);
    let min_abs = min_positive(y);
    let mut detect_on = y.to_vec();
    if p.smoothing != Method::None {
        detect_on = vec![0.0; y.len()];
        for &(s, e) in &runs {
            detect_on[s..=e]
                .copy_from_slice(&smooth_run(y, s, e + 1, p.smoothing, p.window)[s..=e]);
        }
    }
    for &(s, e) in &runs {
        for (a, b) in peak_edges(&detect_on, s, e) {
            if let Some((m, v)) = centroid(mz, y, a, b, min_abs, p) {
                out_mz.push(m);
                out_y.push(v);
            }
        }
    }
    (out_mz, out_y)
}
