//! The experimental design, `disegno/1` (`docs/schemi/disegno-1.json`): conditions, reference, comparisons, factors and batches as data of the plan.
//! `proponi` reads a `campioni/2` and proposes; the user confirms. `valida` gives stable codes (the pages put the words). No statistics here.

mod json;

pub use json::{digest, from_json, to_json};

use crate::campioni::{Avviso, Campione};
use crate::error::{Error, Result};

pub const SCHEMA: &str = "disegno/1";
/// The batch given to every file when the user says nothing.
pub const LOTTO_PREDEFINITO: &str = "1";
/// Fewer replicates than this per time point give `fewReplicates`.
pub const MIN_REPLICHE: u32 = 3;

/// One condition: the files of one `serie`, in the order of injection.
#[derive(Debug, Clone, PartialEq)]
pub struct Condizione {
    /// A `serie` of `campioni/2` ("" = the only series).
    pub name: String,
    /// The fewest sample files over the time points of the condition (derived, 0 when it has none).
    pub replicates: u32,
    /// Level of each factor (factor, value), sorted by factor.
    pub levels: Vec<(String, String)>,
}

/// One comparison: two conditions, or every condition against the reference.
#[derive(Debug, Clone, PartialEq)]
pub enum Confronto {
    Pair(String, String),
    VsReference,
}

/// A factor and the values it takes.
#[derive(Debug, Clone, PartialEq)]
pub struct Fattore {
    pub name: String,
    pub values: Vec<String>,
}

/// The batch of one file (the key is the SHA-256 when known, else the name).
#[derive(Debug, Clone, PartialEq)]
pub struct Lotto {
    pub file: String,
    pub sha256: String,
    pub batch: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Disegno {
    pub conditions: Vec<Condizione>,
    pub reference: Option<String>,
    pub comparisons: Vec<Confronto>,
    pub factors: Vec<Fattore>,
    pub batches: Vec<Lotto>,
}

impl Disegno {
    /// Sorts what has no meaningful order, so that equal designs give equal bytes.
    fn normalize(&mut self) {
        for c in &mut self.conditions {
            c.levels.sort();
        }
        self.factors.sort_by(|a, b| a.name.cmp(&b.name));
        self.batches
            .sort_by(|a, b| (&a.sha256, &a.file, &a.batch).cmp(&(&b.sha256, &b.file, &b.batch)));
    }
}

/// The sample files of a condition.
fn samples<'a>(cs: &'a [Campione], name: &'a str) -> impl Iterator<Item = &'a Campione> {
    cs.iter()
        .filter(move |c| c.role() == "sample" && c.series == name)
}

/// The fewest files over the time points of a condition (an unknown time is a point of its own).
fn replicates(cs: &[Campione], name: &str) -> u32 {
    let times: Vec<Option<f64>> = samples(cs, name).map(|c| c.time_min).collect();
    times
        .iter()
        .map(|t| times.iter().filter(|u| *u == t).count() as u32)
        .min()
        .unwrap_or(0)
}

/// The condition that is likely the reference: the first called ref…, reference, control or no gas.
fn guess_reference(names: &[String]) -> Option<String> {
    names
        .iter()
        .find(|n| {
            let l = n.to_lowercase();
            l.starts_with("ref")
                || l.contains("reference")
                || l.contains("control")
                || l.replace(['_', '-', ' '], "").contains("nogas")
        })
        .cloned()
}

/// The design the names suggest: conditions from `serie` in order of injection, the likely reference with "all against the reference", one batch.
pub fn proponi(cs: &[Campione]) -> Disegno {
    let mut ordered: Vec<&Campione> = samples_all(cs);
    ordered.sort_by(|a, b| (a.order, &a.file).cmp(&(b.order, &b.file)));
    let mut names: Vec<String> = Vec::new();
    for c in ordered.iter().filter(|c| !c.series.is_empty()) {
        if !names.contains(&c.series) {
            names.push(c.series.clone());
        }
    }
    if names.is_empty() && !ordered.is_empty() {
        names.push(String::new());
    }
    let reference = guess_reference(&names);
    let comparisons = if reference.is_some() && names.len() > 1 {
        vec![Confronto::VsReference]
    } else {
        Vec::new()
    };
    let mut d = Disegno {
        conditions: names
            .iter()
            .map(|n| Condizione {
                name: n.clone(),
                replicates: replicates(cs, n),
                levels: Vec::new(),
            })
            .collect(),
        reference,
        comparisons,
        factors: Vec::new(),
        batches: cs
            .iter()
            .map(|c| Lotto {
                file: c.file.clone(),
                sha256: c.sha256.clone(),
                batch: LOTTO_PREDEFINITO.into(),
            })
            .collect(),
    };
    d.normalize();
    d
}

fn samples_all(cs: &[Campione]) -> Vec<&Campione> {
    cs.iter().filter(|c| c.role() == "sample").collect()
}

fn push(out: &mut Vec<Avviso>, code: &'static str, severity: &'static str, name: &str) {
    let a = Avviso {
        code,
        severity,
        file: None,
        series: name.to_string(),
    };
    if !out.contains(&a) {
        out.push(a);
    }
}

/// The batch of a file: the one the design says (by SHA-256 when both are known, else by name), else the default.
fn batch_of<'a>(d: &'a Disegno, c: &Campione) -> &'a str {
    d.batches
        .iter()
        .find(|l| {
            if !l.sha256.is_empty() && !c.sha256.is_empty() {
                l.sha256 == c.sha256
            } else {
                l.file == c.file
            }
        })
        .map_or(LOTTO_PREDEFINITO, |l| l.batch.as_str())
}

