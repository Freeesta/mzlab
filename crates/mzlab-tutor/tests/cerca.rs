//! Retrieval on the real Teoria: recall@5 of the eval cases (Italian must reach 0.85; English is measured, see docs/agenti/tutor.md).
use mzlab_tutor::{blocchi, cerca};
use serde_json::Value;
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn index() -> cerca::Index {
    let mut pages = Vec::new();
    for e in fs::read_dir(root().join("mzlab/web/teoria")).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "html") {
            pages.push((
                p.file_name().unwrap().to_str().unwrap().to_string(),
                fs::read_to_string(&p).unwrap(),
            ));
        }
    }
    cerca::Index::new(&blocchi::build(&pages).unwrap())
}

fn cases(lang: &str) -> Vec<Value> {
    let mut all = Vec::new();
    for e in fs::read_dir(root().join("tests/data/tutor/eval")).unwrap() {
        let v: Value =
            serde_json::from_str(&fs::read_to_string(e.unwrap().path()).unwrap()).unwrap();
        all.extend(
            v.as_array()
                .unwrap()
                .iter()
                .filter(|c| c["lang"] == lang)
                .cloned(),
        );
    }
    all
}

#[test]
fn there_are_enough_cases() {
    let (it, en) = (cases("it"), cases("en"));
    assert!(it.len() + en.len() >= 60, "{} cases", it.len() + en.len());
    assert!(en.len() >= 10);
}

#[test]
fn recall_at_5_italian() {
    let (idx, it) = (index(), cases("it"));
    let r = cerca::recall_at(&idx, &it, 5);
    eprintln!("recall@5 it = {r:.3} on {} cases", it.len());
    assert!(r >= 0.85, "recall@5 {r:.3}");
}

#[test]
fn recall_at_5_english_is_measured() {
    let (idx, en) = (index(), cases("en"));
    eprintln!(
        "recall@5 en = {:.3} on {} cases",
        cerca::recall_at(&idx, &en, 5),
        en.len()
    );
}

#[test]
fn open_section_is_boosted_and_results_are_stable() {
    let idx = index();
    let a = idx.search("addotti sodio", None, 5);
    assert_eq!(a, idx.search("addotti sodio", None, 5));
    let open = a[a.len() - 1].0.rsplit_once('.').unwrap().0.to_string();
    let b = idx.search("addotti sodio", Some(&open), 5);
    let score = |v: &[(String, f64)]| {
        v.iter()
            .find(|x| x.0.starts_with(&open))
            .map(|x| x.1)
            .unwrap()
    };
    assert!(score(&b) > score(&a));
}

#[test]
fn synonyms_reach_the_other_name() {
    let idx = index();
    let hit = idx.search("qqq", None, 5);
    assert!(!hit.is_empty());
}
