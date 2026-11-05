//! The algorithms of `features.align` (`groups`, `join`) and `features.gapfill` (`integrate`, `peak_finder`) on the recipe: the part of
//! `recipe_map` for `Recipe::join` and `Recipe::peak_finder` and their parameters (`align_mzmine`).

use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;

use crate::align_mzmine::{JoinAlign, PeakFinder};
use crate::recipe::Recipe;

/// Parameters of `join` (`features.align`) and of `peak_finder` (`features.gapfill`), besides `algorithm`.
const JOIN_KEYS: [&str; 4] = ["mz_weight", "rt_weight", "rt_tol", "abs_da"];
const FINDER_KEYS: [&str; 2] = ["intensity_tol", "abs_da"];

fn bad(step: &str, name: &str) -> Error {
    Error::new("err.flow.param.type")
        .with("instance", step)
        .with("param", name)
}

fn unknown(step: &str, name: &str) -> Error {
    Error::new("err.flow.param.unknown")
        .with("instance", step)
        .with("param", name)
}

/// Whether `k` of `step` is the algorithm or a parameter of an algorithm of `features.align` or `features.gapfill`.
pub(super) fn owns(step: &str, k: &str) -> bool {
    match step {
        "features.align" => k == "algorithm" || JOIN_KEYS.contains(&k),
        "features.gapfill" => k == "algorithm" || FINDER_KEYS.contains(&k),
        _ => false,
    }
}

/// Writes `algorithm` or a parameter of the chosen algorithm of `features.align` / `features.gapfill`; a parameter of an algorithm that is not
/// chosen is `err.flow.param.unknown`.
pub(super) fn set(r: &mut Recipe, step: &str, k: &str, v: &Value) -> Result<()> {
    let align = step == "features.align";
    if k == "algorithm" {
        match (align, v.str()) {
            (true, Some("groups")) => r.join = None,
            (true, Some("join")) => r.join = Some(JoinAlign::default()),
            (false, Some("integrate")) => r.peak_finder = None,
            (false, Some("peak_finder")) => r.peak_finder = Some(PeakFinder::default()),
            _ => return Err(bad(step, k)),
        }
        return Ok(());
    }
    let x = v
        .f64()
        .filter(|x| x.is_finite() && *x >= 0.0)
        .ok_or_else(|| bad(step, k))?;
    match (align, r.join.as_mut(), r.peak_finder.as_mut(), k) {
        (true, Some(j), _, "mz_weight") => j.mz_weight = x,
        (true, Some(j), _, "rt_weight") => j.rt_weight = x,
        (true, Some(j), _, "rt_tol") => j.rt_tol = x,
        (true, Some(j), _, "abs_da") => j.tol_da = x,
        (false, _, Some(f), "intensity_tol") => f.intensity_tol = x,
        (false, _, Some(f), "abs_da") => f.tol_da = x,
        _ => return Err(unknown(step, k)),
    }
    Ok(())
}

/// The values of the parameters of the chosen algorithm of `step` (none for the default one).
pub(super) fn values(r: &Recipe, step: &str, v: &mut Vec<(&'static str, Value)>) {
    let n = Value::Num;
    match (step, &r.join, &r.peak_finder) {
        ("features.align", Some(j), _) => v.extend([
            ("mz_weight", n(j.mz_weight)),
            ("rt_weight", n(j.rt_weight)),
            ("rt_tol", n(j.rt_tol)),
            ("abs_da", n(j.tol_da)),
        ]),
        ("features.gapfill", _, Some(f)) => v.extend([
            ("intensity_tol", n(f.intensity_tol)),
            ("abs_da", n(f.tol_da)),
        ]),
        _ => {}
    }
}