/// A QC before the first sample and another after the last one, in the order of injection.
fn qc_at_both_ends(files: &[&Campione]) -> bool {
    let is_sample = |c: &&Campione| matches!(c.role(), "sample" | "t0");
    let (Some(first), Some(last)) = (
        files.iter().position(is_sample),
        files.iter().rposition(is_sample),
    ) else {
        return true;
    };
    files[..first].iter().any(|c| c.kind == "qc")
        && files[last + 1..].iter().any(|c| c.kind == "qc")
}

/// What the design lacks, as codes: `fewReplicates` (warning, per condition), `batchNoQc` (warning, per batch), `emptyComparison` (error, per
/// condition without sample files used by a comparison) and `refMissing` (error, "all against the reference" without a known reference).
pub fn valida(d: &Disegno, cs: &[Campione]) -> Vec<Avviso> {
    let mut out = Vec::new();
    for c in &d.conditions {
        let n = replicates(cs, &c.name);
        if n > 0 && n < MIN_REPLICHE {
            push(&mut out, "fewReplicates", "warning", &c.name);
        }
    }
    let mut names: Vec<&str> = Vec::new();
    for c in cs {
        let b = batch_of(d, c);
        if !names.contains(&b) {
            names.push(b);
        }
    }
    for b in names {
        let mut files: Vec<&Campione> = cs.iter().filter(|c| batch_of(d, c) == b).collect();
        files.sort_by(|x, y| (x.order, &x.file).cmp(&(y.order, &y.file)));
        if !qc_at_both_ends(&files) {
            push(&mut out, "batchNoQc", "warning", b);
        }
    }
    let empty = |name: &str| samples(cs, name).next().is_none();
    for cmp in &d.comparisons {
        match cmp {
            Confronto::Pair(a, b) => {
                for n in [a, b] {
                    if empty(n) {
                        push(&mut out, "emptyComparison", "error", n);
                    }
                }
            }
            Confronto::VsReference => match &d.reference {
                None => push(&mut out, "refMissing", "error", ""),
                Some(r) => {
                    for n in std::iter::once(r).chain(d.conditions.iter().map(|c| &c.name)) {
                        if empty(n) {
                            push(&mut out, "emptyComparison", "error", n);
                        }
                    }
                }
            },
        }
    }
    out
}

/// The bindings: `op` is `proponi` (a `campioni/2`), `valida` (`{"disegno","campioni"}`), `canonico` or `digest` (a `disegno/1`); JSON in, JSON out.
pub fn route(op: &str, input: &str) -> Result<String> {
    match op {
        "proponi" => Ok(to_json(&proponi(&crate::campioni::from_json(
            input.as_bytes(),
        )?))),
        "valida" => {
            let v = crate::json::parse(input.as_bytes())?;
            let bad = || Error::new("error.badDisegno");
            let d = json::read(v.get("disegno").ok_or_else(bad)?)?;
            let cs = crate::campioni::from_value(v.get("campioni").ok_or_else(bad)?)?;
            Ok(crate::campioni::valida_json(&valida(&d, &cs)))
        }
        "canonico" => Ok(to_json(&from_json(input.as_bytes())?)),
        "digest" => Ok(digest(&from_json(input.as_bytes())?)),
        _ => Err(Error::new("error.badDisegno").with("op", op)),
    }
}
