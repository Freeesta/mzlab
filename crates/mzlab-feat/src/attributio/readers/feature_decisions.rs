// Ported from Attributio readers/feature_decisions.py.
//! The chemist's decisions on single features of a campaign: one `;`-separated file declared in `[curation] feature_decisions`, with the columns
//! `pol;feature_id;mz;rt;livello;nome;identita;decided_on;decided_by;motivazione` and the optional ones (`group`, the evidence, `rt_check`, the provenance of
//! the review, and `evidence_checklist`: the checks the chemist ticked before a 2b, separated by `|`; read if there, always written last).
//! `livello` `2b` (diagnostic evidence, needs `motivazione`), `3` (a Level 2 goes to Level 3; never raises a level), `2` (a record) or empty; `nome` replaces the
//! feature's name before curation; `group` `ambiguous` takes the identity out of every count. The file is a history: the rows of one feature apply in file
//! order and a later row supersedes the earlier ones; a decision is per FEATURE (found again by m/z and RT when mzMine is re-run), never per name.

use super::standards::{DECISION_MZ_PPM, DECISION_RT_MIN};
use crate::attributio::error::{py_list, CampaignError, Result};
use crate::attributio::frame::{to_csv, to_num, Frame};
use std::collections::BTreeSet;

pub const COLUMNS: [&str; 10] = [
    "pol",
    "feature_id",
    "mz",
    "rt",
    "livello",
    "nome",
    "identita",
    "decided_on",
    "decided_by",
    "motivazione",
];
pub const EVIDENCE_COLUMNS: [&str; 8] = [
    "evidence_type",
    "evidence_library",
    "evidence_entry",
    "evidence_cosine",
    "evidence_matched",
    "evidence_delta_mz",
    "evidence_delta_formula",
    "evidence_note",
];
pub const OPTIONAL_COLUMNS: [&str; 16] = [
    "group",
    "evidence_type",
    "evidence_library",
    "evidence_entry",
    "evidence_cosine",
    "evidence_matched",
    "evidence_delta_mz",
    "evidence_delta_formula",
    "evidence_note",
    "rt_check",
    "app_version",
    "settings_hash",
    "settings_used",
    "evidence_ref_hash",
    "evidence_ref_source",
    "evidence_checklist",
];
/// The fields a decision is compared on when the review app saves one.
pub const DECISION_FIELDS: [&str; 14] = [
    "livello",
    "nome",
    "group",
    "motivazione",
    "evidence_type",
    "evidence_library",
    "evidence_entry",
    "evidence_cosine",
    "evidence_matched",
    "evidence_delta_mz",
    "evidence_delta_formula",
    "evidence_note",
    "rt_check",
    "evidence_checklist",
];

/// Every column of the file, in the order it is written.
pub fn all_columns() -> Vec<String> {
    COLUMNS
        .iter()
        .chain(&OPTIONAL_COLUMNS)
        .map(|s| s.to_string())
        .collect()
}

/// The known columns followed by the others a file holds.
fn with_extra(have: &[String]) -> Vec<String> {
    let mut cols = all_columns();
    for c in have {
        if !cols.contains(c) {
            cols.push(c.clone());
        }
    }
    cols
}

/// The decisions as text, the way the file holds them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DecisionTable {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl DecisionTable {
    pub fn empty() -> DecisionTable {
        DecisionTable {
            columns: all_columns(),
            rows: Vec::new(),
        }
    }
    pub fn get(&self, r: usize, col: &str) -> &str {
        self.columns
            .iter()
            .position(|c| c == col)
            .and_then(|k| self.rows[r].get(k))
            .map_or("", |s| s.as_str())
    }
    pub fn len(&self) -> usize {
        self.rows.len()
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    fn set(&mut self, r: usize, col: &str, value: &str) {
        if let Some(k) = self.columns.iter().position(|c| c == col) {
            self.rows[r][k] = value.to_string();
        }
    }
}

fn bad_values(t: &DecisionTable, col: &str, allowed: &[&str]) -> Vec<String> {
    let set: BTreeSet<&str> = (0..t.len())
        .map(|r| t.get(r, col))
        .filter(|v| !allowed.contains(v))
        .collect();
    set.into_iter().map(String::from).collect()
}

fn check_values(
    t: &DecisionTable,
    path: &str,
    col: &str,
    allowed: &[&str],
    what: &str,
) -> Result<()> {
    let bad = bad_values(t, col, allowed);
    if bad.is_empty() {
        return Ok(());
    }
    Err(CampaignError::value(format!(
        "{path}: {what}, found {}",
        py_list(&bad)
    )))
}

