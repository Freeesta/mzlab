use super::super::parse;
use super::*;

fn plan(steps: &str) -> Plan {
    parse(format!(r#"{{"steps":[{steps}]}}"#).as_bytes()).unwrap()
}
fn step(id: &str, from: &[&str]) -> String {
    let ins: Vec<String> = from.iter().map(|f| format!("\"{f}.out\"")).collect();
    format!(
        r#"{{"id":"{id}","step":"s.x@1","in":{{"p":[{}]}}}}"#,
        ins.join(",")
    )
}
fn ids(p: &Plan, order: &[usize]) -> String {
    order
        .iter()
        .map(|&i| p.steps[i].id.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn ties_follow_the_order_of_the_file() {
    // c and b both wait only for a; c comes first in the file; d waits for both
    let p = plan(
        &[
            step("a", &[]),
            step("c", &["a"]),
            step("b", &["a"]),
            step("d", &["b", "c"]),
        ]
        .join(","),
    );
    assert_eq!(ids(&p, &plan_order(&p).unwrap()), "a c b d");
    // a step written before the one it reads from still runs after it
    let q = plan(&[step("late", &["early"]), step("early", &[])].join(","));
    assert_eq!(ids(&q, &plan_order(&q).unwrap()), "early late");
}

#[test]
fn levels_group_the_independent_instances() {
    let p = plan(
        &[
            step("a", &[]),
            step("b", &["a"]),
            step("c", &["a"]),
            step("d", &["b", "c"]),
            step("e", &[]),
        ]
        .join(","),
    );
    let l = levels(&p).unwrap();
    let named: Vec<String> = l.iter().map(|w| ids(&p, w)).collect();
    assert_eq!(named, ["a e", "b c", "d"]);
}

#[test]
fn a_cycle_is_an_error_and_only_its_members_are_named() {
    let p = plan(
        &[
            step("a", &[]),
            step("b", &["a", "d"]),
            step("c", &["b"]),
            step("d", &["c"]),
            step("e", &["d"]),
        ]
        .join(","),
    );
    assert_eq!(plan_order(&p).unwrap_err().key, "err.flow.cycle");
    assert_eq!(ids(&p, &cyclic(&p)), "b c d");
    let own = plan(&step("x", &["x"]));
    assert_eq!(ids(&own, &cyclic(&own)), "x");
}
