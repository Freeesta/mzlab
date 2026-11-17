//! The table of points of the proofs: the section `points` of the profile of `levels/1` (`data/livelli-1.json`, written from `mzlab/web/livelli-1.js`
//! by `tools/export_levels.py`). Stepped points after Alygizakis et al. 2023, listed best step first; a step threshold given as a string is the key
//! of a threshold of `thresholds` of the same profile, so the points sit beside the thresholds and cannot drift from them.

use super::Kind;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Value};

pub const FORMAT: &str = "levels/1";
/// The profile the points belong to: the default one.
const PROFILE_KEY: &str = "default";
/// The table committed in the crate; `tests/evid.rs` checks that it is what the data file of the web page gives.
pub const BUNDLED: &str = include_str!("../../data/livelli-1.json");

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    pub threshold: f64,
    pub points: f64,
}

/// Points of one type of proof. `le`: the value must be at most the threshold (else at least). `require`: a second measurement (the shared peaks of an MS2)
/// that must pass its own threshold for any point.
#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub le: bool,
    pub abs: bool,
    pub steps: Vec<Step>,
    pub require: Option<(bool, f64)>,
    /// Ceiling of the points of the type (the kinetics: informative only while 0).
    pub cap: Option<f64>,
    pub cap_if_weighed: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grade {
    pub points: f64,
    /// The step reached; when none is, the lowest step (what is missing to earn points). `None`: the type has no steps.
    pub threshold: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct PointsTable {
    pub version: u32,
    pub rules: Vec<(Kind, Rule)>,
    /// Tolerance on the mass (ppm) and margin of the MS2 score under which two candidates are not told apart.
    pub mass_ppm: f64,
    pub ms2_margin: f64,
    /// Kinetics counted with the ceiling `cap_if_weighed` instead of `cap` (decision K1: off).
    pub weighed: bool,
    /// Unit resolution: windows of m/z in Da (`window`), no proof of mass (a nominal mass is not evidence) and no level above 3 with an MS2.
    pub unit: bool,
}

fn bad(what: &str) -> Error {
    Error::new("error.evid.points").with("what", what)
}

fn threshold(v: &Value, thresholds: &Value) -> Result<f64> {
    match v {
        Value::Num(n) => Ok(*n),
        Value::Str(k) => thresholds
            .get(k)
            .and_then(Value::f64)
            .ok_or_else(|| bad("threshold key")),
        _ => Err(bad("threshold")),
    }
}

fn step_of(s: &Value, thresholds: &Value) -> Result<Step> {
    let thr = s.get("threshold").ok_or_else(|| bad("step"))?;
    let points = s.get("points").and_then(Value::f64);
    Ok(Step {
        threshold: threshold(thr, thresholds)?,
        points: points.ok_or_else(|| bad("points"))?,
    })
}

fn require_of(v: &Value, thresholds: &Value) -> Result<Option<(bool, f64)>> {
    let Some(r) = v.get("requires") else {
        return Ok(None);
    };
    let thr = r.get("threshold").ok_or_else(|| bad("require"))?;
    let le = r.get("op").and_then(Value::str) == Some("<=");
    Ok(Some((le, threshold(thr, thresholds)?)))
}

fn rule_of(v: &Value, thresholds: &Value) -> Result<Rule> {
    let le = match v.get("op").and_then(Value::str) {
        Some("<=") => true,
        Some(">=") => false,
        _ => return Err(bad("op")),
    };
    let steps = v.get("steps").map(Value::arr).unwrap_or(&[]);
    Ok(Rule {
        le,
        abs: v.get("absolute").and_then(Value::bool).unwrap_or(false),
        steps: steps
            .iter()
            .map(|s| step_of(s, thresholds))
            .collect::<Result<_>>()?,
        require: require_of(v, thresholds)?,
        cap: v.get("maximum").and_then(Value::f64),
        cap_if_weighed: v.get("maximum_if_weighted").and_then(Value::f64),
    })
}
impl PointsTable {
    /// Reads `levels/1` (the whole data file or the JSON of the crate); a file without the section `points` is refused.
    pub fn parse(bytes: &[u8]) -> Result<PointsTable> {
        let v = json::parse(bytes).map_err(|_| bad("json"))?;
        if v.get("format").and_then(Value::str) != Some(FORMAT) {
            return Err(bad("format"));
        }
        let name = v
            .get(PROFILE_KEY)
            .and_then(Value::str)
            .ok_or_else(|| bad("profile"))?;
        let profile = v
            .get("profiles")
            .and_then(|p| p.get(name))
            .ok_or_else(|| bad("profile"))?;
        let thresholds = profile.get("thresholds").ok_or_else(|| bad("thresholds"))?;
        let points = profile.get("points").ok_or_else(|| bad("points"))?;
        let tol = points.get("tolerances").ok_or_else(|| bad("tolerances"))?;
        let mut rules = Vec::new();
        for kind in Kind::ALL {
            if let Some(r) = points.get("proofs").and_then(|p| p.get(kind.name())) {
                rules.push((kind, rule_of(r, thresholds)?));
            }
        }
        Ok(PointsTable {
            version: points.get("version").and_then(Value::f64).unwrap_or(0.0) as u32,
            rules,
            mass_ppm: threshold(
                tol.get("mass_ppm").ok_or_else(|| bad("mass_ppm"))?,
                thresholds,
            )?,
            ms2_margin: threshold(
                tol.get("ms2_margin").ok_or_else(|| bad("ms2_margin"))?,
                thresholds,
            )?,
            weighed: false,
            unit: false,
        })
    }

    /// The table of the crate.
    pub fn bundled() -> Result<PointsTable> {
        Self::parse(BUNDLED.as_bytes())
    }

    pub fn rule(&self, kind: Kind) -> Option<&Rule> {
        self.rules.iter().find(|(k, _)| *k == kind).map(|(_, r)| r)
    }
}

/// The lowest step of a rule: what is missing to earn points.
fn lowest(rule: &Rule) -> Option<f64> {
    rule.steps.last().map(|s| s.threshold)
}

impl PointsTable {
    /// The table for a unit-resolution series: tolerances in Da.
    pub fn with_unit(mut self) -> PointsTable {
        self.unit = true;
        self.mass_ppm = crate::axis::da(crate::axis::LR_TOL);
        self
    }

    /// A window of m/z of `ppm` for a high-resolution series, `LR_TOL` Da for a unit one (the sign convention of `axis::half`).
    pub fn window(&self, ppm: f64) -> f64 {
        if self.unit {
            crate::axis::da(crate::axis::LR_TOL)
        } else {
            ppm
        }
    }

    /// Points of a measured `value` (NaN: not measured, no points); `support` is the second measurement of the types that need one.
    pub fn grade(&self, kind: Kind, value: f64, support: Option<f64>) -> Grade {
        let Some(rule) = self.rule(kind) else {
            return Grade {
                points: 0.0,
                threshold: None,
            };
        };
        let lowest = lowest(rule);
        let v = if rule.abs { value.abs() } else { value };
        let supported = rule
            .require
            .is_none_or(|(le, thr)| support.is_some_and(|s| if le { s <= thr } else { s >= thr }));
        let hit = (supported && !v.is_nan())
            .then(|| {
                rule.steps.iter().find(|s| {
                    if rule.le {
                        v <= s.threshold
                    } else {
                        v >= s.threshold
                    }
                })
            })
            .flatten();
        let cap = if self.weighed {
            rule.cap_if_weighed.or(rule.cap)
        } else {
            rule.cap
        };
        let points = hit
            .map_or(0.0, |s| s.points)
            .min(cap.unwrap_or(f64::INFINITY));
        Grade {
            points,
            threshold: hit.map(|s| s.threshold).or(lowest),
        }
    }
}
