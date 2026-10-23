//! `mzlab_llm::reading::validate` against the real registry: a good `lettura/1` and `param-scelte/1` pass, and every defect (an evidence key the input does not
//! have, a number it does not have, a question that is not closed, a parameter outside its limits, a source that does not match) gives its own problem.

use mzlab_core::json::{parse, Value};
use mzlab_llm::params_rules::target_param;
use mzlab_llm::reading::{problems_json, validate};

use super::params::{run, G01};
use crate::flow::steps;

const INPUT: &str = r#"{"tabcheck/1":{"checks":[
 {"id":"adducts","value":1,"threshold":0.9,"flag":"note","rows":["r4","r12"],"proof":{"delta_mz":22.0,"expected":21.98}},
 {"id":"blank","value":0.12,"threshold":3,"flag":"warn","rows":[],"proof":{"rows":412,"below":23}}]},
 "answers":{"goal":"transformation products"}}"#;

const GOOD: &str = r#"{"schema":"lettura/1","kind":"table","findings":[
 {"id":"F1","topic":"blank","statement":"23 of 412 rows are below the blank ratio of 3","evidence":["tabcheck.blank.proof","tabcheck.blank.value"],
  "confidence":"high","action":{"type":"set_param","step":"tp.dig_lr@1","name":"tol_da","value":0.4}},
 {"id":"F2","topic":"adducts","statement":"The difference 22 is consistent with the expected 21.98","evidence":["tabcheck.adducts.rows"],
  "confidence":"medium","action":{"type":"ask","ref":"Q1"}}],
 "questions":[{"id":"Q1","text":"Is the signal a known additive?","options":["yes","no"]}],
 "numbers":["23","412","3","22","21.98"]}"#;

fn json(s: &str) -> Value {
    parse(s.as_bytes()).expect("JSON")
}

/// The reasons of the problems of `reading` over `input` (empty = valid).
fn reasons(reading: &str, input: &str) -> Vec<String> {
    match validate(&json(reading), &json(input), &steps()) {
        Ok(()) => Vec::new(),
        Err(p) => p
            .iter()
            .map(|p| p.code.trim_start_matches("err.llm.reading.").to_string())
            .collect(),
    }
}

fn bad(reading: &str) -> Vec<String> {
    reasons(reading, INPUT)
}

#[test]
fn a_good_reading_passes() {
    assert_eq!(bad(GOOD), Vec::<String>::new());
}

#[test]
fn an_evidence_key_the_input_does_not_have_is_refused() {
    let r = GOOD.replace("tabcheck.adducts.rows", "tabcheck.adducts.nothing");
    assert_eq!(bad(&r), ["evidence"]);
    let r = GOOD.replace("tabcheck.adducts.rows", "other.adducts.rows");
    assert_eq!(bad(&r), ["evidence"]);
    let r = GOOD.replace(
        r#""evidence":["tabcheck.adducts.rows"]"#,
        r#""evidence":[]"#,
    );
    assert_eq!(bad(&r), ["no_evidence"]);
}

