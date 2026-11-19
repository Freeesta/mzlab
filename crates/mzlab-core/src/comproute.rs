//! `api/composition` from its query string: the formulas that fit an exact m/z (`_r_composition` of `mzlab/api.py`). The query is read
//! like Python reads it; anything Python would reject (a missing or bad number, an element range that does not parse, a request that
//! `compose` refuses) is `None`, so that Python words the error.

use crate::composition::{compose, parse_elements, Opts};
use crate::routes::Query;

/// A number that is present (even empty is an error, as `float("")` is) or a default.
fn float(q: &Query, k: &str, d: f64) -> Option<f64> {
    q.get(k).map_or(Some(d), |s| s.trim().parse().ok())
}

/// A number that counts only when the value is not empty (`float(q[k]) if q.get(k) else None`).
fn maybe(q: &Query, k: &str) -> Option<Option<f64>> {
    match q.some(k) {
        None => Some(None),
        Some(s) => s.trim().parse().ok().map(Some),
    }
}

/// `mz:intensity;mz:intensity` (`_peaks`): at most 2000 pairs; None when a part is not a pair of numbers.
fn peaks(text: Option<&str>) -> Option<Vec<(f64, f64)>> {
    let mut out = Vec::new();
    for part in text.unwrap_or("").split(';').filter(|p| !p.is_empty()) {
        let (a, b) = part.split_once(':')?;
        out.push((a.trim().parse().ok()?, b.trim().parse().ok()?));
    }
    out.truncate(2000);
    Some(out)
}

/// `_weights`: `1,1,0.5` = (mass, isotopes, fragments); missing or bad = 1, 2, 1.
fn weights(text: Option<&str>) -> (f64, f64, f64) {
    let given: Option<Vec<f64>> = text.unwrap_or("").split(',').map(|x| x.trim().parse::<f64>().ok().map(|v| v.max(0.0))).collect();
    let w = given.unwrap_or_default();
    let at = |i: usize, d: f64| w.get(i).copied().unwrap_or(d);
    (at(0, 1.0), at(1, 2.0), at(2, 1.0))
}

/// The JSON of `/api/composition`, or None when Python must answer.
pub fn composition_route(q: &Query) -> Option<String> {
    let mz: f64 = q.get("mz")?.trim().parse().ok()?;
    let rdb: Option<Vec<f64>> = q.some("rdb").unwrap_or("-1,100").split(',').map(|x| x.trim().parse().ok()).collect();
    let rdb = rdb?;
    if rdb.len() < 2 {
        return None;
    }
    let elements = match q.some("elements") {
        Some(t) => Some(parse_elements(t).ok()?),
        None => None,
    };
    let z = match q.some("z") {
        Some(s) => Some(s.trim().parse::<i32>().ok()?),
        None => None,
    };
    let max_results = q.get("max").map_or(Some(10), |s| s.trim().parse::<i64>().ok())?.min(200);
    let o = Opts {
        elements,
        ion: q.some("ion").unwrap_or("[M+H]+"),
        z,
        tol: float(q, "tol", 5.0)?,
        unit: q.get("unit").unwrap_or("ppm"),
        rdb: (rdb[0], rdb[1]),
        nitrogen: q.get("n").unwrap_or("none"),
        rules: q.some("rules").unwrap_or("").split(',').filter(|x| !x.is_empty()).map(String::from).collect(),
        max_results: usize::try_from(max_results).ok()?,
        parent: q.some("parent"),
        m1: maybe(q, "m1")?,
        m2: maybe(q, "m2")?,
        iso_tol: float(q, "isotol", 0.2)?,
        obs: peaks(q.some("obs"))?,
        frags: peaks(q.some("frags"))?,
        r200: float(q, "r200", 70000.0)?,
        sigma_ppm: float(q, "sppm", 3.0)?,
        sigma_int: float(q, "sint", 0.10)?,
        weights: weights(q.some("w")),
        iso_min_rel: float(q, "imin", 1.0)?,
        iso_noise_rel: float(q, "inoise", 1.0)?,
        frag_ppm: maybe(q, "fppm")?,
        norm: q.some("norm").filter(|n| matches!(*n, "mono" | "max" | "sum")).unwrap_or("fit"),
        ..Opts::default()
    };
    compose(mz, &o).ok()
}
