// Ported from Attributio diagnostics/schymanski_export.py.
//! The cells of a row of the Schymanski workbook: the score and the reason of the level, the library match, the sources and the sites.

use super::ctx::FRow;
use super::pyfmt::{fixed0, float_or_nan, join_distinct, py_float};
use super::schymanski_delta::interpret_delta;
use crate::attributio::frame::Cell;
use crate::attributio::identity::IdentityRow;
use crate::attributio::readers::standards::py_round;
use crate::sortc::SortC;
use std::collections::HashMap;

pub(super) const PPM_DIRECT_MAX: f64 = 20.0;
pub(super) const NONCEC_BUCKET_LABEL: &str = "Other / non-target";
pub(super) fn max_of(v: impl Iterator<Item = Option<f64>>) -> Option<f64> {
    v.flatten().reduce(f64::max)
}

/// The score that decided the level: MS/MS cosine against the standard (1), the best library score (2), COSMIC (3).
pub(super) fn confidence(sub: &[&FRow], level: i32) -> Option<f64> {
    match level {
        1 => max_of(sub.iter().map(|r| r.f.std.as_ref().and_then(|s| s.cosine))),
        2 => max_of(
            sub.iter()
                .flat_map(|r| [r.f.spec.sim, r.f.gnps.score, r.f.mzlib.score]),
        ),
        _ => max_of(sub.iter().map(|r| r.f.csi.cosmic)),
    }
}

pub(super) fn why_level(sub: &[&FRow], level: i32) -> Option<String> {
    match level {
        1 => {
            let key = |r: &&FRow| r.f.std.as_ref().and_then(|s| s.cosine).unwrap_or(-1.0);
            let mut best = sub[0];
            for r in &sub[1..] {
                if key(r) > key(&best) {
                    best = r;
                }
            }
            let s = best.f.std.as_ref()?;
            Some(format!(
                "reference standard {}: {} ppm, dRT {} min, MS/MS cosine {} on {} fragments ({})",
                s.name.as_deref().unwrap_or("None"),
                py_float(s.ppm),
                py_float(s.drt),
                float_or_nan(s.cosine),
                fixed0(s.matched.map(|m| m as f64)),
                s.status
            ))
        }
        3 => {
            let has = |b: &str| sub.iter().any(|r| r.f.level_basis == b);
            let mut parts = Vec::new();
            if has("CSI:FingerID") {
                parts.push("CSI:FingerID structure with COSMIC ≥ threshold (~10% FDR)".to_string());
            }
            if has("suspect-RT") {
                let named: Vec<&&FRow> = sub
                    .iter()
                    .filter(|r| r.f.std.as_ref().is_some_and(|s| s.name.is_some()))
                    .collect();
                let names = join_distinct(
                    named
                        .iter()
                        .filter_map(|r| r.f.std.as_ref().and_then(|s| s.name.as_deref())),
                );
                let status = join_distinct(
                    named
                        .iter()
                        .filter_map(|r| r.f.std.as_ref().map(|s| s.status.as_str())),
                );
                parts.push(format!(
                    "reference standard {names} matches mass + RT, MS/MS {status} (not a Level 1)"
                ));
            }
            Some(format!(
                "{}; no spectral-library match above threshold",
                parts.join("; ")
            ))
        }
        _ => None,
    }
}

pub(super) fn first_max<T>(items: &[T], key: impl Fn(&T) -> Option<f64>) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (i, it) in items.iter().enumerate() {
        if let Some(v) = key(it) {
            if best.is_none_or(|(_, b)| v > b) {
                best = Some((i, v));
            }
        }
    }
    best.map(|b| b.0)
}

pub(super) fn num(v: Option<f64>) -> Cell {
    Cell::num(v)
}

pub(super) fn text(s: String) -> Cell {
    Cell::Str(s)
}

pub(super) fn locations(sub: &[&FRow]) -> String {
    sub.iter()
        .map(|r| format!("{}:{}", r.f.pol, r.f.id))
        .collect::<Vec<_>>()
        .join("; ")
}

pub(super) type Files = HashMap<String, String>;
pub(super) type Pairs = Vec<(String, Cell)>;

pub(super) fn kv(k: &str, v: Cell) -> (String, Cell) {
    (k.to_string(), v)
}

/// The match columns: peaks, error and mass difference of the best library hit and how to read it.
pub(super) fn match_cells(sub: &[&FRow], level: i32) -> Pairs {
    let best = first_max(sub, |r| r.matched_peaks());
    let (mut peaks, mut ppm, mut md) = (Cell::Null, None, None);
    if let Some(b) = best {
        peaks = Cell::Int(sub[b].matched_peaks().unwrap_or(0.0) as i64);
        ppm = sub[b].gp.mz_error_ppm.map(|x| py_round(x, 2));
        md = sub[b].gp.mass_diff.map(|x| py_round(x, 4));
    }
    let match_type = match ppm {
        None => (level == 2).then_some("mzMine (no ppm)"),
        Some(p) => Some(if p.abs() <= PPM_DIRECT_MAX {
            "direct"
        } else {
            "analog"
        }),
    };
    let (hyp, cls) = match (match_type, md) {
        (Some("analog"), Some(m)) => {
            let (h, c) = interpret_delta(-m);
            (Some(h), Some(c))
        }
        _ => (None, None),
    };
    vec![
        kv(
            "match_type",
            match_type.map_or(Cell::Null, |m| text(m.into())),
        ),
        kv("putative_class", cls.map_or(Cell::Null, |m| text(m.into()))),
        kv("analog_hypothesis", hyp.map_or(Cell::Null, text)),
        kv("mass_diff_Da", num(md)),
        kv("matched_peaks", peaks),
        kv("mz_error_ppm", num(ppm)),
    ]
}

