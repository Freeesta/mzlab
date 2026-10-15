// Ported from Attributio readers/sirius.py, campaign.py.
//! SIRIUS write-summaries: the three tables the annotation uses (`canopus_structure_summary.tsv` compound classes, `structure_identifications.tsv`
//! CSI:FingerID top structure with the two confidence scores, `spectral_matches.tsv` SIRIUS's own library match). One project space per polarity; column
//! names are those of SIRIUS 6 (no `SIRIUS_` prefix).

use super::{desc_na_last, num_at, read_table, text_at};
use crate::attributio::campaign::{Campaign, POLARITIES};
use crate::attributio::error::{CampaignError, Result};
use crate::attributio::frame::{Cell, Frame, Record};
use crate::attributio::io::{join, Io};
use std::collections::HashSet;

pub const KEY_FILE: &str = "canopus_structure_summary.tsv";

/// The folder with the SIRIUS write-summaries of a polarity: the one the campaign names, else `output <POL>`, else the first folder whose name holds the
/// polarity.
pub fn sirius_output_dir(c: &Campaign, io: &Io, pol: &str) -> Option<String> {
    let has_key = |d: &str| io.exists(&join(d, KEY_FILE));
    if let Some(explicit) = c.explicit_sirius_dir(pol).filter(|s| !s.is_empty()) {
        let d = join(&c.sirius_dir, explicit);
        return has_key(&d).then_some(d);
    }
    let cand = join(&c.sirius_dir, &format!("output {pol}"));
    if has_key(&cand) {
        return Some(cand);
    }
    let mut subs: Vec<String> = io
        .list(&c.sirius_dir)
        .into_iter()
        .filter(|e| e.is_dir && e.name.contains(pol) && !e.name.starts_with('.'))
        .map(|e| e.name)
        .collect();
    subs.sort();
    subs.into_iter()
        .map(|n| join(&c.sirius_dir, &n))
        .find(|d| has_key(d))
}

/// A SIRIUS summary table (name without `.tsv`) of each polarity that has it, with the feature id of each row (rows without one are dropped).
pub struct SiriusTable {
    pub pol: String,
    pub frame: Frame,
    pub ids: Vec<Option<i64>>,
}

/// `load_sirius`: the table of one polarity (`pol`) or of both; `missing_ok` false makes a missing table an error.
pub fn load_sirius(
    c: &Campaign,
    io: &Io,
    table: &str,
    pol: Option<&str>,
    missing_ok: bool,
) -> Result<Vec<SiriusTable>> {
    let pols: Vec<&str> = pol.map_or_else(|| POLARITIES.to_vec(), |p| vec![p]);
    let mut out = Vec::new();
    for p in pols {
        let Some(d) = sirius_output_dir(c, io, p) else {
            continue;
        };
        let Some(frame) = read_table(io, &join(&d, &format!("{table}.tsv")), '\t')? else {
            continue;
        };
        let idcol = if frame.col("mappingFeatureId").is_some() {
            "mappingFeatureId"
        } else {
            "SIRIUS_mappingFeatureId"
        };
        if frame.col(idcol).is_none() {
            return Err(CampaignError::value(format!("KeyError: '{idcol}'")));
        }
        let ids = (0..frame.len())
            .map(|r| num_at(&frame, r, idcol).map(|v| v as i64))
            .collect();
        out.push(SiriusTable {
            pol: p.to_string(),
            frame,
            ids,
        });
    }
    if out.is_empty() && !missing_ok {
        let dirs: Vec<String> = POLARITIES
            .iter()
            .map(|p| match sirius_output_dir(c, io, p) {
                Some(d) => format!("'{p}': PosixPath('{d}')"),
                None => format!("'{p}': None"),
            })
            .collect();
        return Err(CampaignError::not_found(format!(
            "SIRIUS summary '{table}.tsv' not found ({{{}}})",
            dirs.join(", ")
        )));
    }
    Ok(out)
}

/// Compound classes (NPClassifier, ClassyFire) with the probability of each; annotation shows a class only above a threshold.
#[derive(Clone, Debug, PartialEq)]
pub struct Canopus {
    pub pol: String,
    pub id: i64,
    pub classes: [Option<String>; 5],
    pub probs: [Option<f64>; 5],
}

const CANOPUS_CLASSES: [&str; 5] = [
    "NPC#pathway",
    "NPC#superclass",
    "NPC#class",
    "ClassyFire#superclass",
    "ClassyFire#class",
];
const CANOPUS_PROBS: [&str; 5] = [
    "NPC#pathway Probability",
    "NPC#superclass Probability",
    "NPC#class Probability",
    "ClassyFire#superclass probability",
    "ClassyFire#class Probability",
];

impl Record for Canopus {
    const COLUMNS: &'static [&'static str] = &[
        "_pol",
        "_id",
        "npc_pathway",
        "npc_superclass",
        "npc_class",
        "cf_superclass",
        "cf_class",
        "npc_pathway_prob",
        "npc_superclass_prob",
        "npc_class_prob",
        "cf_superclass_prob",
        "cf_class_prob",
    ];
    fn cells(&self) -> Vec<Cell> {
        let mut v = vec![Cell::Str(self.pol.clone()), Cell::Int(self.id)];
        v.extend(self.classes.iter().map(Cell::str));
        v.extend(self.probs.iter().map(|p| Cell::num(*p)));
        v
    }
}

