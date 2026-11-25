//! The checks that decide `detected` or `forced` for one peak of `quant.targets`, each with the number it measured and the limit it met or missed,
//! so the log can show the evidence of every rejection (value, threshold, shape numbers). The same list sets the row flags. A shape gate that is
//! off is still listed with its measured value and no limit, so a user can read what limit would have separated the peaks.

use super::params::Params;
use super::run::Hit;

/// One check: what was measured, the limit, and whether the peak passed. `min` limits are `value >= limit`, the others `value <= limit`.
/// `limit` is `None` for a shape gate that is off (the value is listed, nothing is judged).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Check {
    pub name: &'static str,
    /// `None`: the trace gave no number (an asymmetry on a peak that does not come down inside its limits); such a check passes.
    pub value: Option<f64>,
    pub limit: Option<f64>,
    pub min: bool,
    pub pass: bool,
}

/// The flag a failed check sets: the S/N one is `below_threshold`, every shape one `bad_shape`.
pub(crate) fn flag_of(name: &str) -> &'static str {
    if name == "sn" {
        "below_threshold"
    } else {
        "bad_shape"
    }
}

/// The checks of `h` under `p`, in a fixed order: S/N, points and shape (always judged), then the three shape gates (judged only when set).
pub(crate) fn checks(h: &Hit, p: &Params) -> Vec<Check> {
    // a limit of 0 is a gate that is off: the value is kept for the evidence and the check passes
    let min = |name, v: f64, limit: f64, on: bool| Check {
        name,
        value: Some(v),
        limit: on.then_some(limit),
        min: true,
        pass: !on || v >= limit,
    };
    let max = |name, v: Option<f64>, limit: f64, on: bool| Check {
        name,
        value: v,
        limit: on.then_some(limit),
        min: false,
        pass: !on || v.is_none_or(|v| v <= limit),
    };
    // the larger of right/left and left/right, so a front and a tail of the same weight both count
    let asym = h.asym.map(|a| {
        let a = if a > 0.0 {
            a.max(1.0 / a).min(9999.0)
        } else {
            9999.0
        };
        (a * 100.0).round() / 100.0
    });
    vec![
        min("sn", h.sn, p.min_sn, true),
        min("points", h.points as f64, p.min_points as f64, true),
        min("shape", h.shape, p.min_shape, true),
        min(
            "half_pts",
            h.half_pts as f64,
            p.min_half as f64,
            p.min_half > 0,
        ),
        max("fwhm_s", Some(h.fwhm_s), p.max_fwhm, p.max_fwhm > 0.0),
        max("asym", asym, p.max_asym, p.max_asym > 0.0),
    ]
}
