//! `--set INSTANCE.NAME=VALUE` of `mzlab-cli flow run` and `convert`: the instance a name points to and the value read with the type of the descriptor.
//! Used by `flow_run.rs`, `convert.rs` and the rule tests.

use mzlab_core::flow::Plan;
use mzlab_core::json::Value;
use mzlab_core::step::{Kind, Step};

/// The value of `text` read with the type of `kind`.
fn typed(kind: &Kind, text: &str) -> Option<Value> {
    let num = |s: &str| s.trim().parse::<f64>().ok().map(Value::Num);
    match kind {
        Kind::Number | Kind::Integer => num(text),
        Kind::Bool => match text {
            "true" => Some(Value::Bool(true)),
            "false" => Some(Value::Bool(false)),
            _ => None,
        },
        Kind::Text => Some(Value::Str(text.into())),
        Kind::Pair(_) => {
            let (a, b) = text.split_once(',')?;
            Some(Value::Arr(vec![num(a)?, num(b)?]))
        }
        Kind::Choice(list) => {
            let as_num = list.iter().any(|v| matches!(v, Value::Num(_)));
            if as_num {
                num(text)
            } else {
                Some(Value::Str(text.into()))
            }
        }
    }
}

/// The instance `who` names: its id, or the id of a step the plan has once.
fn instance_index(plan: &Plan, who: &str) -> Result<usize, String> {
    if let Some(k) = plan.steps.iter().position(|s| s.id == who) {
        return Ok(k);
    }
    let of_step: Vec<usize> = (0..plan.steps.len())
        .filter(|&k| plan.steps[k].step == who)
        .collect();
    match of_step.as_slice() {
        [k] => Ok(*k),
        [] => {
            let ids: Vec<&str> = plan.steps.iter().map(|s| s.id.as_str()).collect();
            Err(format!(
                "--set {who}: no instance or step of that name (instances: {})",
                ids.join(", ")
            ))
        }
        _ => Err(format!(
            "--set {who}: the plan has more than one instance of that step; use the instance id"
        )),
    }
}

/// Applies one `--set` (`INSTANCE.NAME=VALUE`) to `plan`, its value typed by the descriptor.
pub(crate) fn apply_set(plan: &mut Plan, reg: &[Step], set: &str) -> Result<(), String> {
    let bad = || format!("--set {set}: expected INSTANCE.NAME=VALUE");
    let (lhs, text) = set.split_once('=').ok_or_else(bad)?;
    let (who, name) = lhs.rsplit_once('.').ok_or_else(bad)?;
    let k = instance_index(plan, who)?;
    let inst = &mut plan.steps[k];
    match name {
        "algorithm" => inst.algorithm = Some(text.into()),
        "preset" => inst.preset = Some(text.into()),
        _ => {
            let step = reg
                .iter()
                .find(|s| s.id == inst.step && s.version == inst.version);
            let step = step.ok_or(format!("--set {set}: unknown step {}", inst.step_ref()))?;
            let alg = inst.algorithm.as_deref().or(step.default_algorithm);
            let param = step
                .param(name)
                .or_else(|| alg.and_then(|a| step.algorithm_param(a, name)));
            let Some(param) = param else {
                let own = step.params.iter().map(|p| p.name);
                let of_alg = step
                    .algorithms
                    .iter()
                    .filter(|a| Some(a.id) == alg)
                    .flat_map(|a| a.params.iter().map(|p| p.name));
                let names: Vec<&str> = own.chain(of_alg).collect();
                return Err(format!(
                    "--set {set}: {} has no parameter {name} (parameters: {})",
                    inst.id,
                    names.join(", ")
                ));
            };
            let v = typed(&param.kind, text)
                .ok_or(format!("--set {set}: {text} is not a value of {name}"))?;
            inst.params.retain(|(n, _)| n != name);
            inst.params.push((name.into(), v));
        }
    }
    Ok(())
}
