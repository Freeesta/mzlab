#![cfg(feature = "attributio")]
//! The Banco 2b: the decision rules on hand cases, the alternatives with the same formula, the review of one feature of the folder and the whole round of a
//! decision (write `feature_decisions.csv`, run the cascade again). The folder is the synthetic campaign plus `campagna_banco/` (one more feature, a Level 3
//! paracetamol with an MS2 and a library of three isomers), all invented.

#[macro_use]
mod attributio_comune;
use attributio_comune::*;
use mzlab_feat::attributio::io::{Io, MemFiles};
use mzlab_feat::attributio::review::alternatives::{same_formula, AltSources};
use mzlab_feat::attributio::review::banco::review_json;
use mzlab_feat::attributio::review::libraries::{LibFormat, LibInput, LibraryIndex};
use mzlab_feat::attributio::review::matching::Ms2;
use mzlab_feat::attributio::review::rules::{
    decision_changes, note_required, page_level, Current, Rec,
};
use mzlab_feat::attributio::review::save::decision_text;
use mzlab_feat::attributio::review::structures::NoStructures;
use mzlab_feat::attributio::table;
use serde_json::Value;

fn banco_files() -> MemFiles {
    let mut files = fixture("campagna_sintetica").files;
    let base = root().join("tests/data/attributio/campagna_banco");
    walk(&base, &base, &mut files);
    files
}

#[test]
fn page_levels() {
    assert_eq!(page_level(2, "a", false), "2a");
    assert_eq!(page_level(2, "b", true), "2b");
    assert_eq!(page_level(3, "b", true), "2b", "a 2b counted as 3 stays 2b");
    assert_eq!(page_level(3, "b", false), "3", "a 3b is a 3");
    assert_eq!(page_level(1, "", false), "1");
}

#[test]
fn a_note_is_required_only_when_the_decision_changes_something() {
    let f = Current {
        level: 3,
        level_sub: "a",
        decision_2b: false,
        name: Some("Paracetamol"),
        group: None,
    };
    let rec = |level, group, name| Rec { level, group, name };
    assert_eq!(
        decision_changes(&f, &rec("2b", "", "")),
        vec!["level 3 -> 2b"]
    );
    assert!(note_required(&f, &rec("2b", "", "")));
    assert!(!note_required(&f, &rec("3", "", "")), "3 stays 3");
    assert!(
        !note_required(&f, &rec("", "", "Paracetamol")),
        "the same name changes nothing"
    );
    assert_eq!(
        decision_changes(&f, &rec("", "ambiguous", "")),
        vec!["identity rejected"]
    );
    assert_eq!(
        decision_changes(&f, &rec("", "", "Acetaminophen")),
        vec!["name -> Acetaminophen"]
    );
    let two_a = Current {
        level: 2,
        level_sub: "a",
        name: Some("X"),
        ..Current::default()
    };
    assert_eq!(
        decision_changes(&two_a, &rec("3a", "", "")),
        vec!["level 2a -> 3"]
    );
    assert!(decision_changes(&two_a, &rec("2", "", "")).is_empty());
}

fn library() -> LibraryIndex {
    let base = root().join("tests/data/attributio/campagna_banco/library/banco.json");
    LibraryIndex::build(vec![LibInput {
        label: "library/banco.json".into(),
        format: LibFormat::Json,
        bytes: std::fs::read(base).expect("library"),
    }])
}

#[test]
fn the_alternatives_with_the_same_formula_are_the_isomers_of_the_library() {
    let lib = library();
    let exp = Ms2 {
        mz: vec![65.0386, 93.0335, 110.06, 152.0706],
        it: vec![60.0, 150.0, 1000.0, 30.0],
        prec: 152.0706,
    };
    let src = AltSources {
        csi: &[],
        exp: Some(&exp),
        tol: 0.02,
        st: &NoStructures,
    };
    let alts = same_formula("C8H9NO2", &lib, &src);
    let names: Vec<&str> = alts.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["Metacetamol", "Paracetamol", "Methyl anthranilate"]);
    let s: Vec<f64> = alts.iter().map(|a| a.score.expect("score")).collect();
    assert!(s[0] > s[1] && s[1] > s[2], "{s:?}");
    assert!(s[0] > 0.999 && s[1] > 0.99, "{s:?}");
    assert_eq!(alts[1].ik1.as_deref(), Some("RZVAJINKPMORJF"));
    assert_eq!(alts[0].source, "library: library/banco.json");
    assert!(same_formula("C8H10N4O2", &lib, &src)
        .iter()
        .all(|a| a.name == "Caffeine"));
    assert!(same_formula("C99H99", &lib, &src).is_empty());
    assert!(same_formula("", &lib, &src).is_empty());
    let none = AltSources { exp: None, ..src };
    assert!(same_formula("C8H9NO2", &lib, &none)
        .iter()
        .all(|a| a.score.is_none()));
}

