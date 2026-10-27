//! Tests of `line.rs`: the short line, its way back and the errors of a wrong `--set`.

use super::line::*;
use super::{parse, plan_json};
use crate::step::{Param, Step};

fn reg() -> Vec<Step> {
    vec![Step {
        params: vec![
            Param::number("ppm", "Tolerance").default_num(5.0),
            Param::boolean("flag", "Flag").default_bool(false),
            Param::text("name", "Name"),
        ],
        ..Step::core("graph.make", "Graph", "Makes a graph.")
    }]
}

const PLAN: &str = r#"{"format":"mzflow/2","inputs":[],"steps":[{"id":"graph","step":"graph.make@1","params":{"ppm":7,"flag":false,"name":"a b'c"}}]}"#;

#[test]
fn the_line_has_one_set_per_value_that_is_not_the_default() {
    let plan = parse(PLAN.as_bytes()).unwrap();
    let l = line(&plan, &reg());
    assert!(l.starts_with("mzlab-cli flow run plan.json --in "));
    assert!(l.contains(" --set graph.ppm=7"));
    assert!(l.contains(" --set 'graph.name=a b'\\''c'"));
    assert!(!l.contains(" --set graph.flag=false"));
}

#[test]
fn a_line_gives_the_plan_the_cli_set_gives() {
    let plan = parse(PLAN.as_bytes()).unwrap();
    let l = line(&plan, &reg());
    let mut back = parse(PLAN.as_bytes()).unwrap();
    back.steps[0].params.clear();
    apply_sets(&mut back, &reg(), &sets_of(&l).unwrap()).unwrap();
    let mut one = parse(PLAN.as_bytes()).unwrap();
    one.steps[0].params.retain(|(n, _)| n != "flag");
    assert_eq!(plan_json(&back), plan_json(&one));
    // a changed value goes the same way as `--set graph.ppm=9`
    let mut a = parse(PLAN.as_bytes()).unwrap();
    apply_sets(
        &mut a,
        &reg(),
        &sets_of("mzlab-cli flow run plan.json --in x --set graph.ppm=9").unwrap(),
    )
    .unwrap();
    let mut b = parse(PLAN.as_bytes()).unwrap();
    apply_set(&mut b, &reg(), "graph.ppm=9").unwrap();
    assert_eq!(plan_json(&a), plan_json(&b));
}

#[test]
fn a_wrong_set_is_the_error_of_the_cli() {
    let mut plan = parse(PLAN.as_bytes()).unwrap();
    let e = apply_sets(&mut plan, &reg(), &["graph.ppm=x".into()]).unwrap_err();
    assert_eq!(e.key, "err.cli.set");
    assert!(e.params.contains(&(
        "message",
        "--set graph.ppm=x: x is not a value of ppm".into()
    )));
    assert_eq!(sets_of("ls -l").unwrap_err().key, "err.cli.line");
    assert_eq!(
        sets_of("mzlab-cli flow run p --bogus").unwrap_err().key,
        "err.cli.line"
    );
    assert_eq!(
        sets_of("mzlab-cli flow run p --set 'a").unwrap_err().key,
        "err.cli.line"
    );
}

#[test]
fn the_route_builds_the_line_and_applies_it() {
    let body = format!(r#"{{"plan":{PLAN},"registry":[]}}"#);
    assert!(route(&body, Some(&reg()))
        .unwrap()
        .contains("--set graph.ppm=7"));
    let body =
        format!(r#"{{"plan":{PLAN},"line":"mzlab-cli flow run plan.json --set graph.ppm=9"}}"#);
    assert!(route(&body, Some(&reg())).unwrap().contains("\"ppm\":9"));
}
