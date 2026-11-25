//! `features-report/1` (`features --report PATH`: what the unit-resolution steps did by themselves) and `features-plan/1` (`features --dry-run`: what they
//! would do, from the indexes alone). The decisions are the engine's (`mzlab_feat::lrscreen`, `lr_note`); this module only says them. stdout (the CSV) is not touched.

use std::path::PathBuf;

use mzlab_core::error::Result;
use mzlab_core::flow::value_text;
use mzlab_core::json::{Out, Value};
use mzlab_feat::axis::MassAxis;
use mzlab_feat::lrscreen::{centroid_wanted, Log};
use mzlab_feat::recipe::Recipe;
use mzlab_ops::run::feat::lr_note::{note_of, Note};

use crate::convert::Notice;

/// One member of the series: how it is called, what the index says and whether its profile scans are centroided.
struct Member {
    name: String,
    note: Note,
    centroid: bool,
}

fn members(recipe: &Recipe, idxs: &[PathBuf], files: &[PathBuf], labels: &[String]) -> Vec<Member> {
    idxs.iter()
        .enumerate()
        .filter_map(|(k, idx)| {
            let index = crate::open(idx).ok()?;
            let name = match labels.get(k).filter(|l| !l.is_empty()) {
                Some(l) => l.clone(),
                None => files[k].display().to_string(),
            };
            Some(Member {
                centroid: centroid_wanted(&index, recipe),
                note: note_of(recipe, &index),
                name,
            })
        })
        .collect()
}

fn num(x: f64) -> String {
    value_text(&Value::Num(x))
}

/// The note that rows are still close after the duplicate merge, with the first pair's numbers: the hint that `dedup_overlap` exists.
pub(crate) fn near_notice(log: &Log) -> Option<Notice> {
    let n = log.near.first()?;
    Some(Notice {
        key: "note.features.nearDup",
        params: vec![
            ("n", log.near.len().to_string()),
            ("mz_a", format!("{:.3}", n.a.0)),
            ("mz_b", format!("{:.3}", n.b.0)),
            ("rt_a", format!("{:.2}", n.a.1)),
            ("rt_b", format!("{:.2}", n.b.1)),
        ],
    })
}

/// A number that may be missing (NaN): `null` then.
fn opt(x: f64) -> String {
    if x.is_finite() {
        num(x)
    } else {
        "null".into()
    }
}

fn share(n: &Note) -> f64 {
    if n.scans == 0 {
        0.0
    } else {
        n.profile as f64 / n.scans as f64
    }
}

/// The recipe of the series in one line: the axis with the numbers of the first file and, on the unit axis, the three switches.
fn recipe_line(recipe: &Recipe, first: Option<&Note>) -> String {
    let line = recipe_base(recipe, first);
    match recipe.screen.mz_offset {
        s if s.is_zero() => line,
        s => format!("{line}, mz_offset={s}"),
    }
}

fn recipe_base(recipe: &Recipe, first: Option<&Note>) -> String {
    let (floor, min_height) = first.map_or((recipe.detect.floor, recipe.detect.min_height), |n| {
        (n.floor, n.min_height)
    });
    match recipe.detect.axis {
        MassAxis::Da { defect } => format!(
            "unit resolution (1 Da channels centred on n{defect:+.3}), floor {} counts, minimum height {} counts, centroid={}, recenter={}, dedup={}",
            num(floor),
            num(min_height),
            recipe.screen.centroid.name(),
            recipe.screen.recenter.name(),
            recipe.screen.dedup.name()
        ),
        MassAxis::Ppm => format!(
            "high resolution (ppm axis), floor {} counts, minimum height {} counts",
            num(floor),
            num(min_height)
        ),
    }
}

fn defect_of(recipe: &Recipe) -> f64 {
    match recipe.detect.axis {
        MassAxis::Da { defect } => defect,
        MassAxis::Ppm => 0.0,
    }
}

/// `features-plan/1`: the decision of each member from its index; nothing is detected, except that an `auto:` m/z offset is measured on the index
/// (its evidence is in the plan, and its lines go to stderr as in a run).
pub(crate) fn plan_text(
    recipe: &Recipe,
    idxs: &[PathBuf],
    files: &[PathBuf],
    labels: &[String],
) -> Result<String> {
    let m = members(recipe, idxs, files, labels);
    let mut o = Out::default();
    o.raw("{\"schema\":\"features-plan/1\",\"recipe\":")
        .str(&recipe_line(recipe, m.first().map(|x| &x.note)));
    o.raw(",\"members\":[");
    for (k, x) in m.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" })
            .raw("{\"member\":")
            .str(&x.name);
        o.raw(&format!(
            ",\"unit\":{},\"profile_fraction\":{},\"centroid\":{}",
            x.note.unit,
            num(share(&x.note)),
            x.centroid
        ));
        let spec = recipe.screen.mz_offset;
        if !spec.is_zero() {
            let f = spec.resolve(
                &mut crate::idx_open::open(&idxs[k])?,
                recipe.detect.polarity,
            )?;
            eprint!("{}", crate::mz_offset::lines(&x.name, &f));
            o.raw(",").raw(&f.members());
        }
        o.raw("}");
    }
    o.raw("]}\n");
    Ok(o.0)
}

