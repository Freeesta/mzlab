//! The registry of filters `filtri/1`: F1-F4 are read from the `.mzfeat` (quality, isotopes, controls, in-source fragments), F5 is the mass error of
//! the formula against the calibration of the file. A filter hides a row with a reason, it never deletes one; each row carries the first filter that
//! hid it. Used by `interp::build`.

use crate::tp::{ST_CONTROL, ST_ISF};

pub const SCHEMA: &str = "filtri/1";
/// Floor and ceiling of the tolerance of a formula, ppm.
pub const TOL_MIN: f64 = 3.0;
pub const TOL_MAX: f64 = 5.0;

/// Static description of a filter: id, name (English, shown in Tabui) and why.
pub const STAGES: [(&str, &str, &str); 5] = [
    ("F1", "Peak quality", "Shape, signal-to-noise or number of points of the peak are below the thresholds of the recipe."),
    ("F2", "Same molecule", "The ion is an isotopologue of another row, not a molecule of its own."),
    ("F3", "Formed by the treatment", "The ion is as strong in the controls (dark, t0, blank) as in the treated samples."),
    ("F4", "Not an in-source fragment", "The ion follows the parent at a constant ratio, so it is a fragment or adduct made in the source."),
    ("F5", "Mass error", "The error of the formula against the calibration of the file is larger than the tolerance of that file."),
];

/// What the filters read of a row.
#[derive(Debug, Clone, Copy, Default)]
pub struct RowFacts {
    /// `RowMeta::qreason` is not zero.
    pub bad_quality: bool,
    /// A non-first member of an isotope group.
    pub isotope: bool,
    /// `tp/1` status of the row.
    pub tp_status: u8,
    /// The error of the formula is beyond the tolerance of the file (set by `build`; false when it was not measured).
    pub mass_error: bool,
}

/// The first filter that hides the row, 0-based index into `STAGES`; `None` = the row passes.
pub fn first_hidden(f: &RowFacts) -> Option<usize> {
    if f.bad_quality {
        Some(0)
    } else if f.isotope {
        Some(1)
    } else if f.tp_status == ST_CONTROL {
        Some(2)
    } else if f.tp_status == ST_ISF {
        Some(3)
    } else if f.mass_error {
        Some(4)
    } else {
        None
    }
}

/// Median of a slice (NaN when empty).
pub fn median(v: &[f64]) -> f64 {
    let mut s: Vec<f64> = v.iter().copied().filter(|x| x.is_finite()).collect();
    s.sort_by(f64::total_cmp);
    match s.len() {
        0 => f64::NAN,
        n if n % 2 == 1 => s[n / 2],
        n => (s[n / 2 - 1] + s[n / 2]) / 2.0,
    }
}

/// The calibration of one file from the ppm errors of the parent and of its known fragments.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FileCal {
    pub median_ppm: f64,
    pub mad_ppm: f64,
    pub n: usize,
    /// max(3, 2 x MAD), never above 5 ppm.
    pub tolerance: f64,
}

pub fn calibrate(errors: &[f64]) -> FileCal {
    let med = median(errors);
    let mad = 1.4826 * median(&errors.iter().map(|e| (e - med).abs()).collect::<Vec<_>>());
    let tolerance = if mad.is_finite() {
        (2.0 * mad).clamp(TOL_MIN, TOL_MAX)
    } else {
        TOL_MIN
    };
    FileCal {
        median_ppm: if med.is_finite() { med } else { 0.0 },
        mad_ppm: if mad.is_finite() { mad } else { 0.0 },
        n: errors.len(),
        tolerance,
    }
}

/// Whether the error of a row (`ppm`, observed against the formula) is out of tolerance in the file of its best peak.
pub fn out_of_tolerance(ppm: f64, cal: &FileCal) -> bool {
    ppm.is_finite() && (ppm - cal.median_ppm).abs() > cal.tolerance
}

/// The registry: the ids (1-based) each filter hid, in the order of `STAGES`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Registry {
    pub hidden: [Vec<u32>; 5],
    /// Decoy masses passed through the filters (`decoys` of `filtri/1`): `(rules, survived)`, None until measured.
    pub decoys: Option<(usize, usize)>,
}
