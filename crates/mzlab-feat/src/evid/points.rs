//! The table of points of the proofs: the section `punti` of the profile of `livelli/1` (`data/livelli-1.json`, written from `mzlab/web/livelli-1.js`
//! by `tools/esporta_livelli.py`). Stepped points after Alygizakis et al. 2023, listed best step first; a step threshold given as a string is the key
//! of a threshold of `soglie` of the same profile, so the points sit beside the thresholds and cannot drift from them.

use super::Kind;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Value};

pub const FORMAT: &str = "livelli/1";
/// The profile the points belong to: the default one.
const PROFILE_KEY: &str = "predefinito";
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
}

fn bad(what: &str) -> Error {
    Error::new("error.evid.points").with("what", what)
}

fn threshold(v: &Value, soglie: &Value) -> Result<f64> {
    match v {
        Value::Num(n) => Ok(*n),
        Value::Str(k) => soglie
            .get(k)
            .and_then(Value::f64)
            .ok_or_else(|| bad("threshold key")),
        _ => Err(bad("threshold")),
    }
}

fn step_of(s: &Value, soglie: &Value) -> Result<Step> {
    let thr = s.get("soglia").ok_or_else(|| bad("step"))?;
    let points = s.get("punti").and_then(Value::f64);
    Ok(Step {
        threshold: threshold(thr, soglie)?,
        points: points.ok_or_else(|| bad("points"))?,
    })
}

fn require_of(v: &Value, soglie: &Value) -> Result<Option<(bool, f64)>> {
    let Some(r) = v.get("richiede") else {
        return Ok(None);
    };
    let thr = r.get("soglia").ok_or_else(|| bad("require"))?;
    let le = r.get("op").and_then(Value::str) == Some("<=");
    Ok(Some((le, threshold(thr, soglie)?)))
}

fn rule_of(v: &Value, soglie: &Value) -> Result<Rule> {
    let le = match v.get("op").and_then(Value::str) {
        Some("<=") => true,
        Some(">=") => false,
        _ => return Err(bad("op")),
    };
    let steps = v.get("gradini").map(Value::arr).unwrap_or(&[]);
    Ok(Rule {
        le,
        abs: v.get("assoluto").and_then(Value::bool).unwrap_or(false),
        steps: steps
            .iter()
            .map(|s| step_of(s, soglie))
            .collect::<Result<_>>()?,
        require: require_of(v, soglie)?,
        cap: v.get("massimo").and_then(Value::f64),
        cap_if_weighed: v.get("massimo_se_pesata").and_then(Value::f64),
    })
}
impl PointsTable {
    /// Reads `livelli/1` (the whole data file or the JSON of the crate); a file without the section `punti` is refused.
    pub fn parse(bytes: &[u8]) -> Result<PointsTable> {
        let v = json::parse(bytes).map_err(|_| bad("json"))?;
        if v.get("formato").and_then(Value::str) != Some(FORMAT) {
            return Err(bad("format"));
        }
        let name = v
            .get(PROFILE_KEY)
            .and_then(Value::str)
            .ok_or_else(|| bad("profile"))?;
        let profile = v
            .get("profili")
            .and_then(|p| p.get(name))
            .ok_or_else(|| bad("profile"))?;
        let soglie = profile.get("soglie").ok_or_else(|| bad("soglie"))?;
        let punti = profile.get("punti").ok_or_else(|| bad("punti"))?;
        let tol = punti.get("tolleranze").ok_or_else(|| bad("tolleranze"))?;
        let mut rules = Vec::new();
        for kind in Kind::ALL {
            if let Some(r) = punti.get("prove").and_then(|p| p.get(kind.name())) {
                rules.push((kind, rule_of(r, soglie)?));
            }
        }
        Ok(PointsTable {
            version: punti.get("versione").and_then(Value::f64).unwrap_or(0.0) as u32,
            rules,
            mass_ppm: threshold(
                tol.get("massa_ppm").ok_or_else(|| bad("massa_ppm"))?,
                soglie,
            )?,
            ms2_margin: threshold(
                tol.get("margine_ms2").ok_or_else(|| bad("margine_ms2"))?,
                soglie,
            )?,
            weighed: false,
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
