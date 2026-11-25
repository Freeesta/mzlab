//! The checks that decide `detected` or `forced` for one peak of `quant.targets`, each with the number it measured and the limit it met or missed,
//! so the log can show the evidence of every rejection (value, threshold, shape numbers). The same list sets the row flags.

use super::params::Params;
use super::run::Hit;

/// One check: what was measured, the limit, and whether the peak passed. `min` limits are `value >= limit`, the others `value <= limit`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Check {
    pub name: &'static str,
    /// `None`: the trace gave no number (an asymmetry on a peak that does not come down inside its limits); such a check passes.
    pub value: Option<f64>,
    pub limit: f64,
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

/// The checks of `h` under `p`, in a fixed order; a shape gate that is off (0) is not listed.
pub(crate) fn checks(h: &Hit, p: &Params) -> Vec<Check> {
    let min = |name, v: f64, limit: f64| Check {
        name,
        value: Some(v),
        limit,
        min: true,
        pass: v >= limit,
    };
    let max = |name, v: Option<f64>, limit: f64| Check {
        name,
        value: v,
        limit,
        min: false,
        pass: v.is_none_or(|v| v <= limit),
    };
    let mut c = vec![
        min("sn", h.sn, p.min_sn),
        min("points", h.points as f64, p.min_points as f64),
        min("shape", h.shape, p.min_shape),
    ];
    if p.min_half > 0 {
        c.push(min("half_pts", h.half_pts as f64, p.min_half as f64));
    }
    if p.max_fwhm > 0.0 {
        c.push(max("fwhm_s", Some(h.fwhm_s), p.max_fwhm));
    }
    if p.max_asym > 0.0 {
        // the larger of right/left and left/right, so a front and a tail of the same weight both count
        let a = h.asym.map(|a| {
            if a > 0.0 {
                a.max(1.0 / a).min(9999.0)
            } else {
                9999.0
            }
        });
        c.push(max(
            "asym",
            a.map(|a| (a * 100.0).round() / 100.0),
            p.max_asym,
        ));
    }
    c
}
