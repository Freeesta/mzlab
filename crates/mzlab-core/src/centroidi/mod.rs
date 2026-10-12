//! Centroids of a profile scan: every peak becomes one point (m/z, area or height) with flags. One scan at a time, f64, no new dependency, no hash map
//! iterated, ties go to the lowest index. Parity with `tools/centroidi_riferimento.py` (same operations in the same order): `tests/centroidi_parity.rs`.
//!
//! The steps, in order: the points are laid on a regular grid with zeros between the first and the last point (`grid`), smoothed with a quadratic
//! Savitzky-Golay filter (`smooth`), a background is taken as the 25th percentile of the smoothed signal (`smooth::background`), the maxima that stand above
//! it are candidates, adjacent ones are merged unless the valley between them is significant (`detect`), and each region becomes a centroid if it passes
//! the threshold of its mode (counts: Poisson Z; continuous: height over k sigma) (`emit`). `params` estimates the parameters from a few scans.

mod detect;
mod emit;
mod grid;
mod params;
mod shoulder;
mod smooth;

#[cfg(test)]
mod tests;

pub use params::{estimate, Estimate};

/// How the intensities of the file are made: whole multiples of one unit (detector counts) or continuous values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Counts,
    Continuous,
}

/// Flags of a centroid.
pub const TRUNCATED: u8 = 1;
pub const SATURATED: u8 = 2;
pub const ASYMMETRIC: u8 = 4;
pub const SHOULDER: u8 = 8;

#[derive(Clone, Debug)]
pub struct Params {
    pub mode: Mode,
    /// Counts mode: the value of one count (intensity = count * unit).
    pub unit: f64,
    /// Typical distance between two points of the grid, Da.
    pub step: f64,
    /// FWHM of a peak, Da.
    pub w: f64,
    /// Threshold in sigma (Poisson Z in counts mode).
    pub k: f64,
    /// Highest intensity of the file (saturation); 0 = not known.
    pub vmax: f64,
    /// The value written for a centroid: the area (counts, low resolution) or the height (high resolution).
    pub area: bool,
}

impl Params {
    /// Width of a peak in grid points (at least 3).
    pub fn wb(&self) -> usize {
        ((self.w / self.step).round() as usize).max(3)
    }
    /// High resolution: continuous values with at least 7 points per FWHM.
    pub fn hr(&self) -> bool {
        self.mode == Mode::Continuous && self.wb() >= 7
    }
    /// Length of the Savitzky-Golay window: the odd number nearest to 0.7 w / step in [5, 15]; 5 when a continuous peak has fewer than 7 points.
    pub fn sg_len(&self) -> usize {
        if self.mode == Mode::Continuous && self.wb() < 7 {
            return 5;
        }
        let l = (0.7 * self.wb() as f64).round() as usize;
        let l = if l.is_multiple_of(2) { l + 1 } else { l };
        l.clamp(5, 15)
    }
}

/// One peak of a scan.
#[derive(Clone, Debug, PartialEq)]
pub struct Centroid {
    pub mz: f64,
    /// What is written in the file (`Params::area`).
    pub value: f64,
    /// Sum of the raw intensities in the region (no multiplication by the step).
    pub area: f64,
    /// Smoothed intensity at the apex.
    pub height: f64,
    pub flags: u8,
}

/// Buffers reused from one scan to the next.
#[derive(Default)]
pub struct Buf {
    pub(crate) x: Vec<f64>,
    pub(crate) v: Vec<f64>,
    pub(crate) s: Vec<f64>,
    pub(crate) b: Vec<f64>,
    pub(crate) tmp: Vec<f64>,
    pub(crate) pad: Vec<f64>,
}

/// The centroids of one scan (m/z ascending), from its non-zero points.
pub fn centroids(mz: &[f64], y: &[f64], p: &Params, buf: &mut Buf) -> Vec<Centroid> {
    grid::fill(mz, y, p, buf);
    if buf.v.len() < 3 {
        return Vec::new();
    }
    smooth::smooth(buf, p.sg_len());
    smooth::background(buf, p.wb(), p.mode == Mode::Counts);
    let sigma = if p.mode == Mode::Continuous {
        smooth::noise(buf, p.wb())
    } else {
        0.0
    };
    let groups = detect::groups(buf, p, sigma);
    let mut out = emit::emit(buf, p, sigma, &groups);
    out.sort_by(|a, b| a.mz.total_cmp(&b.mz));
    out
}