/// The decisions file, or the empty table when there is no file. `text` is the file, `path` its name in messages.
pub fn load_feature_decisions(text: Option<&str>, path: &str) -> Result<DecisionTable> {
    let Some(text) = text else {
        return Ok(DecisionTable::empty());
    };
    let f = Frame::parse(text, ';')?;
    let missing: Vec<&str> = COLUMNS
        .iter()
        .copied()
        .filter(|c| f.col(c).is_none())
        .collect();
    if !missing.is_empty() {
        let mut m = missing;
        m.sort_unstable();
        return Err(CampaignError::value(format!(
            "{path}: missing columns {}",
            py_list(&m)
        )));
    }
    let mut t = DecisionTable {
        columns: f.columns().to_vec(),
        rows: Vec::new(),
    };
    for r in 0..f.len() {
        t.rows.push(
            (0..f.columns().len())
                .map(|k| f.cell(r, k).to_string())
                .collect(),
        );
    }
    for c in OPTIONAL_COLUMNS {
        if !t.columns.iter().any(|x| x == c) {
            t.columns.push(c.to_string());
            t.rows.iter_mut().for_each(|row| row.push(String::new()));
        }
    }
    let strip = ["pol", "livello", "nome"]
        .into_iter()
        .chain(OPTIONAL_COLUMNS);
    for c in strip {
        let k = t.columns.iter().position(|x| x == c).unwrap_or(0);
        for row in t.rows.iter_mut() {
            row[k] = if c == "pol" {
                row[k].trim().to_uppercase()
            } else {
                row[k].trim().to_string()
            };
        }
    }
    check_values(
        &t,
        path,
        "livello",
        &["", "2", "2b", "3"],
        "livello must be '2b', '3', '2' or empty",
    )?;
    check_values(
        &t,
        path,
        "group",
        &["", "ambiguous"],
        "group must be 'ambiguous' or empty",
    )?;
    check_values(
        &t,
        path,
        "evidence_type",
        &["", "library", "analog", "fragments"],
        "evidence_type must be library, analog, fragments or empty",
    )?;
    check_values(
        &t,
        path,
        "rt_check",
        &["", "peak", "isomer"],
        "rt_check must be 'peak', 'isomer' or empty",
    )?;
    let unjustified: Vec<String> = (0..t.len())
        .filter(|&r| t.get(r, "livello") == "2b" && t.get(r, "motivazione").trim().is_empty())
        .map(|r| format!("{} {}", t.get(r, "pol"), t.get(r, "feature_id")))
        .collect();
    if !unjustified.is_empty() {
        return Err(CampaignError::value(format!(
            "{path}: a Level 2b decision needs 'motivazione' ({})",
            unjustified.join(", ")
        )));
    }
    Ok(t)
}

/// A feature of the loaded table: polarity, id, m/z, RT.
#[derive(Clone, Debug, PartialEq)]
pub struct FeatureKey {
    pub pol: String,
    pub id: i64,
    pub mz: Option<f64>,
    pub rt: Option<f64>,
}

/// The feature id of decision `r` among `table`: (pol, feature_id) when its m/z and RT agree with the file (or the file has none), otherwise m/z + RT alone when
/// exactly one feature fits; `None` when no feature fits or several do.
pub fn find_feature(table: &[FeatureKey], d: &DecisionTable, r: usize) -> Option<i64> {
    let xmz = to_num(d.get(r, "mz"));
    let xrt = to_num(d.get(r, "rt"));
    let pol = d.get(r, "pol");
    let t: Vec<&FeatureKey> = table.iter().filter(|f| f.pol == pol).collect();
    let near = |f: &FeatureKey| match (xmz, xrt) {
        (Some(m), Some(t)) => {
            f.mz.is_some_and(|v| (v - m).abs() / m * 1e6 <= DECISION_MZ_PPM)
                && f.rt.is_some_and(|v| (v - t).abs() <= DECISION_RT_MIN)
        }
        _ => true,
    };
    let fid = to_num(d.get(r, "feature_id"));
    if let Some(f) = t.iter().find(|f| fid == Some(f.id as f64) && near(f)) {
        return Some(f.id);
    }
    if xmz.is_some() && xrt.is_some() {
        let fits: Vec<&&FeatureKey> = t.iter().filter(|f| near(f)).collect();
        if fits.len() == 1 {
            return Some(fits[0].id);
        }
    }
    None
}

/// A decision that applies to a feature of the loaded table.
#[derive(Clone, Debug, PartialEq)]
pub struct MatchedDecision {
    pub row: usize,
    pub livello: String,
    pub nome: String,
    pub motivazione: String,
    pub group: String,
    pub evidence: Vec<(String, String)>,
}

