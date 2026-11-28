//! The parameters of `pair`, checked: a value out of range or a bad adduct entry is `err.pair.param`, never clamped; an unknown name is refused too.
//! An adduct entry is a name of the adduct table (`adduct_table::ion_terms`) or `LABEL:SHIFT`, the mass the adduct adds to M for a singly charged ion.

use mzlab_core::adduct_table::ion_terms;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Ppm,
    Da,
}

/// One way an ion is made from the neutral molecule: m/z = (k * M + shift) / z.
#[derive(Debug, Clone)]
pub struct Adduct {
    pub name: String,
    pub k: f64,
    pub shift: f64,
    pub z: f64,
}

impl Adduct {
    /// The neutral mass M that this adduct gives for an ion of this m/z.
    pub fn neutral(&self, mz: f64) -> f64 {
        (mz * self.z - self.shift) / self.k
    }
}

#[derive(Debug, Clone)]
pub struct Params {
    /// Largest RT difference of a pair, min.
    pub rt_tol: f64,
    /// Largest difference of the two neutral masses, in `unit`.
    pub mass_tol: f64,
    pub unit: Unit,
    pub pos: Vec<Adduct>,
    pub neg: Vec<Adduct>,
}

pub const POS_DEFAULT: &str = "[M+H]+";
pub const NEG_DEFAULT: &str = "[M-H]-";

fn bad(name: &str, value: &str) -> Error {
    Error::new("err.pair.param")
        .with("name", name)
        .with("value", value)
}

fn number(name: &str, v: &Value, lo: f64, hi: f64) -> Result<f64> {
    let (text, n) = match v {
        Value::Num(n) => (n.to_string(), Some(*n)),
        Value::Str(s) => (s.clone(), s.trim().parse::<f64>().ok()),
        _ => (String::new(), None),
    };
    n.filter(|n| n.is_finite() && (lo..=hi).contains(n))
        .ok_or_else(|| bad(name, &text))
}

/// The adducts of one side: `sign` is +1 for the positive table, -1 for the negative one; an entry of the other charge sign is refused.
fn adducts(name: &str, list: &str, sign: i8) -> Result<Vec<Adduct>> {
    let mut out: Vec<Adduct> = Vec::new();
    for e in list.split(',').map(str::trim).filter(|e| !e.is_empty()) {
        let a = if let Some((k, shift, z)) = ion_terms(e) {
            if z.signum() != sign {
                return Err(bad(name, e));
            }
            Adduct {
                name: e.to_string(),
                k: f64::from(k),
                shift,
                z: f64::from(z.abs()),
            }
        } else {
            let (label, shift) = e.rsplit_once(':').ok_or_else(|| bad(name, e))?;
            let shift = shift.trim().parse::<f64>().ok().filter(|s| s.is_finite());
            match (label.trim(), shift) {
                (l, Some(s)) if !l.is_empty() => Adduct {
                    name: l.to_string(),
                    k: 1.0,
                    shift: s,
                    z: 1.0,
                },
                _ => return Err(bad(name, e)),
            }
        };
        if out.iter().all(|o| o.name != a.name) {
            out.push(a);
        }
    }
    if out.is_empty() {
        return Err(bad(name, list));
    }
    Ok(out)
}

impl Params {
    pub fn from_json(v: Option<&Value>) -> Result<Params> {
        let (mut rt_tol, mut mass_tol, mut unit) = (0.1, 5.0, Unit::Ppm);
        let (mut pos, mut neg) = (POS_DEFAULT.to_string(), NEG_DEFAULT.to_string());
        for (k, x) in v.map_or(&[][..], Value::entries) {
            match k.as_str() {
                "rt_tol" => rt_tol = number(k, x, 0.0, 60.0)?,
                "mass_tol" => mass_tol = number(k, x, 0.0, 1000.0)?,
                "mass_unit" => {
                    unit = match x.str() {
                        Some("ppm") => Unit::Ppm,
                        Some("da") => Unit::Da,
                        s => return Err(bad(k, s.unwrap_or(""))),
                    }
                }
                "pos_adducts" | "neg_adducts" => {
                    let s = x.str().ok_or_else(|| bad(k, ""))?.to_string();
                    if k == "pos_adducts" {
                        pos = s
                    } else {
                        neg = s
                    }
                }
                _ => return Err(bad(k, "unknown parameter")),
            }
        }
        Ok(Params {
            rt_tol,
            mass_tol,
            unit,
            pos: adducts("pos_adducts", &pos, 1)?,
            neg: adducts("neg_adducts", &neg, -1)?,
        })
    }

    pub fn unit_name(&self) -> &'static str {
        match self.unit {
            Unit::Ppm => "ppm",
            Unit::Da => "da",
        }
    }
}
