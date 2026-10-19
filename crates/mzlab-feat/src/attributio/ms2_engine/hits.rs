// Ported from Attributio ms2_engine/generate.py and readers/ms2_engine.py.
//! The engine hits file of a campaign (`<data>/engine/engine_<pos|neg>.csv`): one row per feature with the best hit. Written by an explicit command and read
//! only when `use_ms2_engine` is on; reading never writes into the campaign.

use super::matcher::SearchHit;
use crate::attributio::frame::to_csv;

pub const ENGINE_COLUMNS: [&str; 6] = [
    "_pol",
    "_id",
    "engine_name",
    "engine_score",
    "engine_precursor_mz",
    "engine_id",
];

#[derive(Debug, Clone, PartialEq)]
pub struct EngineHitRow {
    pub pol: String,
    pub id: i64,
    pub name: String,
    pub score: f64,
    pub precursor_mz: f64,
    pub library_id: String,
}

impl EngineHitRow {
    /// The best hit of a feature; the name falls back to the library id when the cleaned name is empty.
    pub fn from_hit(pol: &str, id: i64, h: &SearchHit) -> EngineHitRow {
        EngineHitRow {
            pol: pol.into(),
            id,
            name: if h.name.is_empty() {
                h.id.clone()
            } else {
                h.name.clone()
            },
            score: h.score,
            precursor_mz: h.library_precursor_mz,
            library_id: h.id.clone(),
        }
    }
}

/// CSV text of the rows (comma separated, as pandas writes it).
pub fn engine_hits_csv(rows: &[EngineHitRow]) -> String {
    let cols: Vec<String> = ENGINE_COLUMNS.iter().map(|s| s.to_string()).collect();
    let body: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            vec![
                r.pol.clone(),
                r.id.to_string(),
                r.name.clone(),
                r.score.to_string(),
                r.precursor_mz.to_string(),
                r.library_id.clone(),
            ]
        })
        .collect();
    to_csv(&cols, &body, ',')
}

/// Rows of an engine hits file; a missing column or a row without a numeric id is skipped.
pub fn read_engine_hits(text: &str) -> Vec<EngineHitRow> {
    let Ok(f) = crate::attributio::frame::Frame::parse(text, ',') else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for r in 0..f.len() {
        let g = |c: &str| {
            f.col(c)
                .map(|k| f.cell(r, k).to_string())
                .unwrap_or_default()
        };
        let n = |c: &str| f.col(c).and_then(|k| f.num(r, k));
        let Some(id) = n("_id") else { continue };
        out.push(EngineHitRow {
            pol: g("_pol"),
            id: id as i64,
            name: g("engine_name"),
            score: n("engine_score").unwrap_or(0.0),
            precursor_mz: n("engine_precursor_mz").unwrap_or(0.0),
            library_id: g("engine_id"),
        });
    }
    out
}
