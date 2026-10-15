// Ported from Attributio readers/gnps.py.
//! GNPS2 / FBMN exports: the library search and the molecular network. `#Scan#` is the mzMine feature id (the FBMN export keeps it). With Analog Search ON
//! GNPS2 also returns hits with a mass shift: those are analogues of the library compound, not the compound; `exact` flags hits within the tolerance and the
//! best hit prefers exact matches, then MQScore.

use super::{desc_na_last, num_at, read_table, text_at};
use crate::attributio::campaign::Campaign;
use crate::attributio::error::Result;
use crate::attributio::frame::{Cell, Record};
use crate::attributio::io::{join, Io};
use std::collections::{HashMap, HashSet};

const DB_PREFIXES: [&str; 5] = [
    "Massbank:",
    "MassbankEU:",
    "MoNA:",
    "ReSpect:",
    "Spectral Match to ",
];
const INSTR_SUFFIXES: [&str; 4] = [" from NIST14", " - 30.0 eV", " - 40.0 eV", " - 20.0 eV"];

/// `^(?:[A-Z]{2,3}\d{5,}|\d{5,})\s+`: the leading accession of an entry (MassBank, ReSpect, a bare MoNA or GNPS number); the length to remove.
fn accession_len(s: &str) -> usize {
    let b = s.as_bytes();
    let letters = b.iter().take_while(|c| c.is_ascii_uppercase()).count();
    if letters == 1 || letters > 3 {
        return 0;
    }
    let digits = b[letters..]
        .iter()
        .take_while(|c| c.is_ascii_digit())
        .count();
    if digits < 5 {
        return 0;
    }
    let at = letters + digits;
    let ws = s[at..].len() - s[at..].trim_start().len();
    if ws == 0 {
        0
    } else {
        at + ws
    }
}

/// `_CE\d+(?:\.\d+)?$`: the collision energy tag at the end of a name (`Caffeine_CE15`); the length to keep.
fn without_ce_suffix(s: &str) -> &str {
    let tail_ok = |t: &str| {
        let (int, frac) = t.split_once('.').map_or((t, None), |(a, b)| (a, Some(b)));
        let digits = |x: &str| !x.is_empty() && x.bytes().all(|c| c.is_ascii_digit());
        digits(int) && frac.is_none_or(digits)
    };
    for (i, _) in s.match_indices("_CE") {
        if tail_ok(&s[i + 3..]) {
            return &s[..i];
        }
    }
    s
}

/// Strips database prefixes, instrument suffixes and leading accessions from a library compound name (GNPS2 and mzMine library hits share the format).
pub fn clean_library_hit_name(name: &str) -> String {
    let mut n = name.trim().trim_matches('"').trim().to_string();
    for p in DB_PREFIXES {
        n = n.replace(p, "");
    }
    n = n.split('|').next().unwrap_or("").to_string();
    for s in INSTR_SUFFIXES {
        n = n.replace(s, "");
    }
    let n = n.trim();
    let n = n[accession_len(n)..].trim();
    without_ce_suffix(n).trim().to_string()
}

/// The best GNPS2 library hit of a feature.
#[derive(Clone, Debug, PartialEq)]
pub struct GnpsHit {
    pub pol: String,
    pub id: i64,
    pub name: String,
    pub score: Option<f64>,
    pub peaks: Option<f64>,
    pub massdiff: Option<f64>,
    pub exact: bool,
    pub inchikey: Option<String>,
    pub smiles: Option<String>,
}

impl Record for GnpsHit {
    const COLUMNS: &'static [&'static str] = &[
        "_pol",
        "_id",
        "gnps_name",
        "gnps_score",
        "gnps_peaks",
        "gnps_massdiff",
        "gnps_exact",
        "gnps_inchikey",
        "gnps_smiles",
    ];
    fn cells(&self) -> Vec<Cell> {
        vec![
            Cell::Str(self.pol.clone()),
            Cell::Int(self.id),
            Cell::Str(self.name.clone()),
            Cell::num(self.score),
            Cell::num(self.peaks),
            Cell::num(self.massdiff),
            Cell::Bool(self.exact),
            Cell::str(&self.inchikey),
            Cell::str(&self.smiles),
        ]
    }
}

