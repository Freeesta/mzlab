//! `mzlab-cli sample design`: the faults a design file or a samples file can have, the exit code of each, and the answers under `--json`.

use std::path::Path;

use super::tests::{dir, ok, p, problems_of, propose, run};

/// The (key, code) problems of `design` given as text, with the exit code `validate` returns.
fn verdict(tag: &str, design: &str) -> (u8, Vec<(String, String)>) {
    let d = dir(tag);
    std::fs::write(d.join("d.json"), design).unwrap();
    let o = run(&["sample", "design", "validate", &p(&d, "d.json"), "--json"]);
    let got = if o.code == 3 {
        problems_of(&o)
    } else {
        Vec::new()
    };
    let _ = std::fs::remove_dir_all(&d);
    (o.code, got)
}

fn has(got: &[(String, String)], key: &str, code: &str) -> bool {
    got.contains(&(key.to_string(), format!("err.design.problem.{code}")))
}

const COND: &str =
    r#"[{"name":"a","replicates":3,"levels":{}},{"name":"b","replicates":3,"levels":{}}]"#;

#[test]
fn a_design_with_a_missing_reference_is_exit_3_at_the_reference_key() {
    let d = format!(
        r#"{{"schema":"design/1","conditions":{COND},"reference":null,"comparisons":[{{"kind":"vs_reference"}}],"factors":[],"batches":[]}}"#
    );
    let (code, got) = verdict("refmiss", &d);
    assert_eq!(code, 3);
    assert_eq!(
        got,
        vec![(
            "reference".to_string(),
            "err.design.problem.refMissing".to_string()
        )]
    );
}

#[test]
fn the_wrong_schema_the_bad_kind_and_a_duplicate_each_name_their_key() {
    let schema = format!(
        r#"{{"schema":"design/2","conditions":{COND},"reference":null,"comparisons":[],"factors":[],"batches":[]}}"#
    );
    assert!(has(&verdict("schema", &schema).1, "schema", "wrongSchema"));
    let kind = format!(
        r#"{{"schema":"design/1","conditions":{COND},"reference":null,"comparisons":[{{"kind":"all"}}],"factors":[],"batches":[]}}"#
    );
    assert!(has(
        &verdict("kind", &kind).1,
        "comparisons[0].kind",
        "badKind"
    ));
    let twice = r#"{"schema":"design/1","conditions":[{"name":"a","replicates":3,"levels":{}},{"name":"a","replicates":3,"levels":{}}],"reference":null,"comparisons":[],"factors":[],"batches":[]}"#;
    assert_eq!(
        verdict("dup", twice),
        (
            3,
            vec![(
                "conditions[1].name".to_string(),
                "err.design.problem.duplicate".to_string()
            )]
        )
    );
}

