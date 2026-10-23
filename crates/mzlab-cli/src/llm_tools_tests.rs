//! The catalog `strumenti/1` against the registry: every step and every `--set` name is reachable through it, defaults sit inside their limits, the
//! file `docs/schemi/strumenti-1.json` is the export (`MZLAB_UPDATE_STEPS=1 cargo test -p mzlab-cli --lib llm_tools` rewrites it) and the bytes are stable.

// The tests of `proposta-param/1` and of `reading` ride on this module (the crate root is at its line limit).
#[path = "llm_params_tests.rs"]
mod params;
#[path = "llm_params_g_tests.rs"]
mod params_g;
#[path = "llm_reading_tests.rs"]
mod reading;

use std::collections::BTreeSet;

use mzlab_core::json::{parse, Value};

use crate::flow::steps;
use crate::llm::tools_json;

fn catalog() -> Value {
    parse(tools_json().as_bytes()).expect("the catalog is JSON")
}

fn tool<'a>(c: &'a Value, name: &str) -> &'a Value {
    c.get("tools")
        .unwrap()
        .arr()
        .iter()
        .find(|t| t.get("name").and_then(Value::str) == Some(name))
        .unwrap_or_else(|| panic!("tool {name}"))
}

fn schema<'a>(c: &'a Value, name: &str) -> &'a Value {
    tool(c, name).get("input_schema").unwrap()
}

/// The branches of `flow_set_param` as (step ref, names, value schemas).
fn branches(c: &Value) -> Vec<(String, BTreeSet<String>, Vec<&Value>)> {
    schema(c, "flow_set_param")
        .get("oneOf")
        .unwrap()
        .arr()
        .iter()
        .map(|b| {
            let p = b.get("properties").unwrap();
            let step = p.get("step").unwrap().get("const").unwrap().str().unwrap();
            let names = p.get("name").unwrap().get("enum").unwrap().arr();
            let value = p.get("value").unwrap();
            let values = match value.get("anyOf") {
                Some(a) => a.arr().iter().collect(),
                None => vec![value],
            };
            let names = names.iter().map(|n| n.str().unwrap().to_string()).collect();
            (step.to_string(), names, values)
        })
        .collect()
}

#[test]
fn the_twenty_tools_are_there_in_order() {
    let c = catalog();
    let names: Vec<&str> = c
        .get("tools")
        .unwrap()
        .arr()
        .iter()
        .map(|t| t.get("name").unwrap().str().unwrap())
        .collect();
    assert_eq!(names.len(), 20);
    assert_eq!(names[0], "flow_list_steps");
    assert_eq!(names[19], "ui_show");
    assert_eq!(
        &names[11..17],
        [
            "data_summary",
            "data_inspect",
            "params_propose",
            "table_check",
            "table_rows",
            "ask_user"
        ]
    );
    let classes: BTreeSet<&str> = c
        .get("tools")
        .unwrap()
        .arr()
        .iter()
        .map(|t| t.get("class").unwrap().str().unwrap())
        .collect();
    assert_eq!(
        classes,
        ["edit", "propose", "read", "run", "ui"]
            .into_iter()
            .collect()
    );
}

#[test]
fn every_step_is_in_the_add_step_enum() {
    let c = catalog();
    let list = schema(&c, "flow_add_step")
        .get("properties")
        .unwrap()
        .get("step")
        .unwrap()
        .get("enum")
        .unwrap()
        .arr();
    let got: Vec<&str> = list.iter().map(|v| v.str().unwrap()).collect();
    let mut want: Vec<String> = steps()
        .iter()
        .map(|s| format!("{}@{}", s.id, s.version))
        .collect();
    // The catalog lists the steps sorted by id.
    want.sort();
    assert_eq!(got, want);
}

#[test]
fn set_param_names_are_exactly_the_set_names() {
    let c = catalog();
    let got = branches(&c);
    let mut checked = 0;
    for s in steps() {
        let mut want: BTreeSet<String> = s.params.iter().map(|p| p.name.to_string()).collect();
        want.extend(
            s.algorithms
                .iter()
                .flat_map(|a| a.params.iter().map(|p| p.name.to_string())),
        );
        if !s.algorithms.is_empty() {
            want.insert("algorithm".into());
        }
        if !s.presets.is_empty() {
            want.insert("preset".into());
        }
        let sref = format!("{}@{}", s.id, s.version);
        let branch = got.iter().find(|(r, _, _)| *r == sref);
        match branch {
            Some((_, names, _)) => assert_eq!(names, &want, "{sref}"),
            None => assert!(want.is_empty(), "{sref} has no branch"),
        }
        checked += want.len();
    }
    assert!(checked > 100, "only {checked} names were checked");
}

#[test]
fn defaults_sit_inside_their_limits() {
    let c = catalog();
    let mut checked = 0;
    for (sref, _, values) in branches(&c) {
        for v in values {
            let Some(d) = v.get("default") else { continue };
            checked += 1;
            if let (Some(n), Some(lo)) = (d.f64(), v.get("minimum").and_then(Value::f64)) {
                assert!(n >= lo, "{sref}: default {n} below {lo}");
            }
            if let (Some(n), Some(hi)) = (d.f64(), v.get("maximum").and_then(Value::f64)) {
                assert!(n <= hi, "{sref}: default {n} above {hi}");
            }
            if let Some(list) = v.get("enum") {
                assert!(list.arr().contains(d), "{sref}: default {d:?} not in enum");
            }
            if let (Some(a), Some(lo)) = (
                d.arr().first().and_then(Value::f64),
                v.get("items")
                    .and_then(|i| i.get("minimum"))
                    .and_then(Value::f64),
            ) {
                assert!(a >= lo, "{sref}: default {a} below {lo}");
            }
        }
    }
    assert!(checked > 100, "only {checked} defaults were checked");
}

