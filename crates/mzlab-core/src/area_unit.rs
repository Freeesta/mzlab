//! The unit of the areas the automatic integrations export: counts x s (default, as `peak area` and the manual integration give) or counts x min
//! (the engine's internal unit, the old output). Converted at the export boundary only; the engines keep minutes.

use crate::peakq_lrauto::{lr_auto_json, LrAuto};

/// Seconds per minute: the whole conversion.
pub const SECONDS_PER_MIN: f64 = 60.0;

/// The unit of an exported area (`area`, `below`, `area_zero`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AreaUnit {
    /// counts x s, the unit of every manual and feature area.
    #[default]
    Seconds,
    /// counts x min, the old unit of the automatic routes.
    Minutes,
}

impl AreaUnit {
    /// `s` or `min`; anything else is an error.
    pub fn parse(v: &str) -> Result<AreaUnit, String> {
        match v {
            "s" => Ok(AreaUnit::Seconds),
            "min" => Ok(AreaUnit::Minutes),
            _ => Err("area_unit is s or min".into()),
        }
    }

    /// The wasm interface number: 0 = s (default), 1 = min; a missing slot or any other number keeps the default.
    pub fn from_number(v: Option<f64>) -> AreaUnit {
        if v == Some(1.0) {
            AreaUnit::Minutes
        } else {
            AreaUnit::Seconds
        }
    }

    /// The factor from the engine's counts x min.
    pub fn factor(self) -> f64 {
        match self {
            AreaUnit::Seconds => SECONDS_PER_MIN,
            AreaUnit::Minutes => 1.0,
        }
    }
}

/// `lr_auto_json` with the areas in `unit`; `Minutes` is byte-identical to `lr_auto_json`.
pub fn lr_auto_json_in(rt: &[f64], r: &LrAuto, unit: AreaUnit) -> String {
    let f = unit.factor();
    if f == 1.0 {
        return lr_auto_json(rt, r);
    }
    let mut r = r.clone();
    for k in &mut r.peaks {
        k.area *= f;
        k.below *= f;
        k.area_zero *= f;
    }
    lr_auto_json(rt, &r)
}

#[cfg(test)]
#[path = "area_unit_test.rs"]
mod tests;
