//! The lineage of a feature job of a plan: the `(step, written params)` layers `flow run` carries with its output (`mzlab-cli` `flow_base.rs`), as the recipe text of
//! the browser pool (`job_layers_text`) and as the lineage the meta of a job reads (`job_lineage_text`: the layers before it, its own, its canonical values).
//! Here, with no feature code, so the `.wasm` that already holds the planner (`rotte`) answers them and the feature `.wasm` reads only the text.

use super::sched::{JobId, Sched};
use super::{parse, value_text, Ctx, Instance, Job, Plan, Ref};
use crate::error::{Error, Result};
use crate::json::{Out, Value};
use crate::step::Step;

/// The steps that carry a recipe layer.
pub const LAYERED: [&str; 3] = ["features.detect", "features.align", "features.gapfill"];

/// The parameters one feature step writes: its id and the (name, value) pairs.
pub type Layer = (String, Vec<(String, Value)>);

/// What the plan writes for `job`: the algorithm it chose (feature steps only), the values of its preset, then its own parameters (descriptor names).
pub fn written(job: &Job) -> Vec<(String, Value)> {
    let mut v: Vec<(String, Value)> = Vec::new();
    if let Some(a) = job
        .instance
        .algorithm
        .as_deref()
        .filter(|_| LAYERED.contains(&job.step.id))
    {
        v.push(("algorithm".to_string(), Value::Str(a.into())));
    }
    let preset = job.instance.preset.as_deref();
    let presets = job.step.presets.iter().filter(|p| Some(p.id) == preset);
    for (k, x) in presets.flat_map(|p| p.params.iter()) {
        v.push((k.to_string(), x.clone()));
    }
    for (k, x) in &job.instance.params {
        v.retain(|(n, _)| n != k);
        v.push((k.clone(), x.clone()));
    }
    v
}

/// The layers `flow run` carries with the output of `inst` (its lineage, `flow_base.rs`): a detection its own; an alignment those of its first source, then its
/// own; a gap filling those of its table, then its own; any other step none. Only the line of `inst` counts, so a second branch of the plan never leaks in.
fn carried(ctx: &Ctx, plan: &Plan, inst: &Instance, depth: usize) -> Vec<Layer> {
    if !LAYERED.contains(&inst.step.as_str()) || depth > plan.steps.len() {
        return Vec::new();
    }
    let port = match inst.step.as_str() {
        "features.align" => "peaks",
        "features.gapfill" => "table",
        _ => "",
    };
    let first = inst
        .ins
        .iter()
        .filter(|(p, _)| p == port)
        .flat_map(|(_, refs)| refs)
        .find_map(|r| match r {
            Ref::Out { instance, .. } => plan.steps.iter().find(|s| s.id == *instance),
            Ref::File(_) => None,
        });
    let mut layers = first.map_or_else(Vec::new, |s| carried(ctx, plan, s, depth + 1));
    let job = ctx.job(&JobId::new(&inst.id, None), Vec::new());
    layers.push((job.step.id.to_string(), written(&job)));
    layers
}

/// The plan of `plan_text` with its scheduler context, and `instance` of it; `err.flow.lineage` when the plan has no such instance.
fn with_plan<T>(
    plan_text: &[u8],
    registry: &[Step],
    instance: &str,
    f: impl FnOnce(&Ctx, &Plan, &Instance) -> T,
) -> Result<T> {
    let plan = parse(plan_text)?;
    let sched = Sched::new(&plan, registry, env!("CARGO_PKG_VERSION"), &[]);
    let ctx = Ctx::new(&plan, registry, &sched)?;
    let inst = plan
        .steps
        .iter()
        .find(|s| s.id == instance)
        .ok_or_else(|| Error::new("err.flow.lineage"))?;
    Ok(f(&ctx, &plan, inst))
}