#[test]
fn a_samples_file_that_is_not_valid_or_missing_follows_the_exit_codes() {
    let d = dir("badsamples");
    let (_, _) = propose(&d);
    std::fs::write(d.join("junk.json"), "not json").unwrap();
    let (design, junk) = (p(&d, "design.json"), p(&d, "junk.json"));
    // a file that is there but is not a samples/1 document: not valid (3); one that is not there: input (4)
    let o = run(&["sample", "design", "validate", &design, "--samples", &junk]);
    assert_eq!(o.code, 3, "{}", o.stderr);
    assert_eq!(
        run(&[
            "sample",
            "design",
            "validate",
            &design,
            "--samples",
            "/no/such/s.json"
        ])
        .code,
        4
    );
    let o = run(&["sample", "design", "propose", "--samples", &junk]);
    assert_eq!(o.code, 3, "{}", o.stderr);
    assert!(String::from_utf8_lossy(&o.stdout).starts_with("{\"schema\":\"error/1\""));
    assert_eq!(
        run(&[
            "sample",
            "design",
            "propose",
            "--samples",
            "/no/such/s.json"
        ])
        .code,
        4
    );
    // a refused proposal leaves no stale design behind
    let out = p(&d, "stale.json");
    let o = run(&[
        "sample",
        "design",
        "propose",
        "--samples",
        &junk,
        "--out",
        &out,
    ]);
    assert_eq!(o.code, 3);
    assert!(!Path::new(&out).exists(), "no --out file after a failure");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn validate_and_propose_with_samples_answer_json_documents() {
    let d = dir("samplesjson");
    let (s, _) = propose(&d);
    let v = ok(&[
        "sample",
        "design",
        "validate",
        &p(&d, "design.json"),
        "--samples",
        &s,
        "--json",
    ]);
    assert!(
        v.starts_with("{\"schema\":\"design_check/1\",\"valid\":true"),
        "{v}"
    );
    assert!(
        v.contains("\"samples_checked\":true") && v.ends_with("}\n"),
        "{v}"
    );
    let pr = ok(&["sample", "design", "propose", "--samples", &s, "--json"]);
    assert!(pr.starts_with("{\"schema\":\"design_proposal/1\""), "{pr}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn json_that_is_not_samples_is_exit_3_under_validate_and_propose() {
    let d = dir("notsamples");
    let (_, _) = propose(&d);
    std::fs::write(d.join("other.json"), r#"{"files":1}"#).unwrap();
    let (design, other) = (p(&d, "design.json"), p(&d, "other.json"));
    for args in [
        vec!["sample", "design", "validate", &design, "--samples", &other],
        vec!["sample", "design", "propose", "--samples", &other],
    ] {
        let o = run(&args);
        assert_eq!(o.code, 3, "{args:?}: {}", o.stderr);
        assert!(o.stderr.contains("error.badSamples"), "{}", o.stderr);
    }
    let _ = std::fs::remove_dir_all(&d);
}

/// Every rule of `reference_rule` has its own reason text, and no two share one.
#[test]
fn every_reference_rule_has_its_own_lang_key() {
    let mut keys = Vec::new();
    for code in mzlab_core::design::REFERENCE_RULES {
        let key = super::reference_key(code);
        assert!(key.ends_with(code), "{code}: {key}");
        assert!(crate::error_text::catalog_text(key).is_some(), "{key}");
        keys.push(key);
    }
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), mzlab_core::design::REFERENCE_RULES.len());
    // a name for each rule: the code it gives is a known one, and every known code is reached
    let mut seen = Vec::new();
    for name in ["ref_a", "my_reference", "the control", "x_no_gas"] {
        let code = mzlab_core::design::reference_rule(name).unwrap();
        assert!(
            mzlab_core::design::REFERENCE_RULES.contains(&code),
            "{name}: {code}"
        );
        seen.push(code);
    }
    seen.sort_unstable();
    let mut all = mzlab_core::design::REFERENCE_RULES.to_vec();
    all.sort_unstable();
    assert_eq!(seen, all);
    assert!(mzlab_core::design::reference_rule("trt").is_none());
    // a reference no rule matches (the file was edited) has its own text
    assert!(crate::error_text::catalog_text("err.design.why.given").is_some());
}

/// Every proposal says its `why` in a code the schema lists, also one whose reference was edited to a name no rule matches.
#[test]
fn every_proposal_validates_against_its_schema_even_with_an_edited_reference() {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../../../docs/schemi/design_proposal-1.json")).unwrap();
    let item = &schema["properties"]["choices"]["items"];
    let whys: Vec<&str> = item["properties"]["why"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w.as_str().unwrap())
        .collect();
    let names: Vec<&str> = item["properties"]["choice"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w.as_str().unwrap())
        .collect();
    let d = dir("schema");
    let (s, _) = propose(&d);
    let design = std::fs::read_to_string(d.join("design.json"))
        .unwrap()
        .trim_end()
        .to_string();
    let text = std::fs::read_to_string(&s).unwrap();
    let cs = mzlab_core::samples::from_json(text.as_bytes()).unwrap();
    let edited = design.replace("\"reference\":\"ref\"", "\"reference\":\"trt\"");
    assert_ne!(edited, design, "the design has a reference to edit");
    for one in [design.as_str(), edited.as_str()] {
        let doc: serde_json::Value =
            serde_json::from_str(&super::proposal(&cs, one).ok().expect("a proposal")).unwrap();
        for c in doc["choices"].as_array().unwrap() {
            let why = c["why"].as_str().unwrap();
            assert!(whys.contains(&why), "{why} is not in the schema");
            assert!(names.contains(&c["choice"].as_str().unwrap()));
            assert!(!c["reason"].as_str().unwrap().is_empty());
        }
    }
    let _ = std::fs::remove_dir_all(&d);
}
