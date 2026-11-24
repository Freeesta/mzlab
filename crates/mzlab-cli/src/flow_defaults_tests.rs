//! `flow defaults`: every step of the registry is covered, with exactly the parameters of its descriptor, and every default is a value `--set` and
//! `flow check` accept.

use mzlab_core::flow::{check, parse, Platform};
use mzlab_core::json::{self, Value};
use mzlab_core::step::json::step_json;

use crate::execute;
use crate::flow::steps;
use crate::flow_run::apply_set;

fn run(args: &[&str]) -> crate::Outcome {
    execute(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn doc(id: &str) -> Value {
    let o = run(&["flow", "defaults", id, "--json"]);
    assert_eq!(o.code, 0, "{id}: {}", o.stderr);
    json::parse(&o.stdout).unwrap_or_else(|_| panic!("{id}: not JSON"))
}

/// The text `--set` reads back for a default.
fn word(v: &Value) -> String {
    match v {
        Value::Num(n) => format!("{n}"),
        Value::Str(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Arr(a) => a.iter().map(word).collect::<Vec<_>>().join(","),
        other => panic!("no word for {other:?}"),
    }
}

#[test]
fn every_step_lists_exactly_the_parameters_of_its_descriptor() {
    let reg = steps();
    assert!(reg.len() > 50, "the registry is not the whole list");
    for s in &reg {
        let want: Vec<String> = s
            .params
            .iter()
            .map(|p| p.name.to_string())
            .chain(
                s.algorithms
                    .iter()
                    .flat_map(|a| a.params.iter().map(|p| p.name.to_string())),
            )
            .collect();
        let d = doc(&format!("{}@{}", s.id, s.version));
        let got: Vec<String> = d
            .get("params")
            .unwrap()
            .arr()
            .iter()
            .map(|p| p.get("name").and_then(Value::str).unwrap().to_string())
            .collect();
        assert_eq!(got, want, "{}@{}", s.id, s.version);
        // the descriptor of the registry is the source: the same text gives the same answer
        let direct = mzlab_core::step::defaults::route(&step_json(s)).unwrap();
        let cli = run(&[
            "flow",
            "defaults",
            &format!("{}@{}", s.id, s.version),
            "--json",
        ]);
        let body = direct.trim_end().strip_suffix('}').unwrap();
        assert!(
            String::from_utf8(cli.stdout).unwrap().starts_with(body),
            "{}",
            s.id
        );
    }
}

#[test]
fn a_rule_that_is_off_when_empty_has_a_null_default_and_says_so() {
    let d = doc("table.filter");
    let p = d
        .get("params")
        .unwrap()
        .arr()
        .iter()
        .find(|p| p.get("name").and_then(Value::str) == Some("blank_ratio"))
        .expect("blank_ratio is listed");
    assert!(p.get("default").unwrap().is_null());
    assert_eq!(p.get("when_empty").and_then(Value::str), Some("off"));
    assert_eq!(
        p.get("set_key").and_then(Value::str),
        Some("filter.blank_ratio")
    );
}

#[test]
fn every_default_is_accepted_by_set_and_the_checker() {
    let reg = steps();
    let mut checked = 0;
    for s in &reg {
        let id = format!("{}@{}", s.id, s.version);
        let t = run(&["flow", "steps", "--id", &id, "--template"]);
        let plan = parse(&t.stdout).unwrap();
        let d = doc(&id);
        for p in d.get("params").unwrap().arr() {
            let (Some(key), Some(def)) = (p.get("set_key").and_then(Value::str), p.get("default"))
            else {
                continue;
            };
            if def.is_null() {
                continue;
            }
            let mut plan = plan.clone();
            if let Some(a) = p.get("algorithm").and_then(Value::str) {
                let (inst, _) = key.split_once('.').unwrap();
                apply_set(&mut plan, &reg, &format!("{inst}.algorithm={a}")).unwrap();
            }
            let set = format!("{key}={}", word(def));
            apply_set(&mut plan, &reg, &set).unwrap_or_else(|e| panic!("{id}: {e}"));
            let bad: Vec<&str> = check(&plan, &reg, Platform::Native)
                .iter()
                .map(|p| p.error_key)
                .filter(|k| k.starts_with("err.flow.param"))
                .collect();
            assert!(bad.is_empty(), "{id}: {set} gives {bad:?}");
            checked += 1;
        }
    }
    assert!(checked > 100, "only {checked} defaults were checked");
}

#[test]
fn an_unknown_step_exits_5_with_the_closest_ids() {
    let o = run(&["flow", "defaults", "table.fliter"]);
    assert_eq!(o.code, 5);
    assert!(o.stdout.is_empty());
    assert!(
        o.stderr.contains("hint: closest: table.filter"),
        "{}",
        o.stderr
    );
}

#[test]
fn usage_errors_exit_2_and_the_text_form_has_one_line_per_parameter() {
    assert_eq!(run(&["flow", "defaults"]).code, 2);
    assert_eq!(run(&["flow", "defaults", "a", "b"]).code, 2);
    assert_eq!(run(&["flow", "defaults", "--nope"]).code, 2);
    let o = run(&["flow", "defaults", "table.filter"]);
    assert_eq!(o.code, 0);
    let text = String::from_utf8(o.stdout).unwrap();
    assert_eq!(
        text.lines().count(),
        doc("table.filter").get("params").unwrap().arr().len()
    );
    assert!(
        text.contains("filter.blank_ratio") && text.contains("(off)"),
        "{text}"
    );
}
