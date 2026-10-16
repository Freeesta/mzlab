// Ported from Attributio diagnostics/schymanski_export.py.
//! The Schymanski workbook: one sheet row per identity of Levels 1-3 with where it comes from and why it has its level, one per feature, the identities left
//! out of the count, and the legend. The sheets are cells (`Sheet`); the page makes the XLSX.

use super::cells::Sheet;
use super::ctx::{feature_rows, FRow, Loaded};
use super::pyfmt::{fixed0, float_or_nan, join_distinct, py_float, toml_text};
use super::wide::Wide;
use crate::attributio::campaign::Campaign;
use crate::attributio::error::Result;
use crate::attributio::frame::Cell;
use crate::attributio::identity::IdentityRow;
use crate::attributio::io::{file_name, Io};
use crate::attributio::quantify::{load_calibration, quantify_lvl1_environmental, QuantRow};
use crate::attributio::readers::standards::py_round;
use crate::attributio::sources::merged_table_path;
use std::collections::{BTreeMap, HashMap};

pub const LEVEL_TAG: [&str; 3] = ["L1", "L2a", "L3"];
const PPM_DIRECT_MAX: f64 = 20.0;
const NONCEC_BUCKET_LABEL: &str = "Other / non-target";
const DELTA_TOL: f64 = 0.01;
const CH2: f64 = 14.01565;
const DELTA_ARTIFACTS: [(f64, &str); 4] = [
    (1.003355, "13C isotopologue"),
    (0.984016, "deamidation (NH->OH)"),
    (21.981943, "Na/H adduct swap"),
    (37.955882, "K/H adduct swap"),
];
const DELTA_TRANSFORM: [(f64, &str); 16] = [
    (2.01565, "2H (reduction / (de)saturation)"),
    (15.994915, "O (oxidation / hydroxylation)"),
    (18.010565, "H2O ((de)hydration)"),
    (26.01565, "C2H2"),
    (27.994915, "CO"),
    (30.010565, "CH2O"),
    (42.010565, "C2H2O (acetylation)"),
    (43.98983, "CO2 ((de)carboxylation)"),
    (44.026215, "C2H4O"),
    (33.961028, "Cl<->H (chlorination)"),
    (79.956815, "SO3 (sulfation)"),
    (79.966331, "HPO3 (phosphorylation)"),
    (132.042259, "pentose (C5H8O4)"),
    (146.057909, "deoxyhexose (C6H10O4)"),
    (162.052824, "hexose (glycosylation, C6H10O5)"),
    (176.032088, "glucuronide (C6H8O6)"),
];

/// Columns that open the identity sheet, in this order.
pub const FRONT: [&str; 28] = [
    "schymanski_level",
    "identity",
    "category",
    "is_contaminant_CEC",
    "cec_category",
    "confidence_score",
    "why_level",
    "rt_logp_z",
    "rt_logp_check",
    "match_type",
    "putative_class",
    "analog_hypothesis",
    "mass_diff_Da",
    "matched_peaks",
    "mz_error_ppm",
    "evidence_sources",
    "library",
    "formula",
    "inchikey",
    "smiles",
    "adducts",
    "polarity",
    "rt_min_repr",
    "mz_repr",
    "n_features_collapsed",
    "feature_locations (pol:mzMine_id)",
    "mzMine_files",
    "gnps_network_component",
];