/// The recipe text the browser pool runs `instance` with (`{"fill": bool, "layers": [{"step", "params"}]}`), the recipe `flow run` builds for that job. The
/// browser keeps one table per alignment, aligned and filled in one call: with `filled`, an alignment a gap filling reads takes the recipe of the first such filling.
pub fn job_layers_text(
    plan_text: &[u8],
    registry: &[Step],
    instance: &str,
    filled: bool,
) -> Result<String> {
    with_plan(plan_text, registry, instance, |ctx, plan, inst| {
        let reads = |s: &&Instance| {
            s.step == "features.gapfill"
                && s.ins.iter().any(|(p, refs)| {
                    p == "table"
                        && refs
                            .iter()
                            .any(|r| matches!(r, Ref::Out { instance, .. } if *instance == inst.id))
                })
        };
        let target = match inst.step.as_str() {
            "features.align" if filled => plan.steps.iter().find(reads).unwrap_or(inst),
            _ => inst,
        };
        let mut o = Out::default();
        o.raw("{\"fill\":")
            .raw(if target.step == "features.gapfill" {
                "true"
            } else {
                "false"
            })
            .raw(",\"layers\":[");
        for (i, (step, params)) in carried(ctx, plan, target, 0).iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" })
                .raw("{\"step\":")
                .str(step);
            o.raw(",\"params\":")
                .raw(&value_text(&Value::Obj(params.clone())))
                .raw("}");
        }
        o.raw("]}");
        o.0
    })
}

/// `(name, value)` pairs as a JSON object in their own order: the order the plan writes them is the order they are applied.
fn pairs(o: &mut Out, kv: &[(String, Value)]) {
    o.raw("{");
    for (i, (k, v)) in kv.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .str(k)
            .raw(":")
            .raw(&value_text(v));
    }
    o.raw("}");
}

/// What the meta of `instance` reads of the plan (`mzlab_ops` `job_meta`): `{"step", "params", "layers": [{"step", "params"}], "resolved", "noresult"}`, its step,
/// what the plan writes for it, the layers of its line before its own (the recipe `flow run` rebuilds from what it reads), its canonical values and the names
/// that do not change the result. The pairs keep the order of the plan.
pub fn job_lineage_text(plan_text: &[u8], registry: &[Step], instance: &str) -> Result<String> {
    with_plan(plan_text, registry, instance, |ctx, plan, inst| {
        let job = ctx.job(&JobId::new(instance, None), Vec::new());
        let mut up = carried(ctx, plan, inst, 0);
        up.pop();
        let mut o = Out::default();
        o.raw("{\"step\":").str(job.step.id).raw(",\"params\":");
        pairs(&mut o, &written(&job));
        o.raw(",\"layers\":[");
        for (i, (step, params)) in up.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" })
                .raw("{\"step\":")
                .str(step)
                .raw(",\"params\":");
            pairs(&mut o, params);
            o.raw("}");
        }
        o.raw("],\"resolved\":");
        pairs(&mut o, &job.resolved.params);
        o.raw(",\"noresult\":[");
        for (i, n) in job.resolved.noresult.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).str(n);
        }
        o.raw("]}");
        o.0
    })
}

/// A lineage text of `job_lineage_text` read back.
pub struct Lineage {
    pub step: String,
    pub params: Vec<(String, Value)>,
    pub layers: Vec<Layer>,
    pub resolved: Vec<(String, Value)>,
    pub noresult: Vec<String>,
}

/// `job_lineage_text` read back; anything else is `err.flow.lineage`.
pub fn lineage_from_text(text: &str) -> Result<Lineage> {
    let bad = || Error::new("err.flow.lineage");
    let v = crate::json::parse(text.as_bytes()).map_err(|_| bad())?;
    let obj = |x: Option<&Value>| match x {
        Some(Value::Obj(o)) => Ok(o.clone()),
        _ => Err(bad()),
    };
    let layer = |l: &Value| -> Result<Layer> {
        let step = l.get("step").and_then(Value::str).ok_or_else(bad)?;
        Ok((step.to_string(), obj(l.get("params"))?))
    };
    let list = |k: &str| match v.get(k) {
        Some(Value::Arr(a)) => Ok(a.clone()),
        _ => Err(bad()),
    };
    Ok(Lineage {
        step: v
            .get("step")
            .and_then(Value::str)
            .ok_or_else(bad)?
            .to_string(),
        params: obj(v.get("params"))?,
        layers: list("layers")?.iter().map(layer).collect::<Result<_>>()?,
        resolved: obj(v.get("resolved"))?,
        noresult: list("noresult")?
            .iter()
            .map(|n| n.str().map(str::to_string).ok_or_else(bad))
            .collect::<Result<_>>()?,
    })
}
