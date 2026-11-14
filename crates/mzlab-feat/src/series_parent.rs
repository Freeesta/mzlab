//! The parent of the series as `api.run` passes it (name, neutral formula from the formula or the SMILES, ion of the adduct, time), and the ion of
//! an adduct from a neutral formula. Read by the adapter of `legacy`, the interpretation, the network of the products and `run_tp` of `mzlab-ops`.

use mzlab_core::elements::{fmt, ion_mz, mass, parse_formula, Formula};
use mzlab_core::json::{self, Value};

use crate::parent_formula::{neutral_formula, smiles_formula};

/// The parent of `api.run`: name, neutral formula (from the formula or the SMILES), ion of the adduct chosen ([M+H]+ when none).
#[derive(Debug, Clone)]
pub struct Parent {
    pub name: String,
    pub neutral: Formula,
    pub ion: Formula,
    pub mz: f64,
    /// The adduct of `ion` and `mz`, one of `elements::adducts`.
    pub adduct: String,
    /// The retention time of the parent given by the user (min), None to let the program choose it.
    pub rt: Option<f64>,
    /// The SMILES of the parent (`smiles`, or `neutral` when it is not a formula), for the fragmentation tree.
    pub smiles: Option<String>,
}

pub fn parent(parent_json: &str) -> Result<Parent, String> {
    let v = json::parse(parent_json.as_bytes()).map_err(|e| e.to_string())?;
    let text = |k: &str| {
        v.get(k)
            .and_then(Value::str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
    };
    let neutral = match (text("smiles"), text("neutral")) {
        (Some(s), _) => smiles_formula(s)?,
        (None, Some(n)) => neutral_formula(n)?,
        _ => {
            return Err(
                "the neutral molecular formula or the SMILES of the parent is required".into(),
            )
        }
    };
    let adduct = text("adduct").unwrap_or("[M+H]+").to_string();
    let ion = adduct_ion(&neutral, &adduct)?;
    let mz = ion_mz(mass(&neutral), &adduct).ok_or_else(|| format!("unknown adduct {adduct}"))?;
    let name = text("name").unwrap_or_default().to_string();
    let is_formula = |t: &str| {
        t.chars().next().is_some_and(|c| c.is_ascii_uppercase())
            && t.chars().all(|c| c.is_ascii_alphanumeric())
    };
    let smiles = text("smiles")
        .or(text("neutral").filter(|t| !is_formula(t)))
        .map(String::from);
    Ok(Parent {
        name,
        neutral,
        ion,
        mz,
        adduct,
        rt: v
            .get("rt")
            .and_then(Value::f64)
            .filter(|r| r.is_finite() && *r >= 0.0),
        smiles,
    })
}

/// The formula and calculated m/z of the ion of `adduct` from a neutral formula written as text; `None` when either cannot be read.
pub fn ion_of(neutral: &str, adduct: &str) -> Option<(String, f64)> {
    let n = parse_formula(neutral).ok().filter(|f| !f.is_empty())?;
    let ion = adduct_ion(&n, adduct).ok()?;
    Some((fmt(&ion), ion_mz(mass(&n), adduct)?))
}

/// Formula of the ion of `adduct` (`[M+H]+`, `[M+NH4]+`, `[M-H]-`, `[M+HCOO]-`...): the groups after `+` are added, the ones after `-` taken away.
fn adduct_ion(neutral: &Formula, adduct: &str) -> Result<Formula, String> {
    let bad = || format!("unknown adduct {adduct}");
    let body = adduct
        .strip_prefix("[M")
        .and_then(|a| a.rsplit_once(']'))
        .map(|a| a.0)
        .ok_or_else(bad)?;
    let mut ion = neutral.clone();
    let mut sign = 1i64;
    let mut group = String::new();
    for ch in body.chars().chain(std::iter::once('+')) {
        if ch != '+' && ch != '-' {
            group.push(ch);
            continue;
        }
        if !group.is_empty() {
            for (el, n) in parse_formula(&group).map_err(|_| bad())? {
                let k = ion.iter().position(|e| e.0 == el);
                let cur = k.map_or(0, |k| ion[k].1) as i64 + sign * n as i64;
                if cur < 0 {
                    return Err(bad());
                }
                match k {
                    Some(k) => ion[k].1 = cur as u64,
                    None => ion.push((el, cur as u64)),
                }
            }
            group.clear();
        }
        sign = if ch == '-' { -1 } else { 1 };
    }
    Ok(ion)
}

impl Parent {
    pub fn formula(&self) -> String {
        fmt(&self.neutral)
    }
    pub fn ion_formula(&self) -> String {
        fmt(&self.ion)
    }
}