/// For each decision the index of its feature in `feat`. A decision is looked up in `all_features` (every feature of the merged tables; `feat` itself when
/// `None`) and warns only when the feature does not exist there; it applies when that feature is among the loaded ones.
pub fn match_decisions(
    feat: &[FeatureKey],
    d: &DecisionTable,
    all_features: Option<&[FeatureKey]>,
    warnings: &mut Vec<String>,
) -> Vec<MatchedDecision> {
    let reference = all_features.unwrap_or(feat);
    let mut out = Vec::new();
    for r in 0..d.len() {
        let Some(fid) = find_feature(reference, d, r) else {
            warnings.push(format!(
                "feature decision not applied (feature not found or not unique): {} {} {} -- check feature_decisions.csv",
                d.get(r, "pol"),
                d.get(r, "feature_id"),
                d.get(r, "identita")
            ));
            continue;
        };
        let Some(row) = feat
            .iter()
            .position(|f| f.pol == d.get(r, "pol") && f.id == fid)
        else {
            continue;
        };
        out.push(MatchedDecision {
            row,
            livello: d.get(r, "livello").into(),
            nome: d.get(r, "nome").into(),
            motivazione: d.get(r, "motivazione").into(),
            group: d.get(r, "group").into(),
            evidence: EVIDENCE_COLUMNS
                .iter()
                .map(|c| (c.to_string(), d.get(r, c).to_string()))
                .collect(),
        });
    }
    out
}

/// True when two decision rows are about the same feature: same polarity and either the same feature id with m/z and RT within the decision tolerances, or
/// (the ids of another mzMine run) the same m/z and RT. The rule of `find_feature`, applied between two files.
pub fn same_feature(a: &DecisionTable, ra: usize, b: &DecisionTable, rb: usize) -> bool {
    if a.get(ra, "pol").trim().to_uppercase() != b.get(rb, "pol").trim().to_uppercase() {
        return false;
    }
    let (amz, bmz, art, brt) = (
        to_num(a.get(ra, "mz")),
        to_num(b.get(rb, "mz")),
        to_num(a.get(ra, "rt")),
        to_num(b.get(rb, "rt")),
    );
    let near = match (amz, bmz, art, brt) {
        (Some(am), Some(bm), Some(at), Some(bt)) => {
            (am - bm).abs() / bm * 1e6 <= DECISION_MZ_PPM && (at - bt).abs() <= DECISION_RT_MIN
        }
        _ => true,
    };
    let same_id = a.get(ra, "feature_id").trim() == b.get(rb, "feature_id").trim();
    near && (same_id || (amz.is_some() && art.is_some()))
}

/// What `merge_decisions` did with the new rows.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MergeStats {
    pub replaced: usize,
    pub added: usize,
    pub unchanged: usize,
}

/// `existing` with the rows of `new` (the review app's decisions). The older decisions are never overwritten. For each new row: `unchanged` when the last row
/// of its feature already says the same thing (nothing is written); `replaced` when that last row has the same `decided_on` and `decided_by` (the chemist
/// revises the decision of the day: the row is replaced); `added` otherwise (appended after the older ones, which it supersedes).
pub fn merge_decisions(
    existing: &DecisionTable,
    new: &DecisionTable,
) -> (DecisionTable, MergeStats) {
    let cols = with_extra(&existing.columns);
    let mut ex = DecisionTable {
        rows: Vec::new(),
        columns: cols,
    };
    for r in 0..existing.len() {
        let row = ex
            .columns
            .iter()
            .map(|c| existing.get(r, c).to_string())
            .collect();
        ex.rows.push(row);
    }
    let mut stats = MergeStats::default();
    for n in 0..new.len() {
        let row: Vec<String> = ex
            .columns
            .iter()
            .map(|c| new.get(n, c).trim().to_string())
            .collect();
        let incoming = DecisionTable {
            columns: ex.columns.clone(),
            rows: vec![row.clone()],
        };
        let last = (0..ex.len())
            .rev()
            .find(|&i| same_feature(&ex, i, &incoming, 0));
        match last {
            Some(i)
                if DECISION_FIELDS
                    .iter()
                    .all(|c| ex.get(i, c).trim() == incoming.get(0, c)) =>
            {
                stats.unchanged += 1
            }
            Some(i)
                if ex.get(i, "decided_on").trim() == incoming.get(0, "decided_on")
                    && ex.get(i, "decided_by").trim() == incoming.get(0, "decided_by") =>
            {
                ex.rows[i] = row;
                stats.replaced += 1;
            }
            _ => {
                ex.rows.push(row);
                stats.added += 1;
            }
        }
    }
    (ex, stats)
}

/// The text of a decisions file (`;`-separated, every known column first, `evidence_checklist` last among them), for the caller to write atomically.
pub fn write_feature_decisions(t: &DecisionTable) -> String {
    let cols = with_extra(&t.columns);
    let rows: Vec<Vec<String>> = (0..t.len())
        .map(|r| cols.iter().map(|c| t.get(r, c).to_string()).collect())
        .collect();
    to_csv(&cols, &rows, ';')
}

impl DecisionTable {
    /// Sets one cell by column name (the column must exist).
    pub fn put(&mut self, r: usize, col: &str, value: &str) {
        self.set(r, col, value);
    }
}
