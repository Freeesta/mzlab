//! `features-report/1` (`features --report PATH`: what the unit-resolution steps did by themselves) and `features-plan/1` (`features --dry-run`: what they
//! would do, from the indexes alone). The decisions are the engine's (`mzlab_feat::lrscreen`, `lr_note`); this module only says them. stdout (the CSV) is not touched.

use std::path::PathBuf;

use mzlab_core::error::Result;
use mzlab_core::flow::value_text;
use mzlab_core::json::{Out, Value};
use mzlab_feat::axis::MassAxis;
use mzlab_feat::lrscreen::{centroid_wanted, label_reason, LabelCheck, Log};
use mzlab_feat::recipe::Recipe;
use mzlab_ops::run::feat::lr_note::{self, note_of, Note};

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
            let mut index = crate::open(idx).ok()?;
            let name = match labels.get(k).filter(|l| !l.is_empty()) {
                Some(l) => l.clone(),
                None => files[k].display().to_string(),
            };
            Some(Member {
                centroid: centroid_wanted(&mut index, recipe),
                note: note_of(recipe, &mut index),
                name,
            })
        })
        .collect()
}

/// The label check behind a member's notice (its numbers), when the file is labelled profile but holds sparse ion counts.
fn label_of(x: &Member) -> Option<LabelCheck> {
    let n = x
        .note
        .notices
        .iter()
        .find(|n| n.key == lr_note::LABEL_MISMATCH)?;
    let p = |k: &str| n.params.iter().find(|(q, _)| *q == k).map(|(_, v)| *v);
    Some(LabelCheck {
        profile: p("profile")?,
        scans: p("scans")?,
        probed: p("probed")?,
        points: p("points")?,
        spacings: p("spacings")?,
        runs_pct: p("runs")?,
        fine_pct: p("fine")?,
        sparse: true,
    })
}

/// Why the centroid switch is what it is for a member: the numbers of the label check, else the share of profile scans.
fn centroid_why(x: &Member) -> String {
    match label_of(x) {
        Some(c) => format!("{}: {}", x.name, label_reason(&c)),
        None => format!(
            "{}: {} of {} MS1 scans in profile mode",
            x.name, x.note.profile, x.note.scans
        ),
    }
}

fn num(x: f64) -> String {
    value_text(&Value::Num(x))
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

/// The `choices` array of the report: the axis, the centroid switch of each unit-resolution file and what the screening logged by itself.
fn choices_json(o: &mut Out, recipe: &Recipe, m: &[Member], log: &Log) {
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
        choice(o, "axis", if x.note.unit { "Da" } else { "ppm" }, &why);
    }
    for x in m.iter().filter(|x| x.note.unit) {
        let why = centroid_why(x);
        choice(o, "centroid", if x.centroid { "on" } else { "off" }, &why);
    }
    // the label-check choice of a file is already said with that file's own numbers above: dropped per file, by its exact reason
    let said: Vec<String> = m
        .iter()
        .filter_map(label_of)
        .map(|c| label_reason(&c))
        .collect();
    for c in log
        .choices
        .iter()
        .filter(|c| !(c.key == "centroid" && said.contains(&c.reason)))
    {
        choice(o, &c.key, &c.value, &c.reason);
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
    let mut o = Out::default();
    o.raw("{\"schema\":\"features-report/1\",\"recipe\":")
        .str(&recipe_line(recipe, m.first().map(|x| &x.note)));
    files_json(&mut o, recipe, &m, &log, rows);
    o.raw("],\"merged\":[");
    for (k, g) in log.merged.iter().enumerate() {
        o.raw(if k > 0 { "," } else { "" }).raw("{\"kept\":");
        row(&mut o, g.kept);
        o.raw(",\"dropped\":");
        row(&mut o, g.dropped);
        o.raw(&format!(
            ",\"dmz_da\":{},\"drt_min\":{},\"r\":{}}}",
            num(g.dmz_da),
            num(g.drt_min),
            num(g.r)
        ));
    }
    choices_json(&mut o, recipe, &m, &log);
    o.raw("]}\n");
    o.0
}
