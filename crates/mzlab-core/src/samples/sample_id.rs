//! The sample-ID convention of the lab series (`MAT-COND-Ttt`, `MAT-STD<cc>`, `QC-MAT`, `P-<name>`, blanks), read from a `sample_id`; the parts
//! are split at `-` or `_` alike (`MQ_H2O2_T30` is `MQ-H2O2-T30`), a dark control is `COND` + `D` (`H2O2D`, `H2O2-D`) and `MAT-WASH` is a blank (the name model says it is a wash).
//! Used by `model` to fill the fields the name model did not capture and by `validate3` to warn about an unknown condition or a bad time.

use super::Sample;

pub(super) const MATRICES: [&str; 2] = ["TW", "MQ"];
pub(super) const IRRADIATED: [&str; 6] = ["C", "N2", "O2", "AIR", "H2O2", "PMS"];
/// The conditions fused with the dark mark: the same words as `names::DARK_WORDS` (a test keeps them equal), so `CD` is no dark mark.
pub(super) const DARK: [&str; 6] = ["H2O2D", "PMSD", "BLKD", "N2D", "O2D", "AIRD"];
const BLANK: [&str; 4] = ["BLK", "BLKP", "SB", "WASH"];

/// What an ID says: the role, the matrix, the condition, the time in minutes and the standard concentration.
#[derive(Debug, Default, PartialEq)]
pub struct IdInfo {
    pub kind: Option<&'static str>,
    pub matrix: Option<&'static str>,
    pub cond: Option<String>,
    pub time_min: Option<f64>,
    pub conc: Option<f64>,
    /// The ID has a `T...` time part that is not a number.
    pub bad_time: bool,
    /// The ID has the shape `MAT-COND-Ttt` but COND is not one of the known conditions.
    pub unknown_cond: bool,
    /// The ID ends with `-R`.
    pub reinjection: bool,
}

/// `2P5` (the ID is upper-cased) → 2.5, `30` → 30; None when it is not a number written that way.
#[inline(never)]
fn time_of(t: &str) -> Option<f64> {
    let (whole, frac) = t.split_once('P').unwrap_or((t, ""));
    let digits = |s: &str| s.bytes().all(|c| c.is_ascii_digit());
    if whole.is_empty() || !digits(whole) || !digits(frac) || (t.contains('P') && frac.is_empty()) {
        return None;
    }
    format!("{whole}.{}", if frac.is_empty() { "0" } else { frac })
        .parse()
        .ok()
}

#[inline(never)]
fn matrix_of(s: &str) -> Option<&'static str> {
    MATRICES.iter().copied().find(|m| m.eq_ignore_ascii_case(s))
}

/// Reads an ID; an ID of no known shape gives the default (nothing said). A trailing `-R` or `_R` (re-injection) is ignored.
pub fn read(id: &str) -> IdInfo {
    // `_` reads as `-`; a last part `R` is the re-injection mark
    let up = id.trim().to_uppercase().replace('_', "-");
    let mut parts: Vec<&str> = up.split('-').collect();
    let reinjection = parts.len() > 1 && parts.last() == Some(&"R");
    if reinjection {
        parts.pop();
    }
    let mut r = IdInfo {
        reinjection,
        ..IdInfo::default()
    };
    match parts.as_slice() {
        // `MQ-H2O2-D-T30`: the dark mark written apart from its condition, as `MQ-H2O2D-T30`
        [m, cond, t] | [m, cond, "D", t] if matrix_of(m).is_some() && t.starts_with('T') => {
            r.matrix = matrix_of(m);
            match time_of(&t[1..]) {
                Some(x) => r.time_min = Some(x),
                None => r.bad_time = true,
            }
            // a dark control is a known condition (or the matrix blank) followed by `D`, fused or apart (`N2D`, `BLK-D`); the `D` of an unknown condition is
            // not read: the condition stays as written and is reported unknown
            let apart = parts.len() == 4;
            let fused = DARK.contains(cond);
            let stem = if fused {
                &cond[..cond.len() - 1]
            } else {
                *cond
            };
            let known = IRRADIATED.contains(&stem) || ((apart || fused) && stem == "BLK");
            let dark = known && (apart || fused);
            r.kind = Some(if dark { "dark" } else { "sample" });
            r.unknown_cond = !known;
            let mut cond = cond.to_string();
            if apart && known {
                cond.push('D');
            }
            r.cond = Some(cond);
        }
        [m, s] if matrix_of(m).is_some() && s.starts_with("STD") => {
            r.matrix = matrix_of(m);
            r.kind = Some("std");
            r.conc = s[3..].parse().ok();
        }
        ["QC", m] if matrix_of(m).is_some() => {
            r.matrix = matrix_of(m);
            r.kind = Some("qc");
        }
        ["P", _, ..] => r.kind = Some("pool"),
        [m, b] if matrix_of(m).is_some() && BLANK.contains(b) => {
            r.matrix = matrix_of(m);
            r.kind = Some("blank");
        }
        [b] if BLANK.contains(b) => r.kind = Some("blank"),
        _ => {}
    }
    r
}

/// Fills the fields of `c` from its `sample_id`, except the ones named in `skip` (captured by the model): matrix, condition (`series`), group
/// (`MAT-COND`), time, role and the concentration of a standard (mg/L).
pub fn apply(c: &mut Sample, skip: &[&str]) {
    let i = read(&c.sample_id);
    let free = |f: &str| !skip.contains(&f);
    c.reinjection = i.reinjection;
    if let (true, Some(m)) = (free("matrix"), i.matrix) {
        c.matrix = m.into();
    }
    if let (true, Some(k)) = (free("kind"), i.kind) {
        c.kind = k.into();
    }
    if let (true, Some(cond)) = (free("series"), &i.cond) {
        c.series = cond.clone();
    }
    if let (true, Some(m), Some(cond)) = (free("group"), i.matrix, &i.cond) {
        c.group = format!("{m}-{cond}");
    }
    if free("time")
        && free("time_unit")
        && let Some(t) = i.time_min
    {
        c.set_time(Some(t), "min");
        c.time_confirmed = false;
    }
    if let (true, Some(x)) = (free("concentration"), i.conc) {
        c.conc = Some(x);
        c.unit = Some("mg/L".into());
    }
}

#[cfg(test)]
#[path = "sample_id_tests.rs"]
mod tests;
