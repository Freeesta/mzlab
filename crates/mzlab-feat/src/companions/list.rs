//! The nominal mass differences of the step, ESI+, relative to the lighter ion: adducts of [M+H]+ and in-source neutral losses of the heavier ion.
//! Both are user lists (`adducts`, `losses`); a difference outside the names below keeps its number as the name.

use super::Params;
use mzlab_core::error::{Error, Result};

pub const ADDUCTS: &str = "17,22,38,41,63";
pub const LOSSES: &str = "17,18,28,42,44,46";
pub const ADDUCT_NAMES: &[(i64, &str)] = &[
    (17, "adduct_nh4"),
    (22, "adduct_na"),
    (38, "adduct_k"),
    (41, "adduct_acn_h"),
    (63, "adduct_na_acn"),
];
pub const LOSS_NAMES: &[(i64, &str)] = &[
    (17, "loss_nh3"),
    (18, "loss_h2o"),
    (28, "loss_co"),
    (42, "loss_c2h2o"),
    (44, "loss_co2"),
    (46, "loss_hcooh"),
];

/// A mass difference (Da, heavier minus lighter) and the name of the relation it stands for.
#[derive(Debug, Clone, PartialEq)]
pub struct Rel {
    pub delta: f64,
    pub name: String,
}

/// The relations of a comma-separated list of positive numbers; a name from `names` when the number is one of them, else `adduct_N`/`loss_N`
/// (by the first name of `names`). An item that is not a positive number is `err.flow.companions.list`.
pub fn parse(list: &str, names: &[(i64, &str)]) -> Result<Vec<Rel>> {
    let prefix = if names.first().is_some_and(|n| n.1.starts_with("loss_")) {
        "loss_"
    } else {
        "adduct_"
    };
    let mut out = Vec::new();
    for item in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let delta: f64 = item
            .parse()
            .ok()
            .filter(|d: &f64| d.is_finite() && *d > 0.0)
            .ok_or_else(|| Error::new("err.flow.companions.list").with("item", item))?;
        let name = names
            .iter()
            .find(|(n, _)| (*n as f64 - delta).abs() < 1e-9)
            .map_or_else(|| format!("{prefix}{item}"), |(_, s)| (*s).to_string());
        out.push(Rel { delta, name });
    }
    Ok(out)
}

/// The names of every relation of the lists that the difference `d` fits within `da_tol`: all of them, never one forced.
pub fn fitting(p: &Params, d: f64) -> Vec<String> {
    p.adducts
        .iter()
        .chain(&p.losses)
        .filter(|r| (d - r.delta).abs() <= p.da_tol)
        .map(|r| r.name.clone())
        .collect()
}

/// Whether the relations hold an adduct (the lighter ion is the parent) and a loss (the heavier ion is the parent).
pub fn sides(relations: &[String]) -> (bool, bool) {
    let loss = |n: &String| n.starts_with("loss_");
    (
        relations.iter().any(|n| !loss(n)),
        relations.iter().any(loss),
    )
}
