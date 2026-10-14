//! The mass axis of the detection: log-ppm channels for high resolution, channels of 1 Da for unit resolution (the `lr` profile).
//!
//! On the unit axis a channel is centred on a nominal mass shifted by the mass defect of the file (`Da { defect }`, the circular mode of the fractional
//! parts of the strong peaks, estimated in the preview), the two grids are shifted by 0.5 Da and every tolerance of m/z is in Da (`LR_TOL`, 0.35 Da).
//! A tolerance of m/z is one number: positive = ppm, negative = minus the tolerance in Da (`da(0.35)`); `half(mz, tol)` is the half window in Da. So the
//! parameters, their text in the recipe and the tables of the high-resolution profile stay as they were.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

/// Tolerance of m/z of the unit axis, Da (the one of the former low-resolution engine).
pub const LR_TOL: f64 = 0.35;
/// Distance, Da, under which two pieces of a trace in neighbouring channels, or the same ion on the two grids, are one ion.
pub const LR_TOUCH: f64 = 0.1;
/// Largest distance, Da, between the centroids of two pieces of one peak that straddles the border of two unit channels.
pub const LR_STITCH: f64 = 0.2;
/// Window of the trace of an ion in the quality, the families and the isotopes of the unit axis, Da.
pub const LR_TRACE: f64 = 0.3;

/// The axis of the channels of m/z.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MassAxis {
    /// Channels of 4 ppm on a logarithmic axis (Orbitrap, Q-TOF).
    Ppm,
    /// Channels of 1 Da centred on `n + defect`.
    Da { defect: f64 },
}

impl MassAxis {
    pub fn is_da(&self) -> bool {
        matches!(self, MassAxis::Da { .. })
    }
    /// Position of `mz` on the axis in channel units (the channel of a grid is `floor(x + shift)`).
    pub(crate) fn coord(&self, mz: f64, den: f64) -> f64 {
        match self {
            MassAxis::Ppm => libm::log(mz) / den,
            MassAxis::Da { defect } => mz - defect + 0.5,
        }
    }
}

/// A tolerance of `x` Da (see the module).
pub const fn da(x: f64) -> f64 {
    -x
}

/// Half window of m/z around `mz` for the tolerance `tol` (ppm if positive, Da if negative).
#[inline]
pub fn half(mz: f64, tol: f64) -> f64 {
    if tol < 0.0 {
        -tol
    } else {
        mz * tol * 1e-6
    }
}

/// Mass defect of a file: the fractional part of the m/z (in [-0.5, 0.5)) where the strong peaks pile up, as the mode of a histogram of 50 bins
/// (peaks sampled at even distances, weighted by the logarithm of the intensity above `floor`) refined with the circular mean of the peaks within
/// 0.1 Da of it. `None` without peaks above the floor.
pub fn estimate_defect<R: ReadAt>(
    index: &mut Index<R>,
    si: usize,
    floor: f64,
) -> Result<Option<f64>> {
    const BINS: usize = 50;
    const PIECE: u64 = 1 << 16;
    const PIECES: u64 = 32;
    let n = index.sections[si].n_peaks;
    let mut frac: Vec<(f64, f64)> = Vec::new();
    let mut take = |index: &mut Index<R>, a: u64, b: u64| {
        index.each_peak(si, a, b, |_p, mz, it| {
            if it >= floor {
                frac.push((mz - libm::round(mz), libm::log10(it / floor) + 1.0));
            }
        })
    };
    if n <= PIECE * PIECES {
        take(index, 0, n)?;
    } else {
        for k in 0..PIECES {
            let a = (n - PIECE) / (PIECES - 1) * k;
            take(index, a, a + PIECE)?;
        }
    }
    if frac.is_empty() {
        return Ok(None);
    }
    let mut h = [0.0f64; BINS];
    for &(f, w) in &frac {
        h[(((f + 0.5) * BINS as f64) as usize).min(BINS - 1)] += w;
    }
    // the highest bin of a circular window of three bins (the first on ties)
    let sum3 = |i: usize| h[(i + BINS - 1) % BINS] + h[i] + h[(i + 1) % BINS];
    let mode = (0..BINS).fold(0, |b, i| if sum3(i) > sum3(b) { i } else { b });
    let centre = (mode as f64 + 0.5) / BINS as f64 - 0.5;
    let (mut s, mut c, mut wsum) = (0.0f64, 0.0f64, 0.0f64);
    for &(f, w) in &frac {
        let d = f - centre;
        let d = d - libm::round(d);
        if d.abs() <= 0.1 {
            let th = 2.0 * core::f64::consts::PI * f;
            s += w * libm::sin(th);
            c += w * libm::cos(th);
            wsum += w;
        }
    }
    if wsum <= 0.0 {
        return Ok(Some(centre));
    }
    Ok(Some(libm::atan2(s, c) / (2.0 * core::f64::consts::PI)))
}