/// `_interpret_delta`: `(hypothesis, putative class)` from the mass difference (sample neutral mass minus library).
pub fn interpret_delta(dm: f64) -> (String, &'static str) {
    let a = dm.abs();
    let sign = if dm > 0.0 { "+" } else { "-" };
    if a < 0.02 {
        return (
            "near-isobaric: likely SAME compound (low-res library precursor)".into(),
            "near-isobaric",
        );
    }
    if let Some((_, n)) = DELTA_ARTIFACTS.iter().find(|(m, _)| (a - m).abs() <= DELTA_TOL) {
        return (format!("likely artifact: {n} (not a real transformation)"), "artifact");
    }
    let k = (a / CH2).round_ties_even();
    if k >= 1.0 && (a - k * CH2).abs() <= DELTA_TOL {
        return (
            format!("putative homolog: sample {sign}{}xCH2 vs library hit", k as i64),
            "homolog",
        );
    }
    if let Some((_, n)) = DELTA_TRANSFORM.iter().find(|(m, _)| (a - m).abs() <= DELTA_TOL) {
        return (
            format!("putative transformation: sample {sign}{n} vs library hit"),
            "transformation",
        );
    }
    (
        format!("unassigned (delta {sign}{a:.4} Da; analog, relationship unknown)"),
        "unassigned",
    )
}

fn max_of(v: impl Iterator<Item = Option<f64>>) -> Option<f64> {
    v.flatten().reduce(f64::max)
}

/// The score that decided the level: MS/MS cosine against the standard (1), the best library score (2), COSMIC (3).
fn confidence(sub: &[&FRow], level: i32) -> Option<f64> {
    match level {
        1 => max_of(sub.iter().map(|r| r.f.std.as_ref().and_then(|s| s.cosine))),
        2 => max_of(sub.iter().flat_map(|r| [r.f.spec.sim, r.f.gnps.score, r.f.mzlib.score])),
        _ => max_of(sub.iter().map(|r| r.f.csi.cosmic)),
    }
}

fn why_level(sub: &[&FRow], level: i32) -> Option<String> {
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
                let names = join_distinct(named.iter().filter_map(|r| r.f.std.as_ref().and_then(|s| s.name.as_deref())));
                let status = join_distinct(named.iter().filter_map(|r| r.f.std.as_ref().map(|s| s.status.as_str())));
                parts.push(format!(
                    "reference standard {names} matches mass + RT, MS/MS {status} (not a Level 1)"
                ));
            }
            Some(format!("{}; no spectral-library match above threshold", parts.join("; ")))
        }
        _ => None,
    }
}

