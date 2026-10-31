//! Verbatim copy of the pre-optimisation Ricker response and peak picking, kept only so the unit tests can prove the new code is bit-identical.

use super::{filled, resolve, valley, Result, Wavelet, REACH};
use crate::sortc::SortC;
/// Responses of the Ricker wavelet `w` over `y`.
pub(super) fn response(y: &[f64], w: &Wavelet) -> Result<Vec<f64>> {
    let n = y.len();
    let (half, kernel, norm) = (w.half, &w.kernel, w.norm);
    let mut r = filled(n, 0.0f64)?;
    for (t, out) in r.iter_mut().enumerate() {
        // the taps that fall inside the series, in the order of the kernel
        let (i0, i1) = (
            (t as isize - half).max(0) as usize,
            (t + half as usize + 1).min(n),
        );
        let j0 = (i0 as isize - t as isize + half) as usize;
        let mut acc = 0.0;
        for (v, w) in y[i0..i1].iter().zip(&kernel[j0..]) {
            acc += v * w;
        }
        *out = acc * norm;
    }
    Ok(r)
}

/// Apexes (index, scale) of the series: the local maxima of the response at each scale, narrowest scale first; a wider scale adds only what the narrower
/// ones did not find nearby. The apex is the highest point of `y` within one standard deviation of the maximum.
fn apexes(y: &[f64], wavelets: &[Wavelet]) -> Result<Vec<(usize, f64)>> {
    let n = y.len();
    let mut found: Vec<(usize, f64)> = Vec::new();
    for w in wavelets {
        let s = w.s;
        let r = response(y, w)?;
        let reach = libm::ceil(s) as usize;
        for t in 0..n {
            let left = t == 0 || r[t] >= r[t - 1];
            let right = t + 1 == n || r[t] > r[t + 1];
            if r[t] > 0.0 && left && right {
                let (a, z) = (t.saturating_sub(reach), (t + reach).min(n - 1));
                let mut apex = a;
                for i in a..=z {
                    if y[i] > y[apex] {
                        apex = i;
                    }
                }
                if !found.iter().any(|&(f, _)| f.abs_diff(apex) <= reach) {
                    found.push((apex, s));
                }
            }
        }
    }
    found.sortc_by_key(|f| f.0);
    Ok(found)
}

/// Peaks of the series `y` as (first, apex, last) indexes, ordered by apex; adjacent peaks meet at the lowest point between their apexes.
pub(super) fn peaks(
    y: &[f64],
    wavelets: &[Wavelet],
    snr: f64,
    floor: f64,
) -> Result<Vec<(usize, usize, usize)>> {
    let n = y.len();
    let found = resolve(y, apexes(y, wavelets)?);
    let mut out: Vec<(usize, usize, usize)> = Vec::new();
    for (k, &(apex, s)) in found.iter().enumerate() {
        let reach = libm::ceil(REACH * s) as usize;
        let mut lo = apex.saturating_sub(reach);
        let mut hi = (apex + reach).min(n - 1);
        if k > 0 {
            lo = lo.max(valley(y, found[k - 1].0, apex));
        }
        if let Some(next) = found.get(k + 1) {
            hi = hi.min(valley(y, apex, next.0));
        }
        out.push((lo, apex, hi));
    }
    let skirt = wavelets.iter().fold(0.0f64, |m, w| m.max(w.s));
    Ok(keep_above_noise(
        y,
        out,
        snr,
        floor,
        libm::ceil(REACH * skirt) as usize,
    ))
}

/// The peaks whose height over the baseline is `snr` times the noise (at least `floor`). The noise is the spread of the points farther than `skirt` scans
/// from every apex: the flanks of a peak found by a narrow wavelet are not noise, and a short series with no such point has the noise of `floor`.
fn keep_above_noise(
    y: &[f64],
    found: Vec<(usize, usize, usize)>,
    snr: f64,
    floor: f64,
    skirt: usize,
) -> Vec<(usize, usize, usize)> {
    let (mut sum, mut sum2, mut m) = (0.0, 0.0, 0.0);
    for (i, &v) in y.iter().enumerate() {
        if !found
            .iter()
            .any(|&(lo, apex, hi)| (i >= lo && i <= hi) || i.abs_diff(apex) <= skirt)
        {
            sum += v;
            sum2 += v * v;
            m += 1.0;
        }
    }
    let sd = if m >= 3.0 {
        libm::sqrt(((sum2 - sum * sum / m) / (m - 1.0)).max(0.0))
    } else {
        0.0
    };
    let noise = sd.max(floor);
    found
        .into_iter()
        .filter(|&(lo, apex, hi)| (y[apex] - y[lo].min(y[hi])) >= snr * noise)
        .collect()
}
