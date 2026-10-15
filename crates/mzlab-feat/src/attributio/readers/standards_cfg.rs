// Ported from Attributio readers/standards.py, annotate.py, campaign.py.
//! Settings of the reference-standard check (`[standards]` of the campaign), the library file and the chemist's Level 1 decisions.

use super::read_table;
use super::standards::*;
use crate::attributio::campaign::{expand, Campaign};
use crate::attributio::error::{py_list, CampaignError, Result};
use crate::attributio::frame::{to_num, Frame};
use crate::attributio::io::Io;
use crate::attributio::toml::{Table, Value};
use std::collections::BTreeSet;

/// `[standards]` of the campaign with its defaults; `None` when the campaign has no reference-standard library.
#[derive(Clone, Debug, PartialEq)]
pub struct StandardsSettings {
    pub mass_tol_ppm: f64,
    pub rt_tol_min: f64,
    pub rt_column: [String; 2],
    pub adducts: [Vec<String>; 2],
    pub ms2_library: [Option<String>; 2],
    pub ms2_min_cosine: f64,
    pub ms2_min_matched: i64,
    pub ms2_tol_da: f64,
    pub precursor_tol_ppm: f64,
    pub decisions: Option<String>,
}

fn number(t: &Table, key: &str, default: f64) -> f64 {
    t.get(key).and_then(Value::as_f64).unwrap_or(default)
}

fn adduct_list(t: &Table, key: &str, default: &[&str]) -> Vec<String> {
    match t.get(key) {
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => default.iter().map(|s| s.to_string()).collect(),
    }
}

impl StandardsSettings {
    pub fn index(pol: &str) -> usize {
        usize::from(pol != "POS")
    }
    pub fn from_campaign(c: &Campaign) -> Option<StandardsSettings> {
        let st = &c.standards;
        if st.0.is_empty() || c.library_csv.is_none() {
            return None;
        }
        let text = |k: &str| {
            st.get(k)
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(|s| expand(s, &c.env))
        };
        let rt = st.get("rt_column").and_then(Value::as_str).unwrap_or("rt");
        let rt_pol = |k: &str| st.get(k).and_then(Value::as_str).unwrap_or(rt).to_string();
        Some(StandardsSettings {
            mass_tol_ppm: number(st, "mass_tol_ppm", 5.0),
            rt_tol_min: number(st, "rt_tol_min", 0.35),
            rt_column: [rt_pol("rt_column_pos"), rt_pol("rt_column_neg")],
            adducts: [
                adduct_list(st, "adducts_pos", &DEFAULT_POS_ADDUCTS),
                adduct_list(st, "adducts_neg", &DEFAULT_NEG_ADDUCTS),
            ],
            ms2_library: [text("ms2_library_pos"), text("ms2_library_neg")],
            ms2_min_cosine: number(st, "ms2_min_cosine", 0.70),
            ms2_min_matched: number(st, "ms2_min_matched", 2.0) as i64,
            ms2_tol_da: number(st, "ms2_tol_da", 0.01),
            precursor_tol_ppm: number(st, "precursor_tol_ppm", 10.0),
            decisions: text("decisions"),
        })
    }
}

/// The reference-standard library (semicolon-separated, one row per standard); an empty frame, with a warning, when the file is not there.
pub fn load_standards_library(c: &Campaign, io: &Io, warnings: &mut Vec<String>) -> Result<Frame> {
    let Some(path) = &c.library_csv else {
        return Ok(Frame::default());
    };
    match read_table(io, path, ';')? {
        Some(f) => Ok(f),
        None => {
            warnings.push(format!(
                "[{}] standards library not found: {path} -> no Level 1 in this run.",
                c.id
            ));
            Ok(Frame::default())
        }
    }
}

/// One decision of the chemist on a mass + RT candidate (`confirm` or `reject`).
#[derive(Clone, Debug, PartialEq)]
pub struct Level1Decision {
    pub pol: String,
    pub feature_id: i64,
    pub standard: String,
    pub decision: String,
    pub decided_on: String,
    pub note: String,
    pub mz: String,
    pub rt: String,
}

/// The chemist's Level 1 decisions (file `pol;feature_id;standard;decision;decided_on;note;mz;rt`); pending rows (empty decision) are dropped.
pub fn load_decisions(text: Option<&str>, path: &str) -> Result<Vec<Level1Decision>> {
    let Some(text) = text else {
        return Ok(Vec::new());
    };
    let d = Frame::parse(text, ';')?;
    for name in ["pol", "feature_id", "decision"] {
        if d.col(name).is_none() {
            return Err(CampaignError::value(format!("KeyError: '{name}'")));
        }
    }
    let get = |r: usize, n: &str| d.col(n).map_or(String::new(), |k| d.cell(r, k).to_string());
    let mut bad = BTreeSet::new();
    let mut out = Vec::new();
    for r in 0..d.len() {
        let decision = get(r, "decision").trim().to_lowercase();
        if !matches!(decision.as_str(), "confirm" | "reject" | "") {
            bad.insert(decision.clone());
        }
        let Some(fid) = to_num(&get(r, "feature_id")) else {
            continue;
        };
        if decision.is_empty() {
            continue;
        }
        out.push(Level1Decision {
            pol: get(r, "pol").trim().to_uppercase(),
            feature_id: fid as i64,
            standard: get(r, "standard"),
            decision,
            decided_on: get(r, "decided_on"),
            note: get(r, "note"),
            mz: get(r, "mz"),
            rt: get(r, "rt"),
        });
    }
    if !bad.is_empty() {
        let found: Vec<&String> = bad.iter().collect();
        return Err(CampaignError::value(format!(
            "{path}: decision must be 'confirm', 'reject' or empty (pending), found {}",
            py_list(&found)
        )));
    }
    Ok(out)
}
