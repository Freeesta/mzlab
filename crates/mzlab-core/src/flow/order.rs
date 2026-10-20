//! Execution order of a plan: Kahn's topological sort with ties broken by the order of `steps` in the file, so the same file always gives the same plan.
//! `levels` groups the instances that can run together (every ancestor done); the order inside a level is the order of the file.

use super::Plan;
use crate::error::{Error, Result};

/// For each instance, the positions of the instances it reads from (once each; references to unknown instances are `check`'s business).
pub(super) fn dependencies(plan: &Plan) -> Vec<Vec<usize>> {
    plan.steps
        .iter()
        .map(|s| {
            let mut d: Vec<usize> = s
                .sources()
                .iter()
                .filter_map(|id| plan.steps.iter().position(|x| x.id == *id))
                .collect();
            crate::sortc::slice_ord(&mut d);
            d
        })
        .collect()
}

/// Positions of the instances on a cycle (or between two cycles): what stays after peeling every instance that nothing waits for and every one that
/// waits for nothing.
pub(super) fn cyclic(plan: &Plan) -> Vec<usize> {
    let deps = dependencies(plan);
    let mut alive = vec![true; deps.len()];
    loop {
        let mut changed = false;
        for i in 0..deps.len() {
            if !alive[i] {
                continue;
            }
            let waits = deps[i].iter().any(|&d| alive[d]);
            let waited = (0..deps.len()).any(|j| alive[j] && deps[j].contains(&i));
            if !waits || !waited {
                alive[i] = false;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    (0..deps.len()).filter(|&i| alive[i]).collect()
}

fn cycle_error(plan: &Plan, deps: &[Vec<usize>], done: &[bool]) -> Error {
    let first = (0..deps.len()).find(|&i| !done[i]).unwrap_or(0);
    Error::new("err.flow.cycle").with("instance", &plan.steps[first].id)
}

/// The order in which to run the instances (positions in `plan.steps`).
pub fn plan_order(plan: &Plan) -> Result<Vec<usize>> {
    let deps = dependencies(plan);
    let mut done = vec![false; deps.len()];
    let mut order = Vec::with_capacity(deps.len());
    while order.len() < deps.len() {
        let next = (0..deps.len()).find(|&i| !done[i] && deps[i].iter().all(|&d| done[d]));
        let Some(i) = next else {
            return Err(cycle_error(plan, &deps, &done));
        };
        done[i] = true;
        order.push(i);
    }
    Ok(order)
}

/// Waves of instances that can run together: an instance is in the wave after the last of its ancestors.
pub fn levels(plan: &Plan) -> Result<Vec<Vec<usize>>> {
    let deps = dependencies(plan);
    let mut level = vec![0usize; deps.len()];
    for i in plan_order(plan)? {
        level[i] = deps[i].iter().map(|&d| level[d] + 1).max().unwrap_or(0);
    }
    let top = level.iter().max().map_or(0, |m| m + 1);
    Ok((0..top)
        .map(|w| (0..deps.len()).filter(|&i| level[i] == w).collect())
        .collect())
}

#[cfg(test)]
mod tests {
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
}
