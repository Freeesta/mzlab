//! Descriptor guard for randomness: a step that draws pseudo-random numbers exposes its seed as a parameter (an integer from 0 with a default), so that a
//! record can list it under `seeds` and a replicate draws the same numbers. `deterministic: false` is reserved for the structure editor.

use mzlab_core::json::Value;
use mzlab_core::step::{Kind, Param, Step};

/// (step, algorithm or none, parameter) of every pseudo-random source of the registry.
const SEEDED: [(&str, Option<&str>, &str); 2] = [
    ("features.align", None, "rt_seed"),
    ("rt.predict", Some("forest"), "seed"),
];

fn all() -> Vec<Step> {
    let mut v = mzlab_feat::steps::registry();
    v.extend(mzlab_feat::steps::attributio());
    v.extend(mzlab_feat::steps::tabui());
    v
}

fn is_seed(p: &Param) -> bool {
    p.name == "seed" || p.name.ends_with("_seed")
}

fn check(p: &Param, who: &str) {
    assert!(matches!(p.kind, Kind::Integer), "{who}: a seed is an integer");
    assert_eq!(p.min, Some(0.0), "{who}: a seed starts at 0");
    assert!(matches!(p.default, Some(Value::Num(_))), "{who}: a seed has a default");
}

#[test]
fn every_random_source_declares_its_seed() {
    let steps = all();
    for (id, algo, name) in SEEDED {
        let s = steps.iter().find(|s| s.id == id).unwrap_or_else(|| panic!("{id} missing"));
        let params: &[Param] = match algo {
            None => &s.params,
            Some(a) => &s.algorithms.iter().find(|x| x.id == a).unwrap_or_else(|| panic!("{id}/{a} missing")).params,
        };
        let p = params.iter().find(|p| p.name == name).unwrap_or_else(|| panic!("{id}: no {name}"));
        check(p, &format!("{id}.{name}"));
    }
}

#[test]
fn every_seed_parameter_is_well_formed() {
    for s in all() {
        for p in s.params.iter().chain(s.algorithms.iter().flat_map(|a| a.params.iter())) {
            if is_seed(p) {
                check(p, &format!("{}.{}", s.id, p.name));
            }
        }
    }
}

#[test]
fn only_the_editor_is_not_deterministic() {
    for s in all() {
        assert_eq!(!s.deterministic, s.id == "structure.draw", "{}", s.id);
    }
}
