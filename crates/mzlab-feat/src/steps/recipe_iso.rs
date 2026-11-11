//! The isotope grouping of `features.detect` on the recipe: `isotopes` (`averagine`, the default of `isotopes.rs`; `grouper`, the isotope grouper
//! of mzmine; `finder`, its isotope finder), the `iso_*` parameters of the grouper (`align_mzmine::IsoGrouper`) and the `finder_*` ones of the
//! finder (`align_mzmine::IsoFinder`).

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

/// Parameters of `finder`.
const FINDER_KEYS: [&str; 6] = [
    "finder_ppm",
    "finder_abs_da",
    "finder_max_charge",
    "finder_elements",
    "finder_require_c13",
    "finder_auto",
];

/// `err.flow.param.type` of parameter `name` of `step`, with the value it was given.
fn bad(step: &str, name: &str, v: &Value) -> Error {
    Error::new("err.flow.param.type")
        .with("instance", step)
        .with("param", name)
        .with("detail", mzlab_core::flow::value_text(v))
}

/// Whether `k` of `features.detect` is the isotope grouping or one of its parameters.
pub(super) fn owns(k: &str) -> bool {
    k == "isotopes" || KEYS.contains(&k) || FINDER_KEYS.contains(&k)
}

/// Writes `isotopes` or a parameter of the grouper or the finder; the settings of the groupings not chosen are kept and change nothing.
pub(super) fn set(r: &mut Recipe, step: &str, k: &str, v: &Value) -> Result<()> {
    if k == "isotopes" {
        (r.steps.use_grouper, r.steps.use_finder) = match v.str() {
            Some("averagine") => (false, false),
            Some("grouper") => (true, false),
            Some("finder") => (false, true),
            _ => return Err(bad(step, k, v)),
        };
        return Ok(());
    }
    if FINDER_KEYS.contains(&k) {
        return set_finder(r, step, k, v);
    }
    let g = &mut r.steps.grouper;
    match k {
        "iso_monotonic" => g.monotonic = v.bool().ok_or_else(|| bad(step, k, v))?,
        "iso_representative" => {
            g.lowest_mz = match v.str() {
                Some("most_intense") => false,
                Some("lowest_mz") => true,
                _ => return Err(bad(step, k, v)),
            }
        }
        _ => {
            let x = v
                .f64()
                .filter(|x| x.is_finite() && *x >= 0.0)
                .ok_or_else(|| bad(step, k, v))?;
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

/// A parameter of the finder; the elements must all have natural isotopes.
fn set_finder(r: &mut Recipe, step: &str, k: &str, v: &Value) -> Result<()> {
    let g = &mut r.steps.finder;
    match k {
        "finder_require_c13" => g.require_c13 = v.bool().ok_or_else(|| bad(step, k, v))?,
        "finder_auto" => g.auto = v.bool().ok_or_else(|| bad(step, k, v))?,
        "finder_elements" => {
            let list = v.str().ok_or_else(|| bad(step, k, v))?;
            let els: Vec<&str> = list
                .split(',')
                .map(str::trim)
                .filter(|e| !e.is_empty())
                .collect();
            if els.is_empty()
                || els
                    .iter()
                    .any(|e| mzlab_core::isoscore::isotopes_of(e).is_none())
            {
                return Err(bad(step, k, v));
            }
            g.elements = els.join(",");
        }
        _ => {
            let x = v
                .f64()
                .filter(|x| x.is_finite() && *x >= 0.0)
                .ok_or_else(|| bad(step, k, v))?;
            match k {
                "finder_ppm" => g.tol_ppm = x,
                "finder_abs_da" => g.tol_da = x,
                _ => g.max_charge = (x as u8).clamp(1, 5),
            }
        }
    }
    Ok(())
}

/// The isotope grouping and the settings of the grouper and the finder.
pub(super) fn values(r: &Recipe, v: &mut Vec<(&'static str, Value)>) {
    let g = &r.steps.grouper;
    let iso = if r.steps.use_finder {
        "finder"
    } else if r.steps.use_grouper {
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
    let h = &r.steps.finder;
    v.extend([
        ("finder_ppm", Value::Num(h.tol_ppm)),
        ("finder_abs_da", Value::Num(h.tol_da)),
        ("finder_max_charge", Value::Num(f64::from(h.max_charge))),
        ("finder_elements", Value::Str(h.elements.clone())),
        ("finder_require_c13", Value::Bool(h.require_c13)),
        ("finder_auto", Value::Bool(h.auto)),
    ]);
}
