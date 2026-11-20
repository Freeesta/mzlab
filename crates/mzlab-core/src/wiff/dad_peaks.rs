//! The peaks of the DAD trace with the wavelength of their maximum (`dad peaks`): the total-wavelength trace (or a band of wavelengths) is searched with the
//! same peak finder as the MRM traces, and each peak gets the UV spectrum of its apex, read against the two rows at its edges, so the lambda-max is the
//! analyte's and not the solvent's. The MS side (ions at the delay-corrected time) is in `pdapeaks.rs`.

use super::dad::DadData;
use super::dad_uv::window_cols;
use crate::peak1d::{peaks, Found};

/// What is searched: the band of wavelengths of the trace (none: the total trace), at most `max` peaks, with S/N of at least `snr_min`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeakOpts {
    pub nm: Option<(f64, f64)>,
    pub max: usize,
    pub snr_min: f64,
}

impl Default for PeakOpts {
    fn default() -> Self {
        Self {
            nm: None,
            max: 20,
            snr_min: 3.0,
        }
    }
}

/// Points a peak must span and the smoothing of the search: the values of the `peak` commands.
const SMOOTH: usize = 3;
const MIN_POINTS: usize = 5;

/// One UV peak: where and how big (`rt_min`, `height` in mAU of the trace), its extent, and the wavelength of the maximum of its spectrum.
#[derive(Debug, Clone, PartialEq)]
pub struct UvPeak {
    pub rt_min: f64,
    pub height: f64,
    pub area: f64,
    pub snr: f64,
    pub left: f64,
    pub right: f64,
    pub lambda_max_nm: f64,
    /// Absorbance at the lambda-max, in mAU, after the edge rows are taken off.
    pub abs_max: f64,
}

/// The trace the peaks are searched on: the total trace of the file (a mean over the channels when the file keeps none) or the mean of the band.
fn trace(d: &DadData, nm: Option<(f64, f64)>) -> Vec<f64> {
    let cols = nm.and_then(|(a, b)| window_cols(d, a.min(b), a.max(b)));
    if cols.is_none() && d.twc_mau.len() == d.points() && d.twc_mau.iter().any(|v| *v != 0.0) {
        return d.twc_mau.clone();
    }
    let all: Vec<usize> = (0..d.channels()).collect();
    let cols = cols.unwrap_or(all);
    (0..d.points())
        .map(|i| {
            let row = d.spectrum_at(i).unwrap_or_default();
            let n = cols.len().max(1) as f64;
            cols.iter()
                .filter_map(|&j| row.get(j))
                .map(|&v| f64::from(v))
                .sum::<f64>()
                / n
        })
        .collect()
}

/// The row nearest to `t`.
fn row_at(d: &DadData, t: f64) -> usize {
    d.time_min
        .partition_point(|&x| x < t)
        .min(d.points().saturating_sub(1))
}

/// The apex spectrum minus the mean of the rows at the two edges of the peak: `(lambda_max, absorbance there)`; `None` for an empty matrix.
fn lambda_max(d: &DadData, f: &Found) -> Option<(f64, f64)> {
    let (apex, l, r) = (row_at(d, f.apex_rt), row_at(d, f.left), row_at(d, f.right));
    let (a, bl, br) = (d.spectrum_at(apex)?, d.spectrum_at(l)?, d.spectrum_at(r)?);
    let diff = |j: usize| {
        let v = |s: &[f32]| s.get(j).map_or(0.0, |x| f64::from(*x));
        v(a) - (v(bl) + v(br)) / 2.0
    };
    (0..d.channels())
        .map(|j| (j, diff(j)))
        .max_by(|x, y| x.1.total_cmp(&y.1))
        .and_then(|(j, v)| Some((*d.wavelengths_nm.get(j)?, v)))
}

/// The UV peaks of `d`, in the order of their retention time; `None` for a matrix with no rows or no channels. A trace with no peak is an empty list.
pub fn uv_peaks(d: &DadData, o: &PeakOpts) -> Option<Vec<UvPeak>> {
    if d.points() < 3 || d.channels() == 0 {
        return None;
    }
    let y = trace(d, o.nm);
    let mut found = peaks(&d.time_min, &y, SMOOTH, MIN_POINTS, o.snr_min, o.max);
    crate::sortc::slice_by(&mut found, |a, b| a.apex_rt.total_cmp(&b.apex_rt));
    Some(
        found
            .iter()
            .filter_map(|f| {
                let (nm, abs) = lambda_max(d, f)?;
                Some(UvPeak {
                    rt_min: f.apex_rt,
                    height: f.height,
                    area: f.area,
                    snr: f.snr,
                    left: f.left,
                    right: f.right,
                    lambda_max_nm: nm,
                    abs_max: abs,
                })
            })
            .collect(),
    )
}

#[cfg(test)]
#[path = "dad_peaks_tests.rs"]
mod tests;
