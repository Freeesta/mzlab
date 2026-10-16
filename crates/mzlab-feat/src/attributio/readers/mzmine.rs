// Ported from Attributio readers/mzmine.py, campaign.py.
//! mzMine spectral-library matches of the merged alignment. The hit of a feature is MZmine's own spectral-library match written in the FULL merged feature
//! table (columns `spectral_db_matches:*`, one hit per feature): name, weighted cosine, number of shared fragment ions, ion type, formula, SMILES, InChI,
//! library precursor m/z. The annotation export (`merged_<pol>_annotations.csv`, several hits per feature, no ion count, and the rule-based lipid
//! annotations that are not spectral matches) only completes the hit with the InChIKey, the entry id and the USI of the matched spectrum.
//! The structural columns of the mzMine exports are unreliable (many unrelated compounds share one key): they are used only when their formula explains the
//! entry's own precursor (the cascade checks that).

use super::gnps::clean_library_hit_name;
use super::{num_at, read_table, text_at};
use crate::attributio::campaign::Campaign;
use crate::attributio::error::{CampaignError, Result};
use crate::attributio::frame::{is_na, to_num, Cell, Frame, Record};
use crate::attributio::io::{join, Io};
use std::collections::{HashMap, HashSet};

const FULL: &str = "spectral_db_matches:";

/// The hit of one feature.
#[derive(Clone, Debug, PartialEq)]
pub struct MzmineHit {
    pub pol: String,
    pub id: i64,
    pub name: String,
    pub score: Option<f64>,
    pub peaks: Option<f64>,
    pub method: String,
    pub inchikey: Option<String>,
    pub smiles: Option<String>,
    pub formula: Option<String>,
    pub precursor_mz: Option<f64>,
    pub adduct: Option<String>,
    pub entry: Option<String>,
    pub usi: Option<String>,
}

impl Record for MzmineHit {
    const COLUMNS: &'static [&'static str] = &[
        "_pol",
        "_id",
        "mzlib_name",
        "mzlib_score",
        "mzlib_peaks",
        "mzlib_method",
        "mzlib_inchikey",
        "mzlib_smiles",
        "mzlib_formula",
        "mzlib_precursor_mz",
        "mzlib_adduct",
        "mzlib_entry",
        "mzlib_usi",
    ];
    fn cells(&self) -> Vec<Cell> {
        vec![
            Cell::Str(self.pol.clone()),
            Cell::Int(self.id),
            Cell::Str(self.name.clone()),
            Cell::num(self.score),
            Cell::num(self.peaks),
            Cell::Str(self.method.clone()),
            Cell::str(&self.inchikey),
            Cell::str(&self.smiles),
            Cell::str(&self.formula),
            Cell::num(self.precursor_mz),
            Cell::str(&self.adduct),
            Cell::str(&self.entry),
            Cell::str(&self.usi),
        ]
    }
}

/// The annotation export of a polarity: the file `merged_<pol>_annotations.csv` of the merged folder, matched without regard to case.
pub fn annotation_file(c: &Campaign, io: &Io, pol: &str) -> Option<String> {
    let want = format!("merged_{}_annotations.csv", pol.to_lowercase());
    let dir = c.merged_dir(pol);
    if !io.is_dir(&dir) {
        return None;
    }
    let mut names: Vec<String> = io.list(&dir).into_iter().map(|e| e.name).collect();
    names.sort();
    names
        .into_iter()
        .find(|n| n.to_lowercase() == want)
        .map(|n| join(&dir, &n))
}

/// `M+H` (as the full table sometimes writes it) becomes `[M+H]+`; bracketed forms stay.
pub fn bracket_adduct(adduct: Option<&str>, pol: &str) -> Option<String> {
    let a = adduct?.trim();
    if a.is_empty() {
        return None;
    }
    let a = a.replace(' ', "");
    Some(if a.starts_with('[') {
        a
    } else {
        format!("[{a}]{}", if pol == "POS" { '+' } else { '-' })
    })
}

/// A row of the annotation export that is a spectral-library match.
struct AnnRow {
    id: i64,
    name: Option<String>,
    inchi: Option<String>,
    inchi_key: Option<String>,
    entry_id: Option<String>,
    usi: Option<String>,
}

/// `Series.astype(str)` of a column that pandas read as numbers: integers stay integers unless the column has missing values, which makes it float.
fn entry_texts(f: &Frame, col: &str) -> Vec<Option<String>> {
    let Some(k) = f.col(col) else {
        return vec![None; f.len()];
    };
    let cells: Vec<&str> = (0..f.len()).map(|r| f.cell(r, k)).collect();
    let numeric = cells.iter().all(|c| is_na(c.trim()) || to_num(c).is_some());
    let any_na = cells.iter().any(|c| is_na(c.trim()));
    let any_num = cells.iter().any(|c| to_num(c).is_some());
    cells
        .iter()
        .map(|c| {
            if is_na(c.trim()) {
                None
            } else if numeric && any_num {
                let v = to_num(c)?;
                Some(if any_na || v.fract() != 0.0 {
                    py_float(v)
                } else {
                    format!("{}", v as i64)
                })
            } else {
                Some(c.to_string())
            }
        })
        .collect()
}

