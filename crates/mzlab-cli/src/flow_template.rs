//! The plan of `mzlab-cli flow steps --id STEP[@V] --template`: a one-step `mzflow/2` plan for a step of this binary, written by the same writer as the Flow page,
//! with a file input for every port that needs one and every parameter at its default, so an agent starts from a plan the checker reads.

use mzlab_core::flow::{plan_json, FileInput, Instance, Plan, Ref};
use mzlab_core::json::{self, Value};
use mzlab_core::step::json::step_json;
use mzlab_core::step::{DataType, Step};

use crate::Outcome;

/// The extension a file of this type usually has: only a name for the placeholder input.
fn extension(ty: DataType) -> &'static str {
    match ty {
        DataType::Raw => "raw",
        DataType::Mzml => "mzML",
        DataType::Mzidx => "mzidx",
        DataType::Table => "mzfeat",
        DataType::Samples | DataType::Design | DataType::Transform => "json",
        _ => "csv",
    }
}

/// The defaults of the step's parameters, in the order the descriptor lists them (a parameter with no default is left out).
fn defaults(step: &Step) -> Vec<(String, Value)> {
    let Ok(doc) = json::parse(step_json(step).as_bytes()) else {
        return Vec::new();
    };
    match doc.get("params").and_then(|p| p.get("properties")) {
        Some(Value::Obj(props)) => props
            .iter()
            .filter_map(|(k, v)| v.get("default").map(|d| (k.clone(), d.clone())))
            .collect(),
        _ => Vec::new(),
    }
}

/// The plan of one instance of `step` named `id`.
fn plan_of(step: &Step, id: &str) -> Plan {
    let mut plan = Plan {
        format: "mzflow/2".into(),
        ..Plan::default()
    };
    let mut ins = Vec::new();
    for port in step.ins.iter().filter(|p| !p.optional) {
        let name = format!("{}.{}", port.name, extension(port.ty));
        plan.inputs.push(FileInput {
            name: name.clone(),
            ty: (port.ty == DataType::Raw).then(|| "raw".to_string()),
            ..FileInput::default()
        });
        ins.push((port.name.to_string(), vec![Ref::File(name)]));
    }
    plan.steps.push(Instance {
        id: id.into(),
        step: step.id.into(),
        version: step.version,
        algorithm: None,
        preset: None,
        label: None,
        ins,
        params: defaults(step),
    });
    plan
}

/// The plan of `step` as the answer of `flow steps --template`: the instance is named after the step id (after its last dot).
pub(crate) fn template(step: &Step) -> Outcome {
    let id = step.id.rsplit('.').next().unwrap_or(step.id);
    Outcome::ok(plan_json(&plan_of(step, id)).into_bytes())
}

#[cfg(test)]
#[path = "flow_template_tests.rs"]
mod tests;
