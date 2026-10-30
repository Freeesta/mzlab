//! Noise rows of the unit-resolution profile: after the quality of the peaks, a row is dropped when it is much narrower than the peaks of its own file or
//! its apex does not rise over the noise of its flanks. The typical width is the median FWHM of the strongest rows, so the gate follows the
//! chromatography of each file; `analyse_with` calls it, only when `Params::min_width` or `Params::min_snr` is set.

use crate::params::{median, strongest};
use crate::resolve::Multi;
use crate::table::FileFeatures;
use crate::trace::Params;

/// Rows needed for the typical width; fewer and the width gate is off.
const MIN_STRONG: usize = 5;
/// Strongest rows that give the typical width.
const STRONG: usize = 80;

/// Sets the gate named `name` (`min_width` or `min_snr`) of `p`.
pub(crate) fn set_gate(p: &mut Params, name: &str, value: f64) {
    if name == "min_width" {
        p.min_width = value;
    } else {
        p.min_snr = value;
    }
}

/// Drops the noise rows of `f` and moves the rows named by `multi` to their new places (a dropped one leaves the list).
pub fn reject(f: &mut FileFeatures, multi: &mut Vec<Multi>, p: &Params) {
    let mut widths: Vec<f64> = strongest(f, STRONG)
        .into_iter()
        .map(|i| f64::from(f.fwhm[i]))
        .filter(|w| w.is_finite() && *w > 0.0)
        .collect();
    let narrow = if p.min_width > 0.0 && widths.len() >= MIN_STRONG {
        p.min_width * median(&mut widths)
    } else {
        0.0
    };
    let keep: Vec<bool> = (0..f.len())
        .map(|i| f64::from(f.fwhm[i]) >= narrow && f64::from(f.snr[i]) >= p.min_snr)
        .collect();
    if keep.iter().all(|&k| k) {
        return;
    }
    let mut next = vec![0usize; keep.len()];
    let mut out = FileFeatures {
        dt: f.dt,
        ..FileFeatures::default()
    };
    for (i, &k) in keep.iter().enumerate() {
        next[i] = out.len();
        if k {
            out.push_row(f, i);
        }
    }
    multi.retain_mut(|m| {
        let kept = keep[m.row];
        m.row = next[m.row];
        kept
    });
    *f = out;
}
