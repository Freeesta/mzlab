//! Measures of a broad candidate taken before the common mode G(t) is subtracted: the half-height width of its left front (the tail of a
//! narrow peak rises fast and is dropped) and its area over the local linear bridge, plus the noise and base of a short series. Used by `broad`.

use crate::broad_gt::median;
use mzlab_core::error::Result;

use crate::trace::reserve;

/// Seconds from the left half-height point of the front to the apex (block `t` of the decimated trace `y_d`, baseline at block `lo_d`).
/// Zero when the walk to the left border found no flank at all; infinite when the apex is not above the baseline, so a candidate whose front cannot be measured is kept.
pub(crate) fn hwhm_left(rt_d: &[f64], y_d: &[f64], t: usize, lo_d: usize) -> f64 {
    let (base, top) = (y_d[lo_d], y_d[t]);
    // no flank to the left of the apex: the candidate sits at the foot of something higher, a step and not a rise
    if lo_d >= t {
        return 0.0;
    }
    if top <= base {
        return f64::INFINITY;
    }
    let half = base + 0.5 * (top - base);
    for j in (lo_d..t).rev() {
        if y_d[j] < half {
            let frac = (y_d[j + 1] - half) / (y_d[j + 1] - y_d[j]);
            return rt_d[t] - (rt_d[j + 1] - frac * (rt_d[j + 1] - rt_d[j]));
        }
    }
    rt_d[t] - rt_d[lo_d]
}

/// Trapezoid area (scan units, seconds x intensity) of `y` over scans `lo..=hi` after removing the line between `y[lo]` and `y[hi]`.
pub(crate) fn area_over_line(rt_s: &[f64], y: &[f64], lo: usize, hi: usize) -> (f64, f64) {
    let span = (hi - lo).max(1) as f64;
    let line = |i: usize| y[lo] + (y[hi] - y[lo]) * (i - lo) as f64 / span;
    let (mut area, mut area_s) = (0.0, 0.0);
    for i in lo + 1..=hi {
        let (u, v) = (y[i - 1] - line(i - 1), y[i] - line(i));
        area += (u + v) / 2.0;
        area_s += (rt_s[i] - rt_s[i - 1]) * (u + v) / 2.0;
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
    v.sort_by(|a, b| a.total_cmp(b));
    Ok(v.get(v.len() / 4).copied().unwrap_or(0.0))
}
