//! The minimum height of a feature proposed from the noise lump of the intensities (`params::noise_from_hist`): used by `Recipe::for_index` at high
//! resolution, by the preview and by the Flow steps (the proof of the estimate).

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Out;

use crate::params::{intensity_hist, noise_from_hist, Noise};

/// The minimum height of a feature is this many times the mode of the noise lump (the weak tier goes down to 3 times, `recipe::WEAK_RATIO`).
pub const MIN_HEIGHT_FACTOR: f64 = 10.0;

/// The minimum height proposed from the noise lump: `MIN_HEIGHT_FACTOR` times its mode, to two figures and never below the default
/// (the estimate only raises the minimum, so a file with a low noise keeps the canonical recipe).
pub fn min_height_from_noise(n: &Noise) -> f64 {
    let raw = MIN_HEIGHT_FACTOR * n.mode;
    let unit = libm::pow(10.0, libm::floor(libm::log10(raw.max(1.0))) - 1.0);
    (libm::round(raw / unit) * unit).max(crate::trace::MIN_HEIGHT)
}

/// The noise lump of the MS1 section `si` and the minimum height it gives; `None` when the histogram has no lump.
pub fn estimated_min_height<R: ReadAt>(
    index: &mut Index<R>,
    si: usize,
) -> Result<Option<(f64, Noise)>> {
    let h = intensity_hist(index, si)?;
    Ok(noise_from_hist(&h).map(|n| (min_height_from_noise(&n), n)))
}

/// The `min_height` member of the preview: `{value, factor, mode}`, `null` without a noise lump.
pub fn min_height_json(o: &mut Out, noise: Option<&Noise>) {
    o.raw(",\"min_height\":");
    match noise {
        Some(n) => {
            o.raw("{\"value\":")
                .num(min_height_from_noise(n))
                .raw(",\"factor\":")
                .num(MIN_HEIGHT_FACTOR);
            o.raw(",\"mode\":").num(n.mode).raw("}");
        }
        None => {
            o.raw("null");
        }
    }
}
