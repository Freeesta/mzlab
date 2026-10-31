//! The sample-ID convention of the lab series (`MAT-COND-Ttt`, `MAT-STD<cc>`, `QC-MAT`, `P-<name>`, blanks), read from a `campione_id`.
//! Used by `modello` to fill the fields the name model did not capture and by `valida3` to warn about an unknown condition or a bad time.

use super::Campione;

pub(super) const MATRICES: [&str; 2] = ["TW", "MQ"];
pub(super) const IRRADIATED: [&str; 6] = ["C", "N2", "O2", "AIR", "H2O2", "PMS"];
pub(super) const DARK: [&str; 3] = ["H2O2D", "PMSD", "BLKD"];
const BLANK: [&str; 3] = ["BLK", "BLKP", "SB"];

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
fn time_of(t: &str) -> Option<f64> {
    let (whole, frac) = t.split_once('P').unwrap_or((t, ""));
    let digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
    if whole.is_empty() || !digits(whole) || !digits(frac) || (t.contains('P') && frac.is_empty()) {
        return None;
    }
    format!("{whole}.{}", if frac.is_empty() { "0" } else { frac })
        .parse()
        .ok()
}

fn matrix_of(s: &str) -> Option<&'static str> {
    MATRICES.iter().copied().find(|m| m.eq_ignore_ascii_case(s))
}

/// Reads an ID; an ID of no known shape gives the default (nothing said). A trailing `-R` (re-injection) is ignored.
pub fn leggi(id: &str) -> IdInfo {
    let id = id.trim();
    let base = id
        .strip_suffix("-R")
        .or_else(|| id.strip_suffix("-r"))
        .unwrap_or(id);
    let up = base.to_uppercase();
    let reinjection = base.len() != id.len();
    let parts: Vec<&str> = up.split('-').collect();
    let mut r = IdInfo {
        reinjection,
        ..IdInfo::default()
    };
    match parts.as_slice() {
        [m, cond, t] if matrix_of(m).is_some() && t.starts_with('T') => {
            r.matrix = matrix_of(m);
            r.cond = Some(cond.to_string());
            match time_of(&t[1..]) {
                Some(x) => r.time_min = Some(x),
                None => r.bad_time = true,
            }
            r.kind = Some(if DARK.contains(cond) {
                "dark"
            } else {
                "sample"
            });
            r.unknown_cond = !IRRADIATED.contains(cond) && !DARK.contains(cond);
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
        ["P", _] => r.kind = Some("pool"),
        [m, b] if matrix_of(m).is_some() && BLANK.contains(b) => {
            r.matrix = matrix_of(m);
            r.kind = Some("blank");
        }
        [b] if BLANK.contains(b) || *b == "MQ_WASH" => r.kind = Some("blank"),
        _ => {}
    }
    r
}

/// Fills the fields of `c` from its `campione_id`, except the ones named in `skip` (captured by the model): matrix, condition (`serie`), group
/// (`MAT-COND`), time, role and the concentration of a standard (mg/L).
pub fn applica(c: &mut Campione, skip: &[&str]) {
    let i = leggi(&c.sample_id);
    let free = |f: &str| !skip.contains(&f);
    c.reinjection = i.reinjection;
    if let (true, Some(m)) = (free("matrice"), i.matrix) {
        c.matrix = m.into();
    }
    if let (true, Some(k)) = (free("tipo"), i.kind) {
        c.kind = k.into();
    }
    if let (true, Some(cond)) = (free("serie"), &i.cond) {
        c.series = cond.clone();
    }
    if let (true, Some(m), Some(cond)) = (free("gruppo"), i.matrix, &i.cond) {
        c.group = format!("{m}-{cond}");
    }
    if free("tempo") && free("unita_tempo") {
        if let Some(t) = i.time_min {
            c.set_time(Some(t), "min");
            c.time_confirmed = false;
        }
    }
    if let (true, Some(x)) = (free("concentrazione"), i.conc) {
        c.conc = Some(x);
        c.unit = Some("mg/L".into());
    }
}