fn review(files: &MemFiles, pol: &str, id: i64) -> Value {
    let read = |p: &str| files.read(p);
    let list = |d: &str| files.list(d);
    let io = Io::new(&read, &list);
    serde_json::from_str(&review_json(&io, "campagna_banco", pol, id).expect("review"))
        .expect("JSON")
}

#[test]
fn the_review_of_a_feature_has_its_proofs_and_proposes_nothing_ticked() {
    let v = review(&banco_files(), "POS", 15);
    assert_eq!(v["current"]["level"], "3");
    let p = &v["proposal"];
    assert_eq!(p["class"], "2b-check", "a competing isomer matches better");
    assert_eq!(p["best"]["formula"], "C8H9NO2");
    let ev = &p["evidence"];
    assert_eq!(
        (ev["n_explained"].as_i64(), ev["n_peaks"].as_i64()),
        (Some(3), Some(3))
    );
    let peaks = ev["view"]["peaks"].as_array().expect("peaks");
    assert!(peaks.iter().filter(|k| !k["frag"].is_null()).count() >= 3);
    assert!(
        !ev["view"]["mirror"].is_null(),
        "mirror against the library"
    );
    assert!(!ev["view"]["isotopes"]
        .as_array()
        .expect("isotopes")
        .is_empty());
    let c = v["checklist"].as_array().expect("checklist");
    let keys: Vec<&str> = c.iter().map(|x| x["key"].as_str().expect("key")).collect();
    assert_eq!(
        keys,
        [
            "diagnostic_fragment",
            "neutral_loss",
            "isotopes",
            "pure_precursor",
            "rt_plausible",
            "alternatives_excluded"
        ]
    );
    assert!(c[0]["proof"].as_str().expect("proof").starts_with("3 of 3"));
    assert!(c[2]["proof"].is_null(), "no raw data: no isotope proof");
    assert!(v["alternatives"].as_array().expect("alts").len() >= 3);
    assert_eq!(v["alternatives"][0]["is_best"], false);
    assert_eq!(v["alternatives"][1]["is_best"], true);
    assert_eq!(v["has_ms2"], true);
    assert!(v["decision"].is_null());
}

#[test]
fn a_feature_without_ms2_is_reviewed_without_error() {
    let v = review(&banco_files(), "POS", 4);
    assert_eq!(v["has_ms2"], false);
    assert!(v["proposal"]["evidence"]["view"]["peaks"]
        .as_array()
        .expect("peaks")
        .is_empty());
}

fn decision(extra: &str) -> String {
    format!(
        r#"{{"pol":"POS","id":15,"level":"2b","justification":"diagnostic ions explained","decided_on":"2026-10-16","decided_by":"rev",
        "checklist":["diagnostic_fragment","neutral_loss","alternatives_excluded"],
        "evidence":{{"type":"fragments","library":"library/banco.json","entry":"","cosine":0.99,"matched":4,"delta_mz":0,"delta_formula":"","note":"x"}}{extra}}}"#
    )
}

/// The decisions file of the round is kept in `tests/golden/attributio/banco/`; with `ATTRIBUTIO_SCRIVI_ORO=1` the test writes it (the reference tool reads it next).
fn golden_csv(text: &str) {
    let p = root().join("tests/golden/attributio/banco/feature_decisions.csv");
    if std::env::var("ATTRIBUTIO_SCRIVI_ORO").is_ok() {
        std::fs::create_dir_all(p.parent().expect("dir")).expect("mkdir");
        std::fs::write(&p, text).expect("write");
    }
    assert_eq!(
        text,
        std::fs::read_to_string(&p)
            .expect("golden csv")
            .replace("\r\n", "\n"),
        "decisions file"
    );
}

