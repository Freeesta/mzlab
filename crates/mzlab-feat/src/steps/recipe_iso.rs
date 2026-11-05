//! The isotope grouping of `features.detect` on the recipe: `isotopes` (`averagine`, the default of `isotopes.rs`, or `grouper`, the isotope
//! grouper of mzmine) and the `iso_*` parameters of the grouper (`Steps::grouper`, `align_mzmine::IsoGrouper`).

use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;

use crate::recipe::Recipe;

/// Parameters of `grouper`, besides `isotopes`.
const KEYS: [&str; 6] = [
    "iso_ppm",
    "iso_abs_da",
    "iso_rt_tol",
    "iso_max_charge",
    "iso_monotonic",
    "iso_representative",
];

fn bad(step: &str, name: &str) -> Error {
    Error::new("err.flow.param.type")
        .with("instance", step)
        .with("param", name)
}

/// Whether `k` of `features.detect` is the isotope grouping or one of its parameters.
pub(super) fn owns(k: &str) -> bool {
    k == "isotopes" || KEYS.contains(&k)
}

/// Writes `isotopes` or a parameter of the grouper; while `averagine` is chosen the settings are kept and change nothing.
pub(super) fn set(r: &mut Recipe, step: &str, k: &str, v: &Value) -> Result<()> {
    if k == "isotopes" {
        r.steps.use_grouper = match v.str() {
            Some("averagine") => false,
            Some("grouper") => true,
            _ => return Err(bad(step, k)),
        };
        return Ok(());
    }
    let g = &mut r.steps.grouper;
    match k {
        "iso_monotonic" => g.monotonic = v.bool().ok_or_else(|| bad(step, k))?,
        "iso_representative" => {
            g.lowest_mz = match v.str() {
                Some("most_intense") => false,
                Some("lowest_mz") => true,
                _ => return Err(bad(step, k)),
            }
        }
        _ => {
            let x = v
                .f64()
                .filter(|x| x.is_finite() && *x >= 0.0)
                .ok_or_else(|| bad(step, k))?;
            match k {
                "iso_ppm" => g.tol_ppm = x,
                "iso_abs_da" => g.tol_da = x,
                "iso_rt_tol" => g.rt_tol = x,
                _ => g.max_charge = (x as u8).clamp(1, 3),
            }
        }
    }
    Ok(())
}

/// The isotope grouping and the settings of the grouper.
pub(super) fn values(r: &Recipe, v: &mut Vec<(&'static str, Value)>) {
    let g = &r.steps.grouper;
    let iso = if r.steps.use_grouper {
        "grouper"
    } else {
        "averagine"
    };
    let rep = if g.lowest_mz {
        "lowest_mz"
    } else {
        "most_intense"
    };
    v.extend([
        ("isotopes", Value::Str(iso.into())),
        ("iso_ppm", Value::Num(g.tol_ppm)),
        ("iso_abs_da", Value::Num(g.tol_da)),
        ("iso_rt_tol", Value::Num(g.rt_tol)),
        ("iso_max_charge", Value::Num(f64::from(g.max_charge))),
        ("iso_monotonic", Value::Bool(g.monotonic)),
        ("iso_representative", Value::Str(rep.into())),
    ]);
}
