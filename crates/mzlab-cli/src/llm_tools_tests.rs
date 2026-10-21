//! The catalog `strumenti/1` against the registry: every step and every `--set` name is reachable through it, defaults sit inside their limits, the
//! file `docs/schemi/strumenti-1.json` is the export (`MZLAB_UPDATE_STEPS=1 cargo test -p mzlab-cli --lib llm_tools` rewrites it) and the bytes are stable.

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
fn the_fifteen_tools_are_there_in_order() {
    let c = catalog();
    let names: Vec<&str> = c
        .get("tools")
        .unwrap()
        .arr()
        .iter()
        .map(|t| t.get("name").unwrap().str().unwrap())
        .collect();
    assert_eq!(names.len(), 15);
    assert_eq!(names[0], "flow_list_steps");
    assert_eq!(names[14], "ui_show");
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
    let ok = parse(br#"{"instance":"s","step":"spectra.ms2@1","name":"min_intensity","value":100}"#).unwrap();
    assert_eq!(mzlab_llm::schema::validate(s, &ok), Ok(()));
    let wrong = parse(br#"{"instance":"s","step":"spectra.ms2@1","name":"nope","value":1}"#).unwrap();
    assert!(mzlab_llm::schema::validate(s, &wrong).is_err());
}