fn first_max<T>(items: &[T], key: impl Fn(&T) -> Option<f64>) -> Option<usize> {
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

fn num(v: Option<f64>) -> Cell {
    Cell::num(v)
}

fn text(s: String) -> Cell {
    Cell::Str(s)
}

fn locations(sub: &[&FRow]) -> String {
    sub.iter()
        .map(|r| format!("{}:{}", r.f.pol, r.f.id))
        .collect::<Vec<_>>()
        .join("; ")
}

type Files = HashMap<String, String>;
type Pairs = Vec<(String, Cell)>;

fn kv(k: &str, v: Cell) -> (String, Cell) {
    (k.to_string(), v)
}

/// The match columns: peaks, error and mass difference of the best library hit and how to read it.
fn match_cells(sub: &[&FRow], level: i32) -> Pairs {
    let best = first_max(sub, |r| r.matched_peaks());
    let (mut peaks, mut ppm, mut md) = (Cell::Null, None, None);
    if let Some(b) = best {
        peaks = Cell::Int(sub[b].matched_peaks().unwrap_or(0.0) as i64);
        ppm = sub[b].gp.mz_error_ppm.map(|x| py_round(x, 2));
        md = sub[b].gp.mass_diff.map(|x| py_round(x, 4));
    }
    let match_type = match ppm {
        None => (level == 2).then_some("mzMine (no ppm)"),
        Some(p) => Some(if p.abs() <= PPM_DIRECT_MAX { "direct" } else { "analog" }),
    };
    let (hyp, cls) = match (match_type, md) {
        (Some("analog"), Some(m)) => {
            let (h, c) = interpret_delta(-m);
            (Some(h), Some(c))
        }
        _ => (None, None),
    };
    vec![
        kv("match_type", match_type.map_or(Cell::Null, |m| text(m.into()))),
        kv("putative_class", cls.map_or(Cell::Null, |m| text(m.into()))),
        kv("analog_hypothesis", hyp.map_or(Cell::Null, text)),
        kv("mass_diff_Da", num(md)),
        kv("matched_peaks", peaks),
        kv("mz_error_ppm", num(ppm)),
    ]
}

fn level3_cells(sub: &[&FRow]) -> Pairs {
    let class = |i: usize| join_distinct(sub.iter().filter_map(|r| r.f.classes[i].as_deref()));
    let z = first_max(sub, |r| r.rt_z.map(f64::abs));
    let (zc, check) = match z {
        Some(w) => (
            num(sub[w].rt_z.map(|v| py_round(v, 3))),
            sub[w].rt_note.clone().unwrap_or_else(|| "nan".into()),
        ),
        None => {
            let j = join_distinct(sub.iter().filter_map(|r| r.rt_note.as_deref()));
            (Cell::Null, if j.is_empty() { "not evaluated".into() } else { j })
        }
    };
    vec![
        kv("npc_pathway", text(class(0))),
        kv("npc_class", text(class(2))),
        kv("rt_logp_z", zc),
        kv("rt_logp_check", text(check)),
    ]
}

fn identity_row(
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
    let join_of = |parts: Vec<Vec<&str>>| join_distinct(parts.into_iter().flatten());
    let mut row = vec![
        kv("identity", text(meta.identity.clone())),
        kv("category", text(category)),
        kv("is_contaminant_CEC", Cell::Bool(meta.is_cec)),
        kv("cec_category", Cell::str(&meta.cec_category)),
        kv("confidence_score", num(conf)),
        kv("why_level", why_level(sub, level).map_or(Cell::Null, text)),
    ];
    row.extend(match_cells(sub, level));
    row.extend([
        kv("evidence_sources", text(join_distinct(sub.iter().flat_map(|r| r.f.sources.iter().copied())))),
        kv("library", text(join_of(vec![
            sub.iter().filter_map(|r| r.gp.library.as_deref()).collect(),
            sub.iter().filter_map(|r| r.mz_method.as_deref()).collect(),
        ]))),
        kv("formula", text(join_of(vec![
            sub.iter().filter_map(|r| r.f.mzlib.formula.as_deref()).collect(),
            sub.iter().filter_map(|r| r.f.csi.formula.as_deref()).collect(),
        ]))),
        kv("inchikey", text(join_distinct(sub.iter().filter_map(|r| r.match_inchikey())))),
        kv("smiles", text(join_distinct(sub.iter().filter_map(|r| r.match_smiles())))),
        kv("adducts", text(join_of(vec![
            sub.iter().filter_map(|r| r.f.mzlib.adduct.as_deref()).collect(),
            sub.iter().filter_map(|r| r.gp.adduct.as_deref()).collect(),
        ]))),
        kv("polarity", text(join_distinct(sub.iter().map(|r| r.f.pol.as_str())))),
        kv("rt_min_repr", num(rep.f.rt.map(|v| py_round(v, 3)))),
        kv("mz_repr", num(rep.f.mz.map(|v| py_round(v, 4)))),
        kv("n_features_collapsed", Cell::Int(sub.len() as i64)),
        kv("feature_locations (pol:mzMine_id)", text(locations(sub))),
        kv("mzMine_files", text(join_distinct(sub.iter().filter_map(|r| files.get(&r.f.pol).map(String::as_str))))),
        kv("gnps_network_component", rep.f.gnps_component.map_or(Cell::Null, Cell::Int)),
    ]);
    for (i, s) in short.iter().enumerate() {
        let a: f64 = sub.iter().map(|r| r.area[i]).sum();
        row.push((format!("detected_{s}"), text(if a > 0.0 { "yes" } else { "" }.into())));
        row.push((format!("area_sum_{s}"), if a > 0.0 { Cell::Num(a) } else { Cell::Null }));
    }
    if level == 3 {
        row.extend(level3_cells(sub));
    }
    row
}

fn cell_key(p: &Pairs, name: &str) -> Option<Cell> {
    p.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
}

/// The order of the rows of an identity sheet: contaminants first, then by confidence, the missing ones last.
fn sort_rows(rows: &mut [Pairs]) {
    rows.sort_by(|a, b| {
        let cec = |p: &Pairs| matches!(cell_key(p, "is_contaminant_CEC"), Some(Cell::Bool(true)));
        let conf = |p: &Pairs| match cell_key(p, "confidence_score") {
            Some(Cell::Num(v)) => Some(v),
            _ => None,
        };
        cec(b).cmp(&cec(a)).then(crate::attributio::readers::desc_na_last(conf(a), conf(b)))
    });
}

fn feature_cells(l: &Loaded, r: &FRow, files: &Files) -> Pairs {
    let f = r.f;
    let s = f.std.as_ref();
    let gp = &r.gp;
    let mut v = vec![
        kv("identity", text(r.identity.clone().unwrap_or_default())),
        kv("polarity", text(f.pol.clone())),
        kv("mzMine_id", Cell::Int(f.id)),
        kv("level_label", text(f.level_label.clone())),
        kv("mz", num(f.mz)),
        kv("rt", num(f.rt)),
        kv("std_name", s.map_or(Cell::Null, |x| Cell::str(&x.name))),
        kv("std_status", s.map_or(Cell::Null, |x| text(x.status.clone()))),
        kv("std_ppm", num(s.map(|x| x.ppm))),
        kv("std_drt", num(s.map(|x| x.drt))),
        kv("std_ms2_cosine", num(s.and_then(|x| x.cosine))),
        kv("std_ms2_reverse", num(s.and_then(|x| x.reverse))),
        kv("std_ms2_matched", num(s.and_then(|x| x.matched).map(|m| m as f64))),
        kv("spec_name", Cell::str(&f.spec.name)),
        kv("spec_sim", num(f.spec.sim)),
        kv("spec_peaks", num(f.spec.peaks)),
        kv("gnps_name", Cell::str(&f.gnps.name)),
        kv("gnps_score", num(f.gnps.score)),
        kv("gnps_shared_peaks", num(gp.shared_peaks)),
        kv("gnps_mz_error_ppm", num(gp.mz_error_ppm)),
        kv("gnps_mass_diff", num(gp.mass_diff)),
        kv("gnps_library", Cell::str(&gp.library)),
        kv("mzlib_name", Cell::str(&f.mzlib.name)),
        kv("mzlib_score", num(f.mzlib.score)),
        kv("mzlib_method", Cell::str(&r.mz_method)),
        kv("mzlib_adduct", Cell::str(&f.mzlib.adduct)),
        kv("mzlib_formula", Cell::str(&f.mzlib.formula)),
        kv("name", Cell::str(&f.csi.name)),
        kv("cosmic", num(f.csi.cosmic)),
        kv("matched_peaks", num(r.matched_peaks())),
        kv("mz_error_ppm", num(gp.mz_error_ppm)),
        kv("match_inchikey", r.match_inchikey().map_or(Cell::Null, |x| text(x.into()))),
        kv("match_smiles", r.match_smiles().map_or(Cell::Null, |x| text(x.into()))),
        kv("ref_usi", r.ref_usi().map_or(Cell::Null, |x| text(x.into()))),
        kv("gnps_component", f.gnps_component.map_or(Cell::Null, Cell::Int)),
    ];
    for (i, s) in l.sites.iter().enumerate() {
        v.push((format!("area_{s}"), Cell::Num(r.area[i])));
    }
    v.push(kv("mzMine_file", text(files.get(&f.pol).cloned().unwrap_or_default())));
    v
}

fn excluded_cells(ident: &str, group: &str, all: &[&FRow]) -> Pairs {
    let sub: Vec<&FRow> = all.iter().filter(|r| r.identity.as_deref() == Some(ident)).copied().collect();
    let reason = if group == "lab_artifact" {
        "internal standard / analytical artifact"
    } else {
        "uninterpretable library code or composite match"
    };
    vec![
        kv("identity", text(ident.into())),
        kv("group", text(group.into())),
        kv("reason_excluded", text(reason.into())),
        kv("n_features_collapsed", Cell::Int(sub.len() as i64)),
        kv("feature_locations (pol:mzMine_id)", text(locations(&sub))),
    ]
}

/// The three tables of one level: identities, features, excluded identities (each as rows of pairs).
fn level_blocks(
    l: &Loaded,
    rows: &[FRow],
    ids: &[IdentityRow],
    level: i32,
    short: &[String],
    files: &Files,
) -> (Vec<Pairs>, Vec<Pairs>, Vec<Pairs>) {
    let at: Vec<&IdentityRow> = ids.iter().filter(|i| i.best_level == level).collect();
    let dropped = |g: &str| g == "ambiguous" || g == "lab_artifact";
    let included: BTreeMap<&str, &IdentityRow> =
        at.iter().filter(|i| !dropped(&i.group)).map(|i| (i.identity.as_str(), *i)).collect();
    let mut by: BTreeMap<&str, Vec<&FRow>> = BTreeMap::new();
    let mut feat_lvl: Vec<&FRow> = Vec::new();
    for r in rows.iter().filter(|r| r.f.level == level) {
        if let Some(i) = r.identity.as_deref().filter(|i| included.contains_key(i)) {
            by.entry(i).or_default().push(r);
            feat_lvl.push(r);
        }
    }
    let mut ident_rows: Vec<Pairs> = by
        .iter()
        .map(|(i, sub)| identity_row(level, included[i], sub, short, files))
        .collect();
    sort_rows(&mut ident_rows);
    feat_lvl.sort_by(|a, b| {
        (a.identity.as_deref(), a.f.pol.as_str(), a.f.id).cmp(&(b.identity.as_deref(), b.f.pol.as_str(), b.f.id))
    });
    let feats = feat_lvl.iter().map(|r| feature_cells(l, r, files)).collect();
    let everyone: Vec<&FRow> = rows.iter().collect();
    let mut gone: Vec<&&IdentityRow> = at.iter().filter(|i| dropped(&i.group)).collect();
    gone.sort_by(|a, b| a.identity.cmp(&b.identity));
    let exc = gone.iter().map(|g| excluded_cells(&g.identity, &g.group, &everyone)).collect();
    (ident_rows, feats, exc)
}

fn short_names(c: &Campaign, sites: &[String]) -> Vec<String> {
    sites.iter().map(|s| site_short(c, s)).collect()
}

/// The `short` of a site (its key when the campaign gives none).
pub fn site_short(c: &Campaign, site: &str) -> String {
    c.sites
        .iter()
        .find(|(k, _)| k == site)
        .and_then(|(_, t)| t.get("short").and_then(|v| v.as_str()))
        .unwrap_or(site)
        .to_string()
}

fn merged_files(c: &Campaign, io: &Io) -> Files {
    ["POS", "NEG"]
        .iter()
        .filter_map(|p| {
            merged_table_path(c, io, p)
                .ok()
                .map(|path| (p.to_string(), file_name(&path).to_string()))
        })
        .collect()
}

/// `attach_lvl1_quant`: the estimated environmental concentrations of the Level 1 identities (campaigns with a calibration).
fn attach_quant(c: &Campaign, io: &Io, l: &Loaded, rows: &[FRow], block: &mut [Pairs], short: &[String]) -> Result<()> {
    if block.is_empty() || c.calibration_csv.is_none() {
        return Ok(());
    }
    let cal = load_calibration(c, io, &mut Vec::new())?;
    let (q, _) = quantify_lvl1_environmental(c, &l.src, &cal, l.features(), Some(&l.sites))?;
    if q.is_empty() || q[0].env.is_none() {
        return Ok(());
    }
    let mut q: Vec<&QuantRow> = q.iter().collect();
    q.sort_by(|a, b| a.area.partial_cmp(&b.area).unwrap_or(std::cmp::Ordering::Equal));
    let keep: Vec<&QuantRow> = (0..q.len())
        .filter(|&i| !q[i + 1..].iter().any(|n| n.std_name == q[i].std_name && n.site == q[i].site))
        .map(|i| q[i])
        .collect();
    let mut to_ident: HashMap<&str, &str> = HashMap::new();
    for r in rows.iter().filter(|r| r.f.level == 1) {
        if let (Some(n), Some(i)) = (r.f.std.as_ref().and_then(|s| s.name.as_deref()), r.identity.as_deref()) {
            to_ident.insert(n, i);
        }
    }
    let ident_of = |x: &QuantRow| to_ident.get(x.std_name.as_str()).copied();
    let ef = keep[0].env.as_ref().map_or(f64::NAN, |e| e.ef_spe);
    for row in block.iter_mut() {
        let Some(Cell::Str(me)) = cell_key(row, "identity") else { continue };
        for (s, sh) in l.sites.iter().zip(short) {
            let hit = keep.iter().find(|x| x.site == *s && ident_of(x) == Some(me.as_str()));
            let conc = hit.and_then(|x| x.env.as_ref()).map(|e| py_round(e.conc_env_ng_l, 2));
            row.push((format!("conc_env_ngL_{sh}"), num(conc)));
            row.push((format!("xLOQ_{sh}"), num(hit.and_then(|x| x.loq_ratio).map(|v| py_round(v, 3)))));
        }
        let first = keep.iter().find(|x| ident_of(x) == Some(me.as_str()));
        row.push(kv("calib_R2", num(first.map(|x| py_round(x.r2, 3)))));
        row.push(kv(
            "calib_R2_ok",
            first.map_or(Cell::Null, |x| text(if x.r2_ok { "" } else { "R2 below threshold" }.into())),
        ));
        row.push(kv(
            "quant_caveat",
            text(format!(
                "environmental estimate: extract / EF {}x SPE, NO recovery correction (true concentration likely higher); xLOQ < 1 = below the lowest \
                 calibration point, order of magnitude only",
                mzlab_core::json::fmt_g(ef)
            )),
        ));
    }
    Ok(())
}

/// `_notes`: the legend of the columns and the caveats.
pub fn notes(c: &Campaign, sites: &[String], has_quant: bool) -> Sheet {
    let std = |k: &str| toml_text(c.standards.get(k));
    let or = |k: &str, d: &str| c.standards.get(k).map_or(d.to_string(), |v| toml_text(Some(v)));
    let samples = sites
        .iter()
        .map(|s| format!("{s} ({})", site_short(c, s)))
        .collect::<Vec<_>>()
        .join(", ");
    let mut rows: Vec<(String, String)> = vec![
        ("File".into(), "All three levels in one table; filter on 'schymanski_level'. One compound = ONE row, at its best level (L1 > L2a > L3).".into()),
        ("Dataset".into(), format!("{}: {}, {}, DDA POS+NEG, sorbents {}.", c.name, c.instrument, c.fragmentation, c.sorbents.join(" + "))),
        ("Samples".into(), format!("{samples}. A sample is a site; the sorbents are two extractions of the same water (areas summed).")),
        ("Count".into(), "by_identity = identities from identity_table() (the count of the paper). features = one row per feature/ion.".into()),
        ("L1".into(), format!("Reference standard: mass <= {} ppm, RT within {} min of the standard on this instrument, and MS/MS cosine ≥ {} on ≥ {} fragments against the standard's own spectrum of the same ion (or confirmed by the chemist in level1_decisions.csv). confidence_score = that MS/MS cosine.", std("mass_tol_ppm"), std("rt_tol_min"), or("ms2_min_cosine", "0.7"), or("ms2_min_matched", "2"))),
    ];
    rows.extend(NOTES_FIXED.iter().map(|(a, b)| (a.to_string(), b.to_string())));
    if has_quant {
        let ef = c.quantification.get("spe_enrichment_factor").and_then(|v| v.as_f64());
        let ef = ef.map_or("None".into(), mzlab_core::json::fmt_g);
        rows.push(("conc_env_ngL_<sample> (L1)".into(), format!("Estimated concentration in water: extract concentration from the external calibration (recommended adduct) / EF {ef}x * 1000. Shown even when extrapolated.")));
        rows.push(("quantification caveat".into(), "No correction for SPE recovery (not validated): the true concentration is likely higher. xLOQ_<sample> = sample area / area of the lowest calibration point; < 1 = below it (order of magnitude only). calib_R2 below threshold = unreliable line.".into()));
    }
    let mut s = Sheet::new("notes", &["Field", "Meaning"]);
    s.rows = rows.into_iter().map(|(a, b)| vec![Cell::Str(a), Cell::Str(b)]).collect();
    s
}

const NOTES_FIXED: [(&str, &str); 10] = [
    ("L2a", "Spectral-library match: cosine ≥ 0.70 with ≥ 4 shared fragment ions and the library precursor within 15 ppm (otherwise an analogue). confidence_score = best cosine."),
    ("L3", "CSI:FingerID structure with COSMIC (ConfidenceScoreApproximate) ≥ 0.64, ~10% FDR (Hoffmann et al., Nat. Biotechnol. 2022) -- confidence_score = COSMIC; or a reference standard matching mass + RT whose MS/MS could not confirm it (see why_level)."),
    ("match_type", "direct = library hit on the SAME precursor (|ppm| <= 20) -> true L2a; analog = similar spectrum of a DIFFERENT mass (GNPS analogue search): not the same molecule; mzMine (no ppm) = mzMine hit, no error reported."),
    ("putative_class / analog_hypothesis", "Analogue hits only: reading of the mass difference (homologue, transformation, artifact, near-isobaric, unassigned). Always putative."),
    ("matched_peaks / mz_error_ppm", "Shared MS2 peaks and precursor error of the best library hit (GNPS2; SIRIUS for the peaks). mzMine does not report them."),
    ("rt_logp_z / rt_logp_check (L3)", "RT vs logP consistency relative to the other L3 candidates of the batch (isotonic fit, robust z). |z| > 3.5 = outlier: a clue, not a proof, of a wrong structure."),
    ("area_sum_<sample>", "Sum of the areas of all collapsed features (all sorbents), DETECTED only. Semi-quantitative proxy."),
    ("category 'Other / non-target'", "Non-contaminants: a library name does not prove a natural origin (the key 'natural' is a name heuristic)."),
    ("excluded", "Identities left out of the count: library codes and internal standards / carry-over."),
    ("n = 1 per sample", "Exploratory: no statistical tests, no p-values."),
];

/// The sheets of the Schymanski workbook: `by_identity`, `features`, `excluded`, `notes`.
pub fn build(c: &Campaign, io: &Io, matrix: Option<&str>) -> Result<Vec<Sheet>> {
    let l = Loaded::load(c, io, matrix)?;
    let rows = feature_rows(c, io, &l)?;
    let ids = l.all_identities();
    let short = short_names(c, &l.sites);
    let files = merged_files(c, io);
    let (mut by_id, mut feats, mut exc) = (Wide::default(), Wide::default(), Wide::default());
    for level in 1..=3 {
        let (mut a, b, e) = level_blocks(&l, &rows, &ids, level, &short, &files);
        if level == 1 {
            attach_quant(c, io, &l, &rows, &mut a, &short)?;
        }
        let tag = LEVEL_TAG[level as usize - 1];
        let with_tag = |mut p: Pairs| {
            p.insert(0, kv("schymanski_level", text(tag.into())));
            p
        };
        a.into_iter().for_each(|p| by_id.push(with_tag(p)));
        b.into_iter().for_each(|p| feats.push(with_tag(p)));
        e.into_iter().for_each(|p| exc.push(with_tag(p)));
    }
    Ok(vec![
        by_id.sheet("by_identity", &FRONT, "(no identities)"),
        feats.sheet("features", &[], "(no features)"),
        exc.sheet("excluded", &[], "(no excluded identities)"),
        notes(c, &l.sites, c.calibration_csv.is_some()),
    ])
}
