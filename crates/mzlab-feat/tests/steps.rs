//! The `step/1` descriptors of the flagship chain: their defaults are the ones the code uses today (the recipe built from them has the same
//! fingerprints as `Recipe::default()`), and `docs/schemi/steps-1.json` is their export
//! (`MZLAB_UPDATE_STEPS=1 cargo test -p mzlab-feat --test steps` rewrites it; `tests/test_step_descrittori.py` validates it against the schema).

use mzlab_core::json::Value;
use mzlab_core::step::json::registry_json;
use mzlab_core::step::{find, Kind, Param, Step};
use mzlab_feat::axis::MassAxis;
use mzlab_feat::recipe::{Mode, Recipe};
use mzlab_feat::steps::registry;

const IDS: [&str; 10] = [
    "import.raw",
    "convert.mzml",
    "index.mzidx",
    "features.estimate",
    "features.detect",
    "features.align",
    "features.gapfill",
    "table.filter",
    "features.ms2_link",
    "export.table_csv",
];

fn default_of(p: &Param) -> Value {
    p.default
        .clone()
        .unwrap_or_else(|| panic!("{} has no default", p.name))
}

fn num(p: &Param) -> f64 {
    default_of(p)
        .f64()
        .unwrap_or_else(|| panic!("{} is not a number", p.name))
}

fn axis_of(p: &Param) -> MassAxis {
    match default_of(p).str() {
        Some("ppm") => MassAxis::Ppm,
        other => panic!("axis default {other:?}"),
    }
}

/// A recipe with every field the descriptors name set to a value that is not the default.
fn perturbed() -> Recipe {
    let mut r = Recipe::default();
    r.detect.polarity = None;
    r.detect.floor *= 3.0;
    r.detect.min_height *= 3.0;
    r.detect.min_points += 1;
    r.detect.max_gap += 1;
    r.detect.long_trace += 1;
    r.detect.axis = MassAxis::Da { defect: 0.25 };
    r.mode = Mode::Ion;
    r.align.ion.ppm *= 3.0;
    r.align.ion.rt_gap *= 3.0;
    r.align.ion.max_ppm_span *= 3.0;
    r.align.ion.drop_artefacts = false;
    r.align.ion.split_gap = None;
    r.weak_files += 1;
    r.fill.floor *= 3.0;
    r.fill.ppm *= 3.0;
    r.fill.rt_tol *= 3.0;
    r.fill.min_points += 1;
    r.fill.max_gap += 1;
    r.fill.long_trace += 1;
    r.steps.ms2.default_half_window *= 3.0;
    r.steps.ms2.tol_ppm *= 3.0;
    r.steps.ms2.weak = 0.9;
    r
}

fn apply_detect(r: &mut Recipe, s: &Step) {
    let alg = |n: &str| s.algorithm_param("masstrace", n).unwrap();
    r.detect.polarity = match num(s.param("polarity").unwrap()) as i32 {
        0 => None,
        p => Some(p as i8),
    };
    r.detect.floor = num(s.param("floor").unwrap());
    r.detect.min_height = num(s.param("min_height").unwrap());
    r.detect.axis = axis_of(s.param("axis").unwrap());
    r.detect.min_points = num(alg("min_points")) as u32;
    r.detect.max_gap = num(alg("max_gap")) as u32;
    r.detect.long_trace = num(alg("long_trace")) as u32;
}

fn apply_align(r: &mut Recipe, s: &Step) {
    let p = |n: &str| s.param(n).unwrap();
    r.mode = match default_of(p("mode")).str() {
        Some("family") => Mode::Family,
        Some("ion") => Mode::Ion,
        other => panic!("mode default {other:?}"),
    };
    r.align.ion.ppm = num(p("ppm"));
    r.align.ion.rt_gap = num(p("rt_gap"));
    r.align.ion.max_ppm_span = num(p("max_ppm_span"));
    r.align.ion.drop_artefacts = default_of(p("drop_artefacts")) == Value::Bool(true);
    r.align.ion.split_gap = Some(num(p("split_gap")));
    r.weak_files = num(p("weak_files")) as usize;
}

fn apply_fill(r: &mut Recipe, s: &Step) {
    let p = |n: &str| num(s.param(n).unwrap());
    r.fill.floor = p("floor");
    r.fill.ppm = p("ppm");
    r.fill.rt_tol = p("rt_tol");
    r.fill.min_points = p("min_points") as usize;
    r.fill.max_gap = p("max_gap") as usize;
    r.fill.long_trace = p("long_trace") as usize;
}

fn apply_ms2(r: &mut Recipe, s: &Step) {
    let p = |n: &str| num(s.param(n).unwrap());
    r.steps.ms2.default_half_window = p("default_half_window");
    r.steps.ms2.tol_ppm = p("tol_ppm");
    r.steps.ms2.weak = p("weak") as f32;
}

#[test]
fn registry_lists_the_ten_steps_of_the_chain() {
    let reg = registry();
    let ids: Vec<&str> = reg.iter().map(|s| s.id).collect();
    assert_eq!(ids, IDS);
    assert!(reg.iter().all(|s| s.package == "core" && s.version == 1));
}

#[test]
fn defaults_rebuild_the_recipe_of_today() {
    let reg = registry();
    let step = |id: &str| find(&reg, id).unwrap();
    let mut r = perturbed();
    let (now, today) = (Recipe::default(), perturbed());
    assert_ne!(
        now.table_hash(),
        today.table_hash(),
        "the starting recipe must differ"
    );
    apply_detect(&mut r, step("features.detect"));
    apply_align(&mut r, step("features.align"));
    apply_fill(&mut r, step("features.gapfill"));
    apply_ms2(&mut r, step("features.ms2_link"));
    let p = |id: &str, n: &str| num(step(id).param(n).unwrap());
    assert_eq!(
        p("features.estimate", "polarity"),
        p("features.detect", "polarity")
    );
    assert_eq!(p("features.detect", "budget") as usize, now.detect.budget);
    assert_eq!(r.file_hash(), now.file_hash());
    assert_eq!(r.table_hash(), now.table_hash());
    assert_eq!(r.file_text(), now.file_text());
}

#[test]
fn every_default_is_inside_its_own_limits() {
    for s in registry() {
        let algs = s.algorithms.iter().flat_map(|a| a.params.iter());
        for p in s.params.iter().chain(algs) {
            let Some(d) = &p.default else { continue };
            if let (Some(x), Kind::Number | Kind::Integer) = (d.f64(), &p.kind) {
                assert!(
                    p.min.is_none_or(|m| x >= m),
                    "{} {} below its minimum",
                    s.id,
                    p.name
                );
                assert!(
                    p.max.is_none_or(|m| x <= m),
                    "{} {} above its maximum",
                    s.id,
                    p.name
                );
            }
            if let Kind::Choice(values) = &p.kind {
                assert!(
                    values.contains(d),
                    "{} {} default is not a choice",
                    s.id,
                    p.name
                );
            }
        }
    }
}

#[test]
fn exported_json_is_standard_json_and_is_the_committed_file() {
    let text = registry_json(&registry());
    let v: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
    assert_eq!(v.as_array().unwrap().len(), IDS.len());
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/schemi/steps-1.json");
    if std::env::var_os("MZLAB_UPDATE_STEPS").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let committed = std::fs::read_to_string(&path)
        .expect("docs/schemi/steps-1.json (MZLAB_UPDATE_STEPS=1 writes it)");
    assert_eq!(committed, text, "docs/schemi/steps-1.json is stale: MZLAB_UPDATE_STEPS=1 cargo test -p mzlab-feat --test steps");
}