/// The molecular family of a feature; component -1 is a singleton node, not an error.
#[derive(Clone, Debug, PartialEq)]
pub struct GnpsNode {
    pub pol: String,
    pub id: i64,
    pub component: Option<i64>,
    pub component_size: i64,
}

impl Record for GnpsNode {
    const COLUMNS: &'static [&'static str] =
        &["_pol", "_id", "gnps_component", "gnps_component_size"];
    fn cells(&self) -> Vec<Cell> {
        vec![
            Cell::Str(self.pol.clone()),
            Cell::Int(self.id),
            self.component.map_or(Cell::Null, Cell::Int),
            Cell::Int(self.component_size),
        ]
    }
}

fn dedup_first<T>(rows: Vec<T>, key: impl Fn(&T) -> (String, i64)) -> Vec<T> {
    let mut seen = HashSet::new();
    rows.into_iter().filter(|r| seen.insert(key(r))).collect()
}

/// The best GNPS2 library hit per feature of a polarity (exact matches first, then MQScore; ties by file order).
pub fn load_gnps_library(
    c: &Campaign,
    io: &Io,
    pol: &str,
    mass_tol_ppm: f64,
) -> Result<Vec<GnpsHit>> {
    let path = join(
        &c.gnps_output_dir(pol),
        "nf_output/library/merged_results_with_gnps.tsv",
    );
    let Some(g) = read_table(io, &path, '\t')? else {
        return Ok(Vec::new());
    };
    if g.is_empty() || g.col("#Scan#").is_none() {
        return Ok(Vec::new());
    }
    let name_col = g.col("Compound_Name").ok_or_else(|| {
        crate::attributio::error::CampaignError::value("KeyError: 'Compound_Name'")
    })?;
    let ik = ["InChIKey-Planar", "InChIKey"]
        .into_iter()
        .find(|n| g.col(n).is_some());
    let mut rows = Vec::new();
    for r in 0..g.len() {
        let Some(id) = num_at(&g, r, "#Scan#") else {
            continue;
        };
        let md = num_at(&g, r, "MassDiff").map(f64::abs);
        let exact = match (md, num_at(&g, r, "SpecMZ")) {
            (Some(m), Some(s)) => m / s * 1e6 <= mass_tol_ppm,
            _ => false,
        };
        rows.push(GnpsHit {
            pol: pol.to_string(),
            id: id as i64,
            name: clean_library_hit_name(if g.str_na(r, name_col).is_some() {
                g.cell(r, name_col)
            } else {
                "nan"
            }),
            score: num_at(&g, r, "MQScore"),
            peaks: num_at(&g, r, "SharedPeaks"),
            massdiff: md,
            exact,
            inchikey: ik.and_then(|n| text_at(&g, r, n)),
            smiles: text_at(&g, r, "Smiles"),
        });
    }
    rows.sort_by(|a, b| b.exact.cmp(&a.exact).then(desc_na_last(a.score, b.score)));
    Ok(dedup_first(rows, |h| (h.pol.clone(), h.id)))
}

/// The molecular-network component (molecular family) per feature of a polarity.
pub fn load_gnps_network(c: &Campaign, io: &Io, pol: &str) -> Result<Vec<GnpsNode>> {
    let path = join(
        &c.gnps_output_dir(pol),
        "nf_output/networking/clustersummary_with_network.tsv",
    );
    let Some(n) = read_table(io, &path, '\t')? else {
        return Ok(Vec::new());
    };
    if n.is_empty() || n.col("#Scan#").is_none() || n.col("component").is_none() {
        return Ok(Vec::new());
    }
    let mut rows: Vec<(i64, Option<i64>)> = Vec::new();
    for r in 0..n.len() {
        if let Some(id) = num_at(&n, r, "#Scan#") {
            rows.push((id as i64, num_at(&n, r, "component").map(|v| v as i64)));
        }
    }
    let mut sizes: HashMap<i64, i64> = HashMap::new();
    for comp in rows.iter().filter_map(|(_, c)| c.filter(|c| *c != -1)) {
        *sizes.entry(comp).or_default() += 1;
    }
    let nodes = rows
        .into_iter()
        .map(|(id, component)| GnpsNode {
            pol: pol.to_string(),
            id,
            component,
            component_size: component.and_then(|c| sizes.get(&c).copied()).unwrap_or(1),
        })
        .collect();
    Ok(dedup_first(nodes, |h| (h.pol.clone(), h.id)))
}
