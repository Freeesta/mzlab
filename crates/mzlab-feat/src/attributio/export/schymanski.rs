// Ported from Attributio diagnostics/schymanski_export.py.
//! The Schymanski workbook: one sheet row per identity of Levels 1-3 with where it comes from and why it has its level, one per feature, the identities left
//! out of the count, and the legend. The sheets are cells (`Sheet`); the page makes the XLSX.

use super::cells::Sheet;
use super::ctx::{feature_rows, FRow, Loaded};
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
use super::schymanski_delta::*;
use super::schymanski_notes::notes;
use super::schymanski_rows::*;

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
        kv(
            "std_status",
            s.map_or(Cell::Null, |x| text(x.status.clone())),
        ),
        kv("std_ppm", num(s.map(|x| x.ppm))),
        kv("std_drt", num(s.map(|x| x.drt))),
        kv("std_ms2_cosine", num(s.and_then(|x| x.cosine))),
        kv("std_ms2_reverse", num(s.and_then(|x| x.reverse))),
        kv(
            "std_ms2_matched",
            num(s.and_then(|x| x.matched).map(|m| m as f64)),
        ),
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
        kv(
            "match_inchikey",
            r.match_inchikey().map_or(Cell::Null, |x| text(x.into())),
        ),
        kv(
            "match_smiles",
            r.match_smiles().map_or(Cell::Null, |x| text(x.into())),
        ),
        kv(
            "ref_usi",
            r.ref_usi().map_or(Cell::Null, |x| text(x.into())),
        ),
        kv(
            "gnps_component",
            f.gnps_component.map_or(Cell::Null, Cell::Int),
        ),
    ];
    for (i, s) in l.sites.iter().enumerate() {
        v.push((format!("area_{s}"), Cell::Num(r.area[i])));
    }
    v.push(kv(
        "mzMine_file",
        text(files.get(&f.pol).cloned().unwrap_or_default()),
    ));
    v
}

fn excluded_cells(ident: &str, group: &str, all: &[&FRow]) -> Pairs {
    let sub: Vec<&FRow> = all
        .iter()
        .filter(|r| r.identity.as_deref() == Some(ident))
        .copied()
        .collect();
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
    let included: BTreeMap<&str, &IdentityRow> = at
        .iter()
        .filter(|i| !dropped(&i.group))
        .map(|i| (i.identity.as_str(), *i))
        .collect();
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
        (a.identity.as_deref(), a.f.pol.as_str(), a.f.id).cmp(&(
            b.identity.as_deref(),
            b.f.pol.as_str(),
            b.f.id,
        ))
    });
    let feats = feat_lvl
        .iter()
        .map(|r| feature_cells(l, r, files))
        .collect();
    let everyone: Vec<&FRow> = rows.iter().collect();
    let mut gone: Vec<&&IdentityRow> = at.iter().filter(|i| dropped(&i.group)).collect();
    gone.sort_by(|a, b| a.identity.cmp(&b.identity));
    let exc = gone
        .iter()
        .map(|g| excluded_cells(&g.identity, &g.group, &everyone))
        .collect();
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
fn attach_quant(
    c: &Campaign,
    io: &Io,
    l: &Loaded,
    rows: &[FRow],
    block: &mut [Pairs],
    short: &[String],
) -> Result<()> {
    if block.is_empty() || c.calibration_csv.is_none() {
        return Ok(());
    }
    let cal = load_calibration(c, io, &mut Vec::new())?;
    let (q, _) = quantify_lvl1_environmental(c, &l.src, &cal, l.features(), Some(&l.sites))?;
    if q.is_empty() || q[0].env.is_none() {
        return Ok(());
    }
    let mut q: Vec<&QuantRow> = q.iter().collect();
    q.sort_by(|a, b| {
        a.area
            .partial_cmp(&b.area)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let keep: Vec<&QuantRow> = (0..q.len())
        .filter(|&i| {
            !q[i + 1..]
                .iter()
                .any(|n| n.std_name == q[i].std_name && n.site == q[i].site)
        })
        .map(|i| q[i])
        .collect();
    let mut to_ident: HashMap<&str, &str> = HashMap::new();
    for r in rows.iter().filter(|r| r.f.level == 1) {
        if let (Some(n), Some(i)) = (
            r.f.std.as_ref().and_then(|s| s.name.as_deref()),
            r.identity.as_deref(),
        ) {
            to_ident.insert(n, i);
        }
    }
    let ident_of = |x: &QuantRow| to_ident.get(x.std_name.as_str()).copied();
    let ef = keep[0].env.as_ref().map_or(f64::NAN, |e| e.ef_spe);
    for row in block.iter_mut() {
        let Some(Cell::Str(me)) = cell_key(row, "identity") else {
            continue;
        };
        for (s, sh) in l.sites.iter().zip(short) {
            let hit = keep
                .iter()
                .find(|x| x.site == *s && ident_of(x) == Some(me.as_str()));
            let conc = hit
                .and_then(|x| x.env.as_ref())
                .map(|e| py_round(e.conc_env_ng_l, 2));
            row.push((format!("conc_env_ngL_{sh}"), num(conc)));
            row.push((
                format!("xLOQ_{sh}"),
                num(hit.and_then(|x| x.loq_ratio).map(|v| py_round(v, 3))),
            ));
        }
        let first = keep.iter().find(|x| ident_of(x) == Some(me.as_str()));
        row.push(kv("calib_R2", num(first.map(|x| py_round(x.r2, 3)))));
        row.push(kv(
            "calib_R2_ok",
            first.map_or(Cell::Null, |x| {
                text(if x.r2_ok { "" } else { "R2 below threshold" }.into())
            }),
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
