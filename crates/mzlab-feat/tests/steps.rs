//! The `step/1` descriptors of the flagship chain: their defaults are the ones the code uses today (the recipe built from them has the same
//! fingerprints as `Recipe::default()`), and `docs/schemi/steps-1.json` is their export
//! (`MZLAB_UPDATE_STEPS=1 cargo test -p mzlab-feat --test steps` rewrites it; `tests/test_step_descrittori.py` validates it against the schema).

use mzlab_core::json::Value;
use mzlab_core::step::json::registry_json;
use mzlab_core::step::{find, Kind, Param, Step};
use mzlab_feat::axis::MassAxis;
use mzlab_feat::recipe::{Mode, Recipe};
use mzlab_feat::steps::{recipe_map, registry};

const FEATURE_STEPS: [&str; 4] = [
    "features.detect",
    "features.align",
    "features.gapfill",
    "features.ms2_link",
];

const IDS: [&str; 12] = [
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
    "export.inclusion",
    "export.mztab",
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

/// The defaults of `s` (its own parameters and those of its default algorithm), as a plan would resolve them.
fn defaults(s: &Step) -> Vec<(String, Value)> {
    let alg = s
        .algorithms
        .iter()
        .find(|a| Some(a.id) == s.default_algorithm);
    let own = s
        .params
        .iter()
        .chain(alg.into_iter().flat_map(|a| a.params.iter()));
    own.filter_map(|p| p.default.clone().map(|d| (p.name.to_string(), d)))
        .collect()
}

/// The defaults of `s` written onto `r` with the mapping of the native runner.
fn apply_defaults(r: &mut Recipe, s: &Step) {
    recipe_map::apply(r, s.id, &defaults(s)).unwrap();
}

#[test]
fn registry_lists_the_twelve_steps_of_the_chain() {
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
    for id in FEATURE_STEPS {
        apply_defaults(&mut r, step(id));
    }
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

/// `values` reads back what `apply` wrote: the defaults of every feature step, through the recipe.
#[test]
fn recipe_map_reads_back_what_it_writes() {
    let reg = registry();
    for id in FEATURE_STEPS {
        let s = find(&reg, id).unwrap();
        let mut r = perturbed();
        apply_defaults(&mut r, s);
        let mut want = defaults(s);
        mzlab_core::sortc::slice_by(&mut want, |a, b| a.0.cmp(&b.0));
        let got: Vec<(String, Value)> = recipe_map::values(&r, id)
            .into_iter()
            .filter(|(k, _)| want.iter().any(|w| w.0 == *k))
            .collect();
        assert_eq!(got, want, "{id}");
    }
    let mut r = Recipe::default();
    let unknown = recipe_map::apply(
        &mut r,
        "features.align",
        &[("nope".into(), Value::Num(1.0))],
    );
    assert_eq!(unknown.unwrap_err().key, "err.flow.param.unknown");
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
