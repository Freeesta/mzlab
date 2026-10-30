//! Measures of a broad candidate taken before the common mode G(t) is subtracted: the half-height width of its left front (the tail of a
//! narrow peak rises fast and is dropped) and its area over the local linear bridge, plus the noise and base of a short series. Used by `broad`.

use crate::sortc::SortC;
use crate::broad_gt::median;
use mzlab_core::error::Result;

use crate::trace::reserve;

/// Seconds from the left half-height point of the front to the apex: block `t` of the series `y` cut in blocks of `k` scans, baseline at block `lo_d`.
/// Zero when `lo_d` is the apex block (no flank: a step, as at the foot of something higher); when no block is below half height the width runs to `lo_d`.
pub(crate) fn hwhm_left(rt_d: &[f64], y: &[f64], k: usize, t: usize, lo_d: usize) -> f64 {
    let block = |b: usize| {
        let s = &y[b * k..((b + 1) * k).min(y.len())];
        s.iter().sum::<f64>() / s.len() as f64
    };
    let half = (block(lo_d) + block(t)) / 2.0;
    let at = (lo_d..t)
        .rev()
        .find(|&j| block(j) < half)
        .map_or(lo_d, |j| j + 1);
    rt_d[t] - rt_d[at]
}

/// Trapezoid area (scan units, seconds x intensity) of `y` over scans `lo..=hi` after removing the line that runs from `b_lo` at `lo` to `b_hi` at `hi`.
pub(crate) fn area_over_line(
    rt_s: &[f64],
    y: &[f64],
    (lo, hi): (usize, usize),
    (b_lo, b_hi): (f64, f64),
) -> (f64, f64) {
    let line = |i: usize| b_lo + (b_hi - b_lo) * (i - lo) as f64 / (hi - lo) as f64;
    let (mut area, mut area_s, mut u) = (0.0, 0.0, y[lo] - b_lo);
    for i in lo + 1..=hi {
        let v = y[i] - line(i);
        area += (u + v) / 2.0;
        area_s += (rt_s[i] - rt_s[i - 1]) * (u + v) / 2.0;
        u = v;
    }
    (area, area_s)
}

/// Noise of a short series: 1.4826 x median of |first difference| / sqrt 2 (robust to the bands themselves; no reach of a peak is involved).
pub(crate) fn noise_of(y: &[f64]) -> Result<f64> {
    let mut d = Vec::new();
    reserve(&mut d, y.len())?;
    d.extend(y.windows(2).map(|w| (w[1] - w[0]).abs()));
    Ok(1.4826 * median(&mut d) / core::f64::consts::SQRT_2)
}

/// Lower quartile of a series: the base of the bands.
pub(crate) fn base_of(y: &[f64]) -> Result<f64> {
    let mut v = Vec::new();
    reserve(&mut v, y.len())?;
    v.extend_from_slice(y);
    v.sortc_by(|a, b| a.total_cmp(b));
    Ok(v.get(v.len() / 4).copied().unwrap_or(0.0))
}