fn row(o: &mut Out, r: (f64, f64)) {
    o.raw(&format!("{{\"mz\":{},\"rt\":{}}}", num(r.0), num(r.1)));
}

/// The `files` array of the report: one entry per file with what the screening did to it.
fn files_json(o: &mut Out, recipe: &Recipe, m: &[Member], log: &Log, rows: &[usize]) {
    o.raw(",\"files\":[");
    for (k, x) in m.iter().enumerate() {
        let s = log.shifts.get(k);
        o.raw(if k > 0 { "," } else { "" })
            .raw("{\"member\":")
            .str(&x.name);
        o.raw(&format!(
            ",\"unit\":{},\"profile_fraction\":{},\"centroided\":{},\"defect\":{},\"mz_shift_da\":{},\"recentred\":{},\"rows\":{}{}}}",
            x.note.unit,
            num(share(&x.note)),
            x.centroid,
            num(defect_of(recipe)),
            num(s.map_or(0.0, |s| s.mz_da)),
            s.is_some_and(|s| s.applied),
            rows.get(k).copied().unwrap_or(0),
            // only with `--mz-offset`, so the report of a run without it keeps its bytes
            log.offsets
                .get(k)
                .map_or(String::new(), |f| format!(",{}", f.members()))
        ));
    }
}

/// The `merged` array of the report: every row `dedup` dropped, with the rule, its threshold and both rows' numbers.
fn merged_json(o: &mut Out, log: &Log) {
    for (k, g) in log.merged.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" }).raw("{\"kept\":");
        row(o, g.kept);
        o.raw(",\"dropped\":");
        row(o, g.dropped);
        o.raw(&format!(
            ",\"dmz_da\":{},\"drt_min\":{},\"r\":{},\"rule\":\"{}\",\"overlap\":{},\"shared_files\":{},\"threshold\":{},\"height_kept\":{},\"height_dropped\":{}}}",
            num(g.dmz_da),
            num(g.drt_min),
            opt(g.r),
            g.rule,
            opt(g.overlap),
            g.shared,
            num(g.threshold),
            num(g.height.0),
            num(g.height.1)
        ));
    }
}

/// The `near` array: the pairs still within the dedup window that neither rule merged (the hint that `dedup_overlap` exists).
fn near_json(o: &mut Out, log: &Log) {
    for (k, g) in log.near.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" }).raw("{\"a\":");
        row(o, g.a);
        o.raw(",\"b\":");
        row(o, g.b);
        o.raw(&format!(
            ",\"dmz_da\":{},\"drt_min\":{},\"r\":{},\"overlap\":{},\"shared_files\":{}}}",
            num(g.dmz_da),
            num(g.drt_min),
            opt(g.r),
            opt(g.overlap),
            g.shared
        ));
    }
}

/// `features-report/1`: `rows` = features detected in each member; the evidence of the recipe holds the shifts, the merged rows and the choices.
pub(crate) fn report_text(
    recipe: &Recipe,
    idxs: &[PathBuf],
    files: &[PathBuf],
    labels: &[String],
    rows: &[usize],
) -> String {
    let m = members(recipe, idxs, files, labels);
    let log: Log = recipe
        .screen
        .evidence
        .as_ref()
        .map(|e| e.take())
        .unwrap_or_default();
    for (x, f) in m.iter().zip(&log.offsets) {
        eprint!("{}", crate::mz_offset::lines(&x.name, f));
    }
    if let Some(n) = near_notice(&log) {
        eprintln!("note: {}", crate::error_text::warning(&n));
    }
    let mut o = Out::default();
    o.raw("{\"schema\":\"features-report/1\",\"recipe\":")
        .str(&recipe_line(recipe, m.first().map(|x| &x.note)));
    files_json(&mut o, recipe, &m, &log, rows);
    o.raw("],\"merged\":[");
    merged_json(&mut o, &log);
    o.raw("],\"near\":[");
    near_json(&mut o, &log);
    o.raw("],\"choices\":[");
    let mut first = true;
    let mut choice = |o: &mut Out, key: &str, value: &str, reason: &str| {
        o.raw(if first { "" } else { "," })
            .raw("{\"key\":")
            .str(key)
            .raw(",\"value\":")
            .str(value);
        o.raw(",\"reason\":").str(reason).raw("}");
        first = false;
    };
    if let Some(x) = m.first() {
        let why = if x.note.unit {
            format!(
                "{} is unit resolution: 1 Da channels centred on n{:+.3}",
                x.name,
                defect_of(recipe)
            )
        } else {
            format!("{} is high resolution", x.name)
        };
        choice(&mut o, "axis", if x.note.unit { "Da" } else { "ppm" }, &why);
    }
    for x in m.iter().filter(|x| x.note.unit) {
        let why = format!(
            "{}: {} of {} MS1 scans in profile mode",
            x.name, x.note.profile, x.note.scans
        );
        choice(
            &mut o,
            "centroid",
            if x.centroid { "on" } else { "off" },
            &why,
        );
    }
    for c in &log.choices {
        choice(&mut o, &c.key, &c.value, &c.reason);
    }
    o.raw("]}\n");
    o.0
}