/// `repr(float)` for the values that appear in identifiers.
fn py_float(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e16 {
        format!("{v:.1}")
    } else {
        format!("{v}")
    }
}

fn annotation_rows(c: &Campaign, io: &Io, pol: &str) -> Result<Vec<AnnRow>> {
    let Some(path) = annotation_file(c, io, pol) else {
        return Ok(Vec::new());
    };
    let Some(a) = read_table(io, &path, ',')? else {
        return Ok(Vec::new());
    };
    if a.is_empty() {
        return Ok(Vec::new());
    }
    if a.col("id").is_none() {
        return Err(CampaignError::value("KeyError: 'id'"));
    }
    let method = a.col("method");
    let entries = entry_texts(&a, "entry_id");
    let mut out = Vec::new();
    #[allow(clippy::needless_range_loop)]
    for r in 0..a.len() {
        if method.is_some_and(|k| a.cell(r, k).to_lowercase().contains("lipid")) {
            continue;
        }
        let Some(id) = num_at(&a, r, "id") else {
            continue;
        };
        out.push(AnnRow {
            id: id as i64,
            name: text_at(&a, r, "compound_name"),
            inchi: text_at(&a, r, "inchi"),
            inchi_key: text_at(&a, r, "inchi_key"),
            entry_id: entries[r].clone(),
            usi: text_at(&a, r, "usi"),
        });
    }
    Ok(out)
}

fn first_text<'a>(values: impl Iterator<Item = Option<&'a String>>) -> Option<String> {
    values.flatten().find(|v| !v.trim().is_empty()).cloned()
}

/// The annotation rows of each lookup key, in file order.
struct Lookups<'a> {
    by_name: HashMap<(i64, &'a str), Vec<&'a AnnRow>>,
    by_id_inchi: HashMap<(i64, &'a str), Vec<&'a AnnRow>>,
    by_inchi: HashMap<&'a str, Vec<&'a AnnRow>>,
}

fn lookups(rows: &[AnnRow]) -> Lookups<'_> {
    let mut l = Lookups {
        by_name: HashMap::new(),
        by_id_inchi: HashMap::new(),
        by_inchi: HashMap::new(),
    };
    for r in rows {
        if let Some(n) = &r.name {
            l.by_name.entry((r.id, n.as_str())).or_default().push(r);
        }
        if let Some(i) = &r.inchi {
            l.by_id_inchi.entry((r.id, i.as_str())).or_default().push(r);
            l.by_inchi.entry(i.as_str()).or_default().push(r);
        }
    }
    l
}

/// MZmine's spectral-library hit per feature (full merged table), completed with the InChIKey, entry id and USI of the annotation export.
pub fn load_mzmine_library(c: &Campaign, io: &Io, df: &Frame, pol: &str) -> Result<Vec<MzmineHit>> {
    let name_col = format!("{FULL}compound_name");
    let Some(nk) = df.col(&name_col) else {
        return Ok(Vec::new());
    };
    let ann = annotation_rows(c, io, pol)?;
    let look = lookups(&ann);
    let id_col = df
        .col("id")
        .ok_or_else(|| CampaignError::value("KeyError: 'id'"))?;
    let field = |r: usize, n: &str| text_at(df, r, &format!("{FULL}{n}"));
    let number = |r: usize, n: &str| num_at(df, r, &format!("{FULL}{n}"));
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for r in (0..df.len()).filter(|&r| df.str_na(r, nk).is_some()) {
        let id = df.int(r, id_col).unwrap_or(0);
        if !seen.insert(id) {
            continue;
        }
        let raw = df.cell(r, nk);
        let inchi = field(r, "inchi");
        let named = look.by_name.get(&(id, raw));
        let group = named.or_else(|| {
            let i = inchi.as_deref()?;
            look.by_id_inchi
                .get(&(id, i))
                .or_else(|| look.by_inchi.get(i))
        });
        out.push(MzmineHit {
            pol: pol.to_string(),
            id,
            name: clean_library_hit_name(raw),
            score: number(r, "similarity_score"),
            peaks: number(r, "n_matching_signals"),
            method: "spectral library".into(),
            inchikey: group.and_then(|g| first_text(g.iter().map(|a| a.inchi_key.as_ref()))),
            smiles: field(r, "smiles"),
            formula: field(r, "mol_formula"),
            precursor_mz: number(r, "precursor_mz"),
            adduct: bracket_adduct(field(r, "ion_adduct").as_deref(), pol),
            entry: named.and_then(|g| first_text(g.iter().map(|a| a.entry_id.as_ref()))),
            usi: named.and_then(|g| first_text(g.iter().map(|a| a.usi.as_ref()))),
        });
    }
    Ok(out)
}