#[test]
fn list_steps_reports_the_native_executors() {
    let c = catalog();
    let got: BTreeSet<&str> = schema(&c, "flow_list_steps")
        .get("x-executable-native")
        .unwrap()
        .arr()
        .iter()
        .map(|v| v.str().unwrap())
        .collect();
    let want: BTreeSet<&str> = steps()
        .iter()
        .map(|s| s.id)
        .filter(|id| crate::flow::executable(id))
        .collect();
    assert_eq!(got, want);
    assert!(!got.is_empty());
}

#[test]
fn the_file_is_the_export_and_the_bytes_are_stable() {
    let text = tools_json();
    assert_eq!(text, tools_json());
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/schemi/strumenti-1.json"
    );
    if std::env::var_os("MZLAB_UPDATE_STEPS").is_some() {
        std::fs::write(path, &text).unwrap();
    }
    let on_disk = std::fs::read_to_string(path)
        .expect("docs/schemi/strumenti-1.json (MZLAB_UPDATE_STEPS=1 writes it)");
    assert_eq!(
        on_disk, text,
        "docs/schemi/strumenti-1.json is stale: MZLAB_UPDATE_STEPS=1 cargo test -p mzlab-cli --lib llm_tools"
    );
}

#[test]
fn a_set_param_call_of_the_catalog_passes_its_own_schema() {
    let c = catalog();
    let s = schema(&c, "flow_set_param");
    let ok =
        parse(br#"{"instance":"s","step":"spectra.ms2@1","name":"min_intensity","value":100}"#)
            .unwrap();
    assert_eq!(mzlab_llm::schema::validate(s, &ok), Ok(()));
    let wrong =
        parse(br#"{"instance":"s","step":"spectra.ms2@1","name":"nope","value":1}"#).unwrap();
    assert!(mzlab_llm::schema::validate(s, &wrong).is_err());
}

fn accepts(name: &str, args: &[u8]) -> bool {
    let c = catalog();
    mzlab_llm::schema::validate(schema(&c, name), &parse(args).unwrap()).is_ok()
}

#[test]
fn the_assistant_tools_only_read_or_propose() {
    let c = catalog();
    for (n, class) in [
        ("data_inspect", "read"),
        ("params_propose", "propose"),
        ("table_check", "read"),
        ("table_rows", "read"),
        ("ask_user", "propose"),
    ] {
        assert_eq!(
            tool(&c, n).get("class").and_then(Value::str),
            Some(class),
            "{n}"
        );
        assert_eq!(
            schema(&c, n)
                .get("additionalProperties")
                .and_then(Value::bool),
            Some(false),
            "{n}"
        );
    }
}

#[test]
fn the_assistant_tools_take_closed_arguments() {
    assert!(accepts("data_inspect", br#"{}"#));
    assert!(accepts("data_inspect", br#"{"files":["a","b"]}"#));
    assert!(!accepts("data_inspect", br#"{"files":"a"}"#));
    assert!(!accepts("data_inspect", br#"{"path":"/x"}"#));
    let step = steps()
        .iter()
        .map(|s| format!("{}@{}", s.id, s.version))
        .next()
        .unwrap();
    assert!(accepts(
        "params_propose",
        format!(r#"{{"steps":["{step}"]}}"#).as_bytes()
    ));
    assert!(!accepts("params_propose", br#"{"steps":["nope@1"]}"#));
    assert!(!accepts("params_propose", br#"{}"#));
    assert!(accepts("table_check", br#"{"checks":["blank","qc_cv"]}"#));
    assert!(!accepts("table_check", br#"{"checks":["nope"]}"#));
    assert!(accepts("table_rows", br#"{"ids":["3"],"limit":50}"#));
    assert!(accepts(
        "table_rows",
        br#"{"where":{"column":"area","op":"gt","value":1000}}"#
    ));
    assert!(!accepts("table_rows", br#"{"limit":51}"#));
    assert!(!accepts(
        "table_rows",
        br#"{"where":{"column":"area","op":"like","value":1}}"#
    ));
}

#[test]
fn ask_user_wants_one_to_five_closed_questions() {
    let q = |n: usize| {
        let one = r#"{"id":"q","text":"Which?","options":["a","b"]}"#;
        format!(r#"{{"questions":[{}]}}"#, vec![one; n].join(","))
    };
    assert!(accepts("ask_user", q(1).as_bytes()));
    assert!(accepts("ask_user", q(5).as_bytes()));
    assert!(!accepts("ask_user", q(6).as_bytes()));
    assert!(!accepts("ask_user", br#"{"questions":[]}"#));
    assert!(!accepts(
        "ask_user",
        br#"{"questions":[{"id":"q","text":"Which?","options":["a"]}]}"#
    ));
    assert!(!accepts(
        "ask_user",
        br#"{"questions":[{"id":"q","text":"Which?"}]}"#
    ));
}
