//! The rules of the CLI (`docs/agenti/cli.md`) as tests: every parameter of every step of `flow::steps()` (algorithms included) is reachable with
//! `--set` and lands in the canonical parameters; every step that claims `runs_in.native` has an executor, but for a closed list of known holes
//! that can only get shorter.

use mzlab_core::flow::{canonical, Instance, Plan};
use mzlab_core::json::Value;
use mzlab_core::step::{Kind, Param, Step};

use crate::flow::{executable, steps};
use crate::flow_run::apply_set;

/// Steps that say `native: true` and have no executor yet. A step leaves this list when its executor lands; nothing is added.
const BUCHI: &[&str] = &[
    // included in features.detect (pipeline::analyse links the MS2 in the same pass), not separable without changing the bytes
    "features.ms2_link",
    "rt.predict",
    "annot.regulatory",
    "annot.hazard",
];

fn default_num(p: &Param) -> Option<f64> {
    match p.default {
        Some(Value::Num(n)) => Some(n),
        _ => None,
    }
}

/// A value of `p` that is not its default, as the text of `--set` and as the value the plan must hold.
fn sample(p: &Param) -> (String, Value) {
    let num = |n: f64| (format!("{n}"), Value::Num(n));
    match &p.kind {
        Kind::Number => num(default_num(p).map_or(7.25, |d| d + 1.5)),
        Kind::Integer => num(default_num(p).map_or(7.0, |d| d + 1.0)),
        Kind::Bool => {
            let on = p.default != Some(Value::Bool(true));
            (on.to_string(), Value::Bool(on))
        }
        Kind::Text => ("a,b".into(), Value::Str("a,b".into())),
        Kind::Pair(_) => (
            "1,2".into(),
            Value::Arr(vec![Value::Num(1.0), Value::Num(2.0)]),
        ),
        Kind::Choice(list) => {
            let v = list
                .iter()
                .find(|v| Some(*v) != p.default.as_ref())
                .unwrap_or(&list[0]);
            let text = match v {
                Value::Str(s) => s.clone(),
                Value::Num(n) => format!("{n}"),
                _ => unreachable!("choices are strings or numbers"),
            };
            (text, v.clone())
        }
    }
}

/// A plan with one instance of `step` (id = the step id), the algorithm set by `--set` like a user would.
fn plan_of(step: &Step, reg: &[Step], algorithm: Option<&str>) -> Plan {
    let mut plan = Plan {
        steps: vec![Instance {
            id: step.id.into(),
            step: step.id.into(),
            version: step.version,
            algorithm: None,
            preset: None,
            label: None,
            ins: Vec::new(),
            params: Vec::new(),
        }],
        ..Plan::default()
    };
    if let Some(a) = algorithm {
        apply_set(&mut plan, reg, &format!("{}.algorithm={a}", step.id)).unwrap();
    }
    plan
}

#[test]
fn every_parameter_is_reachable_with_set() {
    let reg = steps();
    let mut checked = 0;
    for step in &reg {
        let contexts: Vec<Option<&str>> = std::iter::once(None)
            .chain(step.algorithms.iter().map(|a| Some(a.id)))
            .collect();
        for alg in contexts {
            let own = step.params.iter();
            let of_alg = step
                .algorithms
                .iter()
                .filter(|a| alg.is_some_and(|id| a.id == id))
                .flat_map(|a| a.params.iter());
            for p in own.chain(of_alg) {
                let (text, want) = sample(p);
                let mut plan = plan_of(step, &reg, alg);
                let set = format!("{}.{}={text}", step.id, p.name);
                apply_set(&mut plan, &reg, &set).unwrap_or_else(|e| panic!("{e}"));
                let got = canonical(&plan.steps[0], step).unwrap().params;
                let found = got.iter().find(|(n, _)| n == p.name).map(|(_, v)| v);
                assert_eq!(found, Some(&want), "{set} (algorithm {alg:?})");
                checked += 1;
            }
        }
    }
    assert!(checked > 100, "only {checked} parameters were checked");
}

#[test]
fn algorithm_and_preset_are_reachable_with_set() {
    let reg = steps();
    for step in &reg {
        for a in &step.algorithms {
            let mut plan = plan_of(step, &reg, None);
            apply_set(&mut plan, &reg, &format!("{}.algorithm={}", step.id, a.id)).unwrap();
            let got = canonical(&plan.steps[0], step).unwrap().algorithm;
            assert_eq!(got.map(|g| g.0), Some(a.id.to_string()), "{}", step.id);
        }
        for pr in &step.presets {
            let mut plan = plan_of(step, &reg, None);
            apply_set(&mut plan, &reg, &format!("{}.preset={}", step.id, pr.id)).unwrap();
            let got = canonical(&plan.steps[0], step).unwrap().preset;
            assert_eq!(got.as_deref(), Some(pr.id), "{}", step.id);
        }
    }
}

#[test]
fn every_native_step_has_an_executor_but_the_known_holes() {
    let reg = steps();
    let ids: Vec<&str> = reg.iter().map(|s| s.id).collect();
    for hole in BUCHI {
        assert!(
            ids.contains(hole),
            "{hole} is not a step: drop it from BUCHI"
        );
        assert!(
            !executable(hole),
            "{hole} has an executor now: drop it from BUCHI"
        );
    }
    for s in reg.iter().filter(|s| s.runs_in.native) {
        assert!(
            executable(s.id) || BUCHI.contains(&s.id),
            "{} says native: true and has no executor (add one; BUCHI never grows)",
            s.id
        );
    }
    for s in reg.iter().filter(|s| !s.runs_in.native) {
        assert!(
            !executable(s.id),
            "{} has an executor but native: false",
            s.id
        );
    }
}
