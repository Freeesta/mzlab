//! What the adapter reads: the files of the Dig (as the page passes them to `api.run`), the parent, the table of the series with its `evid/1` graph
//! and the registry `ipotesi/1` of the hypotheses.

use std::collections::HashMap;

use mzlab_core::elements::{fmt, ion_mz, mass, Formula};
use mzlab_core::json::{self, Value};

use super::formula::{neutral_formula, smiles_formula};
use super::sample::guess;
use super::Res;

/// One LC file of the series (a column of the table), in the order of the table.
#[derive(Debug, Clone)]
pub struct FileIn {
    pub name: String,
    pub label: String,
    pub kind: String,
    pub time: Option<f64>,
    /// `-1` for a reference (dark, t0, blank, control), the time in minutes, NaN when unknown.
    pub t: f64,
}

impl FileIn {
    pub fn is_ref(&self) -> bool {
        self.time.is_some_and(|t| t <= 0.0) || self.kind == "blank" || self.kind == "control"
    }
    pub fn is_treated(&self) -> bool {
        !self.is_ref() && self.t > 0.0 && self.kind == "sample"
    }
}

/// The files of `api.run` that make the series: samples, blanks and controls (standards are not columns of the table).
pub fn lc_files(files_json: &str) -> Res<Vec<FileIn>> {
    let v = json::parse(files_json.as_bytes()).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for f in v.arr() {
        let name = f
            .get("name")
            .and_then(Value::str)
            .unwrap_or_default()
            .to_string();
        let kind = f
            .get("type")
            .and_then(Value::str)
            .unwrap_or("sample")
            .to_string();
        if !matches!(kind.as_str(), "sample" | "blank" | "control") {
            continue;
        }
        let time = f.get("time").and_then(Value::f64);
        let mut x = FileIn {
            label: guess(&name).label,
            name,
            kind,
            time,
            t: f64::NAN,
        };
        x.t = match time {
            Some(t) => t,
            None if x.is_ref() => -1.0,
            None => f64::NAN,
        };
        out.push(x);
    }
    Ok(out)
}

/// The parent of `api.run`: name, neutral formula (from the formula or the SMILES), ion [M+H]+.
#[derive(Debug, Clone)]
pub struct Parent {
    pub name: String,
    pub neutral: Formula,
    pub ion: Formula,
    pub mz: f64,
}

pub fn parent(parent_json: &str) -> Res<Parent> {
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
    let mut ion = neutral.clone();
    match ion.iter_mut().find(|e| e.0 == "H") {
        Some(h) => h.1 += 1,
        None => ion.push(("H".into(), 1)),
    }
    let mz = ion_mz(mass(&neutral), "[M+H]+").unwrap_or(f64::NAN);
    let name = text("name").unwrap_or_default().to_string();
    Ok(Parent {
        name,
        neutral,
        ion,
        mz,
    })
}

impl Parent {
    pub fn formula(&self) -> String {
        fmt(&self.neutral)
    }
    pub fn ion_formula(&self) -> String {
        fmt(&self.ion)
    }
}

/// One proof of an identity (`contributi` of `evid/1`).
#[derive(Debug, Clone)]
pub struct Proof {
    pub kind: String,
    pub value: Option<f64>,
    pub threshold: Option<f64>,
    pub points: f64,
    pub sign: String,
}

/// One identity of `evid/1`: its row, hypothesis, formula and proofs.
#[derive(Debug, Clone)]
pub struct Ident {
    pub row: usize,
    pub hypothesis: Option<u32>,
    pub formula: Option<String>,
    pub total: f64,
    pub proofs: Vec<Proof>,
}

pub fn identities(evid: &[u8]) -> Res<Vec<Ident>> {
    if evid.is_empty() {
        return Ok(Vec::new());
    }
    let v = json::parse(evid).map_err(|e| e.to_string())?;
    let num = |x: &Value, k: &str| x.get(k).and_then(Value::f64);
    let text = |x: &Value, k: &str| {
        x.get(k)
            .and_then(Value::str)
            .unwrap_or_default()
            .to_string()
    };
    let ids = v.get("identita").map(Value::arr).unwrap_or_default();
    Ok(ids
        .iter()
        .map(|x| Ident {
            row: num(x, "id").unwrap_or(0.0) as usize,
            hypothesis: num(x, "ipotesi").map(|h| h as u32),
            formula: x.get("formula").and_then(Value::str).map(String::from),
            total: num(x, "totale").unwrap_or(0.0),
            proofs: x
                .get("contributi")
                .map(Value::arr)
                .unwrap_or_default()
                .iter()
                .map(|p| Proof {
                    kind: text(p, "tipo"),
                    value: num(p, "valore"),
                    threshold: num(p, "soglia"),
                    points: num(p, "punti").unwrap_or(0.0),
                    sign: text(p, "segno"),
                })
                .collect(),
        })
        .collect())
}

/// One hypothesis of the registry: how the formula was derived from the parent.
#[derive(Debug, Clone)]
pub struct Hyp {
    pub label: String,
    pub formula: String,
    pub mz: f64,
    pub level: u8,
    pub sources: Vec<String>,
    pub alternatives: Vec<String>,
}

/// The registry `ipotesi/1` by id, and the number of hypotheses tried and found.
pub fn registry(text: &str) -> Res<(HashMap<u32, Hyp>, u32, u32)> {
    if text.trim().is_empty() {
        return Ok((HashMap::new(), 0, 0));
    }
    let v = json::parse(text.as_bytes()).map_err(|e| e.to_string())?;
    let strs = |x: &Value, k: &str| -> Vec<String> {
        x.get(k)
            .map(Value::arr)
            .unwrap_or_default()
            .iter()
            .filter_map(Value::str)
            .map(String::from)
            .collect()
    };
    let mut out = HashMap::new();
    for h in v.get("hypotheses").map(Value::arr).unwrap_or_default() {
        let Some(id) = h.get("id").and_then(Value::f64) else {
            continue;
        };
        out.insert(
            id as u32,
            Hyp {
                label: h
                    .get("label")
                    .and_then(Value::str)
                    .unwrap_or_default()
                    .to_string(),
                formula: h
                    .get("formula")
                    .and_then(Value::str)
                    .unwrap_or_default()
                    .to_string(),
                mz: h.get("mz").and_then(Value::f64).unwrap_or(f64::NAN),
                level: h.get("level").and_then(Value::f64).unwrap_or(0.0) as u8,
                sources: strs(h, "sources"),
                alternatives: strs(h, "alternatives"),
            },
        );
    }
    let count = |k: &str| v.get(k).and_then(Value::f64).unwrap_or(0.0) as u32;
    Ok((out, count("tried"), count("found")))
}