pub(super) fn level3_cells(sub: &[&FRow]) -> Pairs {
    let class = |i: usize| join_distinct(sub.iter().filter_map(|r| r.f.classes[i].as_deref()));
    let z = first_max(sub, |r| r.rt_z.map(f64::abs));
    let (zc, check) = match z {
        Some(w) => (
            num(sub[w].rt_z.map(|v| py_round(v, 3))),
            sub[w].rt_note.clone().unwrap_or_else(|| "nan".into()),
        ),
        None => {
            let j = join_distinct(sub.iter().filter_map(|r| r.rt_note.as_deref()));
            (
                Cell::Null,
                if j.is_empty() {
                    "not evaluated".into()
                } else {
                    j
                },
            )
        }
    };
    vec![
        kv("npc_pathway", text(class(0))),
        kv("npc_class", text(class(2))),
        kv("rt_logp_z", zc),
        kv("rt_logp_check", text(check)),
    ]
}

/// The source columns: sources, library, formula, structure, adducts, representative ion and the files of the feature tables.
pub(super) fn source_cells(sub: &[&FRow], rep: &FRow, files: &Files) -> Pairs {
    let join_of = |parts: Vec<Vec<&str>>| join_distinct(parts.into_iter().flatten());
    vec![
        kv(
            "evidence_sources",
            text(join_distinct(
                sub.iter().flat_map(|r| r.f.sources.iter().copied()),
            )),
        ),
        kv(
            "library",
            text(join_of(vec![
                sub.iter().filter_map(|r| r.gp.library.as_deref()).collect(),
                sub.iter().filter_map(|r| r.mz_method.as_deref()).collect(),
            ])),
        ),
        kv(
            "neutral_formula",
            text(join_of(vec![
                sub.iter()
                    .filter_map(|r| r.f.mzlib.formula.as_deref())
                    .collect(),
                sub.iter()
                    .filter_map(|r| r.f.csi.formula.as_deref())
                    .collect(),
            ])),
        ),
        kv(
            "inchikey",
            text(join_distinct(sub.iter().filter_map(|r| r.match_inchikey()))),
        ),
        kv(
            "smiles",
            text(join_distinct(sub.iter().filter_map(|r| r.match_smiles()))),
        ),
        kv(
            "adducts",
            text(join_of(vec![
                sub.iter()
                    .filter_map(|r| r.f.mzlib.adduct.as_deref())
                    .collect(),
                sub.iter().filter_map(|r| r.gp.adduct.as_deref()).collect(),
            ])),
        ),
        kv(
            "polarity",
            text(join_distinct(sub.iter().map(|r| r.f.pol.as_str()))),
        ),
        kv("rt_min_repr", num(rep.f.rt.map(|v| py_round(v, 3)))),
        kv("mz_repr", num(rep.f.mz.map(|v| py_round(v, 4)))),
        kv("n_features_collapsed", Cell::Int(sub.len() as i64)),
        kv("feature_locations (pol:mzMine_id)", text(locations(sub))),
        kv(
            "mzMine_files",
            text(join_distinct(
                sub.iter()
                    .filter_map(|r| files.get(&r.f.pol).map(String::as_str)),
            )),
        ),
        kv(
            "gnps_network_component",
            rep.f.gnps_component.map_or(Cell::Null, Cell::Int),
        ),
    ]
}

/// `detected_<site>` and `area_sum_<site>` of each site.
pub(super) fn site_cells(sub: &[&FRow], short: &[String]) -> Pairs {
    let mut row = Vec::new();
    for (i, s) in short.iter().enumerate() {
        let a: f64 = sub.iter().map(|r| r.area[i]).sum();
        row.push((
            format!("detected_{s}"),
            text(if a > 0.0 { "yes" } else { "" }.into()),
        ));
        row.push((
            format!("area_sum_{s}"),
            if a > 0.0 { Cell::Num(a) } else { Cell::Null },
        ));
    }
    row
}

pub(super) fn identity_row(
    level: i32,
    meta: &IdentityRow,
    sub: &[&FRow],
    short: &[String],
    files: &Files,
) -> Pairs {
    let rep = sub[first_max(sub, |r| Some(r.total_area)).unwrap_or(0)];
    let conf = confidence(sub, level).map(|v| py_round(v, 4));
    let category = if meta.identity_category == "natural" {
        NONCEC_BUCKET_LABEL.to_string()
    } else {
        meta.identity_category.clone()
    };
    let mut row = vec![
        kv("identity", text(meta.identity.clone())),
        kv("category", text(category)),
        kv("is_contaminant_CEC", Cell::Bool(meta.is_cec)),
        kv("cec_category", Cell::str(&meta.cec_category)),
        kv("confidence_score", num(conf)),
        kv("why_level", why_level(sub, level).map_or(Cell::Null, text)),
    ];
    row.extend(match_cells(sub, level));
    row.extend(source_cells(sub, rep, files));
    row.extend(site_cells(sub, short));
    if level == 3 {
        row.extend(level3_cells(sub));
    }
    row
}

pub(super) fn cell_key(p: &Pairs, name: &str) -> Option<Cell> {
    p.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
}

/// The order of the rows of an identity sheet: contaminants first, then by confidence, the missing ones last.
pub(super) fn sort_rows(rows: &mut [Pairs]) {
    rows.sortc_by(|a, b| {
        let cec = |p: &Pairs| matches!(cell_key(p, "is_contaminant_CEC"), Some(Cell::Bool(true)));
        let conf = |p: &Pairs| match cell_key(p, "confidence_score") {
            Some(Cell::Num(v)) => Some(v),
            _ => None,
        };
        cec(b)
            .cmp(&cec(a))
            .then(crate::attributio::readers::desc_na_last(conf(a), conf(b)))
    });
}
