//! The parameters of `union`, checked: a value out of range is `err.union.param`, never clamped; an unknown name is refused too.

use crate::error::{Error, Result};
use crate::json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Warp {
    Off,
    Piecewise,
}

#[derive(Debug, Clone)]
pub struct Params {
    /// m/z agreement of two rows of one target, Da.
    pub da: f64,
    /// RT agreement after the map, min.
    pub rt_tol: f64,
    /// Widest RT gap between two rows that may serve as an anchor of the map, min.
    pub rt_gap: f64,
    pub warp: Warp,
    /// Targets seen in fewer series are listed as dropped, not written.
    pub min_series: usize,
    /// Minutes added on both sides of every window.
    pub pad: f64,
    /// Polarity of the rows that have none of their own: `None` = every scan.
    pub polarity: Option<i8>,
}

fn bad(name: &str, value: &str) -> Error {
    Error::new("err.union.param")
        .with("name", name)
        .with("value", value)
}

fn number(name: &str, v: &Value) -> Result<f64> {
    let (text, n) = match v {
        Value::Num(n) => (n.to_string(), Some(*n)),
        Value::Str(s) => (s.clone(), s.trim().parse::<f64>().ok()),
        _ => (String::new(), None),
    };
    n.filter(|n| n.is_finite()).ok_or_else(|| bad(name, &text))
}

fn within(name: &str, v: &Value, lo: f64, hi: f64) -> Result<f64> {
    let n = number(name, v)?;
    if (lo..=hi).contains(&n) {
        Ok(n)
    } else {
        Err(bad(name, &n.to_string()))
    }
}

impl Params {
    pub fn defaults() -> Params {
        Params {
            da: 0.5,
            rt_tol: 0.1,
            rt_gap: 0.5,
            warp: Warp::Off,
            min_series: 1,
            pad: 0.0,
            polarity: None,
        }
    }

    pub fn from_json(v: Option<&Value>) -> Result<Params> {
        let mut p = Params::defaults();
        for (k, x) in v.map_or(&[][..], Value::entries) {
            match k.as_str() {
                "da" => p.da = within(k, x, 0.001, 5.0)?,
                "rt_tol" => p.rt_tol = within(k, x, 0.001, 5.0)?,
                "rt_gap" => p.rt_gap = within(k, x, 0.001, 5.0)?,
                "pad" => p.pad = within(k, x, 0.0, 5.0)?,
                "min_series" => {
                    let n = within(k, x, 1.0, 1000.0)?;
                    if n.fract() != 0.0 {
                        return Err(bad(k, &n.to_string()));
                    }
                    p.min_series = n as usize;
                }
                "rt_warp" => {
                    p.warp = match x.str() {
                        Some("off") => Warp::Off,
                        Some("piecewise") => Warp::Piecewise,
                        s => return Err(bad(k, s.unwrap_or(""))),
                    }
                }
                "polarity" => {
                    p.polarity = match x.str().map(str::trim) {
                        Some("") | None => None,
                        Some("+") => Some(1),
                        Some("-") => Some(-1),
                        Some(s) => return Err(bad(k, s)),
                    }
                }
                _ => return Err(bad(k, "unknown parameter")),
            }
        }
        Ok(p)
    }

    pub fn warp_name(&self) -> &'static str {
        match self.warp {
            Warp::Off => "off",
            Warp::Piecewise => "piecewise",
        }
    }
}