#[test]
fn a_number_that_is_not_in_the_input_is_refused_in_a_sentence_and_in_the_list() {
    let r = GOOD.replace("of 3", "of 5");
    assert_eq!(bad(&r), ["number"]);
    let r = GOOD.replace(r#""numbers":["23""#, r#""numbers":["99","23""#);
    assert_eq!(bad(&r), ["number"]);
    // a name with digits glued to letters is not a number
    let r = GOOD.replace("The difference 22", "Rows r12 and r4 differ by 22");
    assert_eq!(bad(&r), Vec::<String>::new());
}

#[test]
fn questions_are_closed_few_and_the_ask_action_points_at_one() {
    let r = GOOD.replace(r#""options":["yes","no"]"#, r#""options":["yes"]"#);
    assert_eq!(bad(&r), ["question"]);
    let r = GOOD.replace(r#""ref":"Q1""#, r#""ref":"Q9""#);
    assert_eq!(bad(&r), ["action"]);
    let many: Vec<String> = (1..=6)
        .map(|i| format!(r#"{{"id":"Q{i}","text":"Q?","options":["a","b"]}}"#))
        .collect();
    let r = GOOD.replace(
        r#""questions":[{"id":"Q1","text":"Is the signal a known additive?","options":["yes","no"]}]"#,
        &format!(r#""questions":[{}]"#, many.join(",")),
    );
    assert_eq!(bad(&r), ["too_many"]);
}

#[test]
fn at_most_eight_findings_and_a_known_kind_and_confidence() {
    let one = r#"{"id":"F","topic":"t","statement":"ok","evidence":["answers.goal"],"confidence":"low","action":{"type":"none"}}"#;
    let nine = [one; 9].join(",");
    let r = format!(r#"{{"schema":"lettura/1","kind":"table","findings":[{nine}]}}"#);
    assert_eq!(bad(&r), ["too_many"]);
    let r = GOOD.replace(r#""kind":"table""#, r#""kind":"verdict""#);
    assert_eq!(bad(&r), ["kind"]);
    let r = GOOD.replace(r#""confidence":"high""#, r#""confidence":"sure""#);
    assert_eq!(bad(&r), ["confidence"]);
    assert_eq!(bad(r#"{"schema":"lettura/2"}"#), ["schema"]);
}

#[test]
fn set_param_names_a_real_parameter_inside_its_limits() {
    let r = GOOD.replace(r#""value":0.4"#, r#""value":-1"#);
    assert_eq!(bad(&r), ["out_of_range"]);
    let r = GOOD.replace(r#""name":"tol_da""#, r#""name":"nothing""#);
    assert_eq!(bad(&r), ["unknown_param"]);
    let r = GOOD.replace("tp.dig_lr@1", "nothing.here@1");
    assert_eq!(bad(&r), ["unknown_param"]);
    let r = GOOD.replace(
        r#""action":{"type":"set_param","step":"tp.dig_lr@1","name":"tol_da","value":0.4}"#,
        r#""action":{"type":"add_step","step":"nothing@1"}"#,
    );
    assert_eq!(bad(&r), ["action"]);
}

fn text(v: &Value) -> String {
    let mut o = mzlab_core::json::Out::default();
    mzlab_llm::tools::value(&mut o, v);
    o.0
}

/// The input of `param-scelte/1`: the proposals of the rules for the unit-resolution series G01 and the answers of the user.
fn proposals_input() -> String {
    let out = run(G01, "{}");
    let mut o = String::from(r#"{"proposta-param/1":"#);
    o.push_str(&text(&out));
    o.push_str(r#","answers":{"goal":"transformation products"}}"#);
    o
}

/// A `param-scelte/1` that keeps every proposal, with `edit` applied to the choice of `name`.
fn scelte(input: &str, edit: &dyn Fn(&str, &Value) -> Option<String>) -> String {
    let doc = json(input);
    let props = doc
        .get("proposta-param/1")
        .unwrap()
        .get("proposals")
        .unwrap();
    let mut choices = Vec::new();
    for p in props.arr() {
        let name = p.get("name").and_then(Value::str).unwrap();
        let value = text(p.get("value").unwrap());
        let kept = format!(
            r#"{{"step":"{}","name":"{name}","value":{value},"source":"rule:{}","reason":"Kept."}}"#,
            p.get("step").and_then(Value::str).unwrap(),
            p.get("rule").and_then(Value::str).unwrap(),
        );
        let alg = p.get("algorithm").and_then(Value::str);
        let kept = match alg {
            Some(a) => kept.replacen('{', &format!(r#"{{"algorithm":"{a}","#), 1),
            None => kept,
        };
        choices.push(edit(name, p).unwrap_or(kept));
    }
    format!(
        r#"{{"schema":"param-scelte/1","choices":[{}],"questions":[],"skipped":[],"numbers":[]}}"#,
        choices.join(",")
    )
}

#[test]
fn choices_that_keep_every_proposal_pass() {
    let input = proposals_input();
    assert_eq!(
        reasons(&scelte(&input, &|_, _| None), &input),
        Vec::<String>::new()
    );
}

#[test]
fn a_changed_value_says_model_a_kept_one_names_its_rule() {
    let input = proposals_input();
    let doc = json(&input);
    let props = doc
        .get("proposta-param/1")
        .unwrap()
        .get("proposals")
        .unwrap();
    let registry = steps();
    let lower = |p: &Value| {
        let id = p.get("step").and_then(Value::str)?.split('@').next()?;
        let alg = p.get("algorithm").and_then(Value::str).unwrap_or("");
        let name = p.get("name").and_then(Value::str)?;
        target_param(&registry, id, alg, name)?.min
    };
    let first = props
        .arr()
        .iter()
        .find(|p| lower(p).is_some() && p.get("value").and_then(Value::f64).is_some())
        .expect("a numeric proposal with a lower limit");
    let value = first.get("value").and_then(Value::f64).unwrap();
    let below = lower(first).unwrap() - 1.0;
    let (target, step) = (
        first.get("name").and_then(Value::str).unwrap(),
        first.get("step").and_then(Value::str).unwrap(),
    );
    let rule = first.get("rule").and_then(Value::str).unwrap();
    let alg = first
        .get("algorithm")
        .and_then(Value::str)
        .map_or(String::new(), |a| format!(r#""algorithm":"{a}","#));
    let pick = |source: &str, value: &str| {
        let one = format!(
            r#"{{"step":"{step}",{alg}"name":"{target}","value":{value},"source":"{source}","reason":"Changed."}}"#
        );
        move |name: &str, _: &Value| (name == target).then(|| one.clone())
    };
    let r = scelte(&input, &pick("model", &(value + 1.0).to_string()));
    assert!(!reasons(&r, &input).contains(&"source".to_string()));
    let r = scelte(
        &input,
        &pick(&format!("rule:{rule}"), &(value + 1.0).to_string()),
    );
    assert!(reasons(&r, &input).contains(&"source".to_string()));
    let r = scelte(&input, &pick("model", &below.to_string()));
    assert!(reasons(&r, &input).contains(&"out_of_range".to_string()));
}

#[test]
fn a_missing_unknown_or_invented_choice_is_refused() {
    let input = proposals_input();
    let r = scelte(&input, &|_, _| None);
    // drop the first choice
    let doc = json(&r);
    let kept: Vec<String> = doc
        .get("choices")
        .unwrap()
        .arr()
        .iter()
        .skip(1)
        .map(text)
        .collect();
    let short = format!(
        r#"{{"schema":"param-scelte/1","choices":[{}],"questions":[],"skipped":[],"numbers":[]}}"#,
        kept.join(",")
    );
    assert_eq!(reasons(&short, &input), ["missing_choice"]);
    let extra = r.replacen(
        r#""choices":["#,
        r#""choices":[{"step":"nothing@1","name":"x","value":1,"source":"model","reason":"r"},"#,
        1,
    );
    assert_eq!(reasons(&extra, &input), ["unknown_choice"]);
    let invented = r.replacen("Kept.", "Half of 0.123456 is better.", 1);
    assert_eq!(reasons(&invented, &input), ["number"]);
}

#[test]
fn problems_go_back_to_the_subagent_as_json() {
    let r = GOOD.replace("tabcheck.adducts.rows", "tabcheck.adducts.nothing");
    let p = validate(&json(&r), &json(INPUT), &steps()).err().unwrap();
    assert_eq!(
        problems_json(&p),
        r#"[{"code":"err.llm.reading.evidence","at":"findings[1].evidence[0]"}]"#
    );
}
