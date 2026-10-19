//! The registry of filters `filtri/1`: F1-F4 are read from the `.mzfeat` (quality, isotopes, controls, in-source fragments), F5 is the mass error of
//! the formula against the calibration of the file. A filter hides a row with a reason, it never deletes one; each row carries the first filter that
//! hid it. Used by `interp::build`.

use crate::legacy::series::Hyp;
use crate::tp::{ST_CONTROL, ST_ISF, ST_PARENT};

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

/// Mass defect added to the delta of a rule (Da) to make a decoy, times k = 1..=`DECOY_K`; a decoy match is a match by chance.
pub const DECOY_STEP: f64 = 0.0137;
pub const DECOY_K: u32 = 5;
/// A decoy must lie beyond this many ppm of every true delta, and a row matches a decoy within it.
pub const DECOY_PPM: f64 = 10.0;

/// The decoy m/z: the ion m/z of each rule shifted by `DECOY_STEP` x k, dropping the ones within `DECOY_PPM` of any rule (a true match).
pub fn decoy_masses(rule_mz: &[f64]) -> Vec<f64> {
    let near = |a: f64, b: f64| (a - b).abs() / b.abs().max(1.0) * 1e6 <= DECOY_PPM;
    let mut out: Vec<f64> = vec![];
    for &m in rule_mz {
        for k in 1..=DECOY_K {
            let d = m + DECOY_STEP * f64::from(k);
            if !rule_mz.iter().any(|&r| near(d, r)) && !out.iter().any(|&o| near(d, o)) {
                out.push(d);
            }
        }
    }
    out
}

/// `decoys` of the registry: how many decoys there are and how many of them are matched by a row that passes F1-F5. `rows` = (m/z, passes).
pub fn registry_decoys(
    rule_mz: &[f64],
    rows: impl IntoIterator<Item = (f64, bool)>,
) -> Option<(usize, usize)> {
    let (masses, rows) = (decoy_masses(rule_mz), rows.into_iter().collect::<Vec<_>>());
    let hit = |d: f64| {
        rows.iter()
            .any(|&(mz, ok)| ok && (mz - d).abs() / d * 1e6 <= DECOY_PPM)
    };
    (!masses.is_empty()).then(|| (masses.len(), masses.iter().filter(|&&d| hit(d)).count()))
}

/// `registry_decoys` over a table: the rows that are not the parent, with their `mz`, `status` and the facts the filters read.
pub fn table_decoys(
    hyps: &[Hyp],
    mz: &[f64],
    status: &[u8],
    facts: &[RowFacts],
) -> Option<(usize, usize)> {
    let rule_mz: Vec<f64> = hyps.iter().map(|h| h.mz).collect();
    let rows = (0..mz.len()).filter(|&r| status[r] != ST_PARENT);
    registry_decoys(
        &rule_mz,
        rows.map(|r| (mz[r], first_hidden(&facts[r]).is_none())),
    )
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