/// CSI:FingerID top structure with the two confidence scores (Exact, Approximate = COSMIC).
#[derive(Clone, Debug, PartialEq)]
pub struct Structure {
    pub pol: String,
    pub id: i64,
    pub name: Option<String>,
    pub formula: Option<String>,
    pub xlogp: Option<f64>,
    pub conf: Option<f64>,
    pub cosmic: Option<f64>,
    pub inchikey2d: Option<String>,
    pub smiles: Option<String>,
}

impl Record for Structure {
    const COLUMNS: &'static [&'static str] = &[
        "_pol",
        "_id",
        "name",
        "formula",
        "xlogp",
        "conf",
        "cosmic",
        "sirius_inchikey2d",
        "sirius_smiles",
    ];
    fn cells(&self) -> Vec<Cell> {
        vec![
            Cell::Str(self.pol.clone()),
            Cell::Int(self.id),
            Cell::str(&self.name),
            Cell::str(&self.formula),
            Cell::num(self.xlogp),
            Cell::num(self.conf),
            Cell::num(self.cosmic),
            Cell::str(&self.inchikey2d),
            Cell::str(&self.smiles),
        ]
    }
}

/// SIRIUS's own spectral-library match (the best by similarity per feature).
#[derive(Clone, Debug, PartialEq)]
pub struct SpectralMatch {
    pub pol: String,
    pub id: i64,
    pub name: Option<String>,
    pub sim: Option<f64>,
    pub peaks: Option<f64>,
    pub inchikey2d: Option<String>,
}

impl Record for SpectralMatch {
    const COLUMNS: &'static [&'static str] = &[
        "_pol",
        "_id",
        "spec_name",
        "spec_sim",
        "spec_peaks",
        "spec_inchikey2d",
    ];
    fn cells(&self) -> Vec<Cell> {
        vec![
            Cell::Str(self.pol.clone()),
            Cell::Int(self.id),
            Cell::str(&self.name),
            Cell::num(self.sim),
            Cell::num(self.peaks),
            Cell::str(&self.inchikey2d),
        ]
    }
}

/// The three tables, each one row per `(pol, id)` where Attributio makes it so (`canopus` and `structure` keep the rows of the export).
#[derive(Clone, Debug, Default)]
pub struct SiriusTables {
    pub canopus: Vec<Canopus>,
    pub structure: Vec<Structure>,
    pub spectral: Vec<SpectralMatch>,
}

fn canopus_rows(tables: &[SiriusTable]) -> Vec<Canopus> {
    let mut out = Vec::new();
    for t in tables {
        for (r, id) in t.ids.iter().enumerate() {
            let Some(id) = id else { continue };
            out.push(Canopus {
                pol: t.pol.clone(),
                id: *id,
                classes: CANOPUS_CLASSES.map(|n| text_at(&t.frame, r, n)),
                probs: CANOPUS_PROBS.map(|n| num_at(&t.frame, r, n)),
            });
        }
    }
    out
}

fn structure_rows(tables: &[SiriusTable]) -> Vec<Structure> {
    let score = |f: &Frame, r: usize, n: &str| num_at(f, r, n).filter(|v| *v != f64::NEG_INFINITY);
    let mut out = Vec::new();
    for t in tables {
        for (r, id) in t.ids.iter().enumerate() {
            let Some(id) = id else { continue };
            let f = &t.frame;
            out.push(Structure {
                pol: t.pol.clone(),
                id: *id,
                name: text_at(f, r, "name"),
                formula: text_at(f, r, "molecularFormula"),
                xlogp: num_at(f, r, "xlogp"),
                conf: score(f, r, "ConfidenceScoreExact"),
                cosmic: score(f, r, "ConfidenceScoreApproximate"),
                inchikey2d: text_at(f, r, "InChIkey2D"),
                smiles: text_at(f, r, "smiles"),
            });
        }
    }
    out
}

fn spectral_rows(tables: &[SiriusTable]) -> Result<Vec<SpectralMatch>> {
    let mut rows = Vec::new();
    for t in tables {
        let f = &t.frame;
        if f.is_empty() {
            continue;
        }
        for name in ["similarity", "sharedPeaks"] {
            if f.col(name).is_none() {
                return Err(CampaignError::value(format!("KeyError: '{name}'")));
            }
        }
        for (r, id) in t.ids.iter().enumerate() {
            let Some(id) = id else { continue };
            rows.push(SpectralMatch {
                pol: t.pol.clone(),
                id: *id,
                name: text_at(f, r, "referenceName"),
                sim: num_at(f, r, "similarity"),
                peaks: num_at(f, r, "sharedPeaks"),
                inchikey2d: text_at(f, r, "InChIkey2D"),
            });
        }
    }
    rows.sort_by(|a, b| desc_na_last(a.sim, b.sim));
    let mut seen = HashSet::new();
    rows.retain(|m| seen.insert((m.pol.clone(), m.id)));
    Ok(rows)
}

/// `load_sirius_tables`: the CANOPUS classes, the CSI:FingerID structures and SIRIUS's spectral matches of both polarities.
pub fn load_sirius_tables(c: &Campaign, io: &Io) -> Result<SiriusTables> {
    let canopus = load_sirius(c, io, "canopus_structure_summary", None, true)?;
    let structure = load_sirius(c, io, "structure_identifications", None, true)?;
    let spectral = load_sirius(c, io, "spectral_matches", None, true)?;
    Ok(SiriusTables {
        canopus: canopus_rows(&canopus),
        structure: structure_rows(&structure),
        spectral: spectral_rows(&spectral)?,
    })
}
