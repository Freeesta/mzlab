//! `mzlab-cli flow template STEP[@V] [--id NAME]`: a one-step `mzflow/2` plan for a step of this binary, written by the same writer as the Flow page,
//! with a file input for every port that needs one and every parameter at its default, so an agent starts from a plan the checker reads.

use mzlab_core::error::Error;
use mzlab_core::flow::{plan_json, FileInput, Instance, Plan, Ref};
use mzlab_core::json::{self, Value};
use mzlab_core::step::json::step_json;
use mzlab_core::step::{DataType, Step};

use crate::flow::steps;
use crate::flow_json::fail;
use crate::Outcome;

const HELP: &str = "mzlab-cli flow template: a plan with one step, to edit.

usage: mzlab-cli flow template STEP[@VERSION] [--id NAME] [--json]

  STEP[@V]    a step of `flow steps` (without @V the highest version)
  --id NAME   the instance name in the plan (default: the step id after its last dot)
  --json      accepted; the answer is always the mzflow/2 plan on stdout

The plan has a file input per required port (named after the port, edit the names) and every parameter at its default (`flow steps --json --id STEP` has
their types, limits and meaning); a port fed by another step is a `file:` input here, to be replaced by `INSTANCE.PORT`. `flow check` reads it as written.

exit codes: 0 done, 2 usage, 5 unknown step (err.flow.step.unknown)";

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

/// `mzlab-cli flow template ARGS...` (the arguments after `template`).
pub(crate) fn run(args: Vec<String>) -> Outcome {
    let usage = |msg: &str| {
        let err = Error::new("err.cli.usage").with("message", msg);
        fail(&err, msg, "mzlab-cli flow template --help")
    };
    let (mut want, mut id) = (None, None);
    let mut it = args.into_iter();
    while let Some(x) = it.next() {
        match x.as_str() {
            "--help" | "-h" => return Outcome::ok(format!("{HELP}\n").into_bytes()),
            "--json" => {}
            "--id" => match it.next() {
                Some(v) => id = Some(v),
                None => return usage("--id needs a name"),
            },
            s if s.starts_with('-') => return usage(&format!("unknown option {s}")),
            s if want.is_none() => want = Some(s.to_string()),
            s => return usage(&format!("unexpected argument {s}")),
        }
    }
    let Some(want) = want else {
        return usage("flow template needs a step id");
    };
    let (name, version) = want
        .split_once('@')
        .map_or((want.as_str(), None), |(n, v)| (n, v.parse::<u32>().ok()));
    let reg = steps();
    let Some(step) = reg
        .iter()
        .filter(|s| s.id == name && version.is_none_or(|v| s.version == v))
        .max_by_key(|s| s.version)
    else {
        let err = Error::new("err.flow.step.unknown").with("step", &want);
        return fail(
            &err,
            &format!("unknown step {want}"),
            "mzlab-cli flow steps lists the steps",
        );
    };
    let id = id.unwrap_or_else(|| step.id.rsplit('.').next().unwrap_or(step.id).to_string());
    if !mzlab_core::flow::instance_id_ok(&id) {
        return usage(&format!(
            "--id {id} is not a plain name (letters, digits, _ . -)"
        ));
    }
    Outcome::ok(plan_json(&plan_of(step, &id)).into_bytes())
}

#[cfg(test)]
#[path = "flow_template_tests.rs"]
mod tests;