fn decide(files: &mut MemFiles, json: &str) -> mzlab_feat::attributio::review::save::Saved {
    let read = |p: &str| files.read(p);
    let list = |d: &str| files.list(d);
    let io = Io::new(&read, &list);
    let saved = decision_text(&io, "campagna_banco", json).expect("saved");
    files.add(&saved.path, saved.text.as_bytes());
    saved
}

fn row_of(files: &MemFiles, id: i64) -> Vec<mzlab_feat::attributio::annotate::Feature> {
    let read = |p: &str| files.read(p);
    let list = |d: &str| files.list(d);
    let io = Io::new(&read, &list);
    table::run(&io, "campagna_banco")
        .expect("run")
        .features
        .into_iter()
        .filter(|f| f.id == id && f.pol == "POS")
        .collect()
}

/// Every level of the campaign after the decision is the one `tools/attributio_riferimento.py` gives on the same folder (`livelli.json`).
fn levels_match_reference(files: &MemFiles) {
    let want: Value = serde_json::from_str(
        &std::fs::read_to_string(root().join("tests/golden/attributio/banco/livelli.json"))
            .expect("golden levels"),
    )
    .expect("JSON");
    let read = |p: &str| files.read(p);
    let list = |d: &str| files.list(d);
    let io = Io::new(&read, &list);
    let run = table::run(&io, "campagna_banco").expect("run");
    assert_eq!(run.features.len(), want.as_object().expect("object").len());
    for f in &run.features {
        let w = &want[format!("{}:{}", f.pol, f.id)];
        // the sublevel of a Level 3 (3a/3b) is newer than the Attributio the reference ran on: only the 2a/2b sublevels are compared
        let sub = if f.level == 2 || f.level_sub == "b" {
            f.level_sub.as_str()
        } else {
            ""
        };
        let wsub = if w[0] == 2 || w[1] == "b" {
            w[1].as_str().expect("sub")
        } else {
            ""
        };
        let got = (f.level as i64, sub, f.level_basis.as_str());
        assert_eq!(
            (
                w[0].as_i64().expect("level"),
                wsub,
                w[2].as_str().expect("basis")
            ),
            got,
            "{} {}",
            f.pol,
            f.id
        );
    }
}

#[test]
fn a_2b_decision_makes_the_feature_2b_and_survives_the_cascade() {
    let mut files = banco_files();
    assert_eq!(row_of(&files, 15)[0].level, 3);
    let saved = decide(&mut files, &decision(""));
    assert_eq!(saved.path, "03 Stats/scripts/shared/feature_decisions.csv");
    assert_eq!((saved.added, saved.replaced), (1, 0));
    let header = saved.text.lines().next().expect("header");
    assert!(header.ends_with(";evidence_checklist"), "{header}");
    let last = saved.text.lines().last().expect("row");
    assert!(
        last.ends_with(";diagnostic_fragment|neutral_loss|alternatives_excluded"),
        "{last}"
    );
    let f = &row_of(&files, 15)[0];
    assert_eq!(
        (f.level, f.level_sub.as_str(), f.level_basis.as_str()),
        (2, "b", "diagnostic evidence")
    );
    golden_csv(&saved.text);
    levels_match_reference(&files);
    // the same day and reviewer: the row is replaced, not repeated
    let again = decide(&mut files, &decision(r#","name":"Acetaminophen""#));
    assert_eq!((again.added, again.replaced), (0, 1));
    assert_eq!(again.text.lines().count(), saved.text.lines().count());
    // another day: the history grows
    let day2 = decision("").replace("2026-10-16", "2026-10-17");
    assert_eq!(decide(&mut files, &day2).added, 1);
}

#[test]
fn a_decision_against_the_proposal_needs_a_note() {
    let mut files = banco_files();
    let read = |p: &str| files.read(p);
    let list = |d: &str| files.list(d);
    let io = Io::new(&read, &list);
    let no_note = decision("").replace("diagnostic ions explained", "");
    let e = decision_text(&io, "campagna_banco", &no_note).expect_err("note");
    assert!(e.to_string().contains("note"), "{e}");
    let bad = decision("").replace("\"2b\"", "\"9\"");
    assert!(decision_text(&io, "campagna_banco", &bad).is_err());
    let unknown = decision("").replace("\"id\":15", "\"id\":99");
    assert!(decision_text(&io, "campagna_banco", &unknown).is_err());
    let box_ = decision("").replace("diagnostic_fragment\"", "magic\"");
    assert!(decision_text(&io, "campagna_banco", &box_).is_err());
    let _ = &mut files;
}
