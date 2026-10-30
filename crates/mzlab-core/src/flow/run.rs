//! Execution of a checked plan: the jobs come from the live scheduler (`flow::sched`) and go to `workers` threads as soon as their inputs are made, the
//! outputs are gathered in the order of the plan inputs so that the record does not depend on who finished first. A `Runner` runs one job; the library has
//! no files and no clock.
//! An error stops only the descendants of the instance that failed: its activity carries the error, the descendants are listed in `Run::skipped`.

use super::canon::{canonical, Resolved};
use super::check::{check, Platform};
use super::order::levels;
use super::prov::{Activity, ErrorRecord, Mem};
use super::{FileInput, Instance, Plan, Ref};
use crate::error::{Error, Result};
use crate::flow::sched::JobId;
use crate::json::Value;
use crate::step::{DataType, Step};

#[path = "run_sched.rs"]
mod live;

pub use live::{file_roots, work, Core, Ctx, Hooks, Results};

#[path = "run_web.rs"]
mod web;
pub use web::{record_jobs, route as record_route};

/// A file the plan reads or a step made, identified by its content.
#[derive(Debug, Clone, PartialEq)]
pub struct Artifact {
    pub sha256: String,
    pub bytes: u64,
    pub ty: DataType,
    /// File name for a file of the plan, free text for what a step made.
    pub label: Option<String>,
}

/// One run of a step: a whole instance, or one item of a `per_item` instance (`item`).
pub struct Job<'a> {
    pub instance: &'a Instance,
    pub step: &'a Step,
    pub resolved: &'a Resolved,
    /// For every port written in the file, what it reads; the driving port of a `per_item` job holds its one item.
    pub inputs: Vec<(String, Vec<Artifact>)>,
    pub item: Option<usize>,
    /// For a `per_item` job: the name of the plan input that drives it (never an index).
    pub item_name: Option<String>,
    /// `item_name` as a file-name root, the same for the same name and different for every input of the plan.
    pub item_root: Option<String>,
}

/// What a job made and measured.
#[derive(Debug, Clone, Default)]
pub struct Outcome {
    /// Output port and artifact; a `many` port can appear more than once.
    pub outputs: Vec<(String, Artifact)>,
    /// Parameter, value and proof of every parameter the step measured.
    pub estimates: Vec<(String, Value, Value)>,
    pub ms: f64,
    pub heap_peak_bytes: Option<u64>,
    /// Memory the step asked for and the way it took (steps that estimate it).
    pub mem: Option<Mem>,
    /// `warn.*` keys of what the step noticed and went on with.
    pub warnings: Vec<String>,
    /// The preset the step ran with when the files decided it (`features.detect` on unit-resolution files runs the `lr` preset); the record names it instead of the default one.
    pub preset: Option<String>,
    /// The output came from the cache key (`cache_key`), the step did not run.
    pub cached: bool,
    /// What a per-file step says of its file for the record of the run (`{"via": "centroided"|"passthrough", ...}`).
    pub file: Option<Value>,
}

/// The owner of the files and the clock: runs the step of a job. It must not read the clock or unseeded randomness to decide a result.
pub trait Runner: Sync {
    fn run(&self, job: &Job) -> Result<Outcome>;
}

/// The record of a run: what was read and made, what each instance did, which instances were not started.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Run {
    pub entities: Vec<(String, Artifact)>,
    pub activities: Vec<Activity>,
    pub skipped: Vec<String>,
}

/// An artifact with the key of its entity.
type Keyed = (String, Artifact);
/// Output port and what it holds, keyed.
type Made = Vec<(String, Vec<Keyed>)>;

/// A file of the plan as an artifact, typed by the plan or else by the port that reads it.
fn file_artifact(step: &Step, port: &str, f: &FileInput) -> Artifact {
    let want = step.ins.iter().find(|p| p.name == port).map(|p| p.ty);
    let ty =
        f.ty.as_deref()
            .and_then(DataType::from_name)
            .or(want)
            .unwrap_or(DataType::Raw);
    Artifact {
        sha256: f.sha256.clone(),
        bytes: f.bytes.unwrap_or(0),
        ty,
        label: Some(f.name.clone()),
    }
}

/// What the instance reads on each port, keyed; `done` holds what the instances before it made.
fn read(plan: &Plan, step: &Step, inst: &Instance, done: &[Option<Made>]) -> Made {
    let mut all = Vec::new();
    for (port, refs) in &inst.ins {
        let mut got: Vec<Keyed> = Vec::new();
        for r in refs {
            match r {
                Ref::File(name) => {
                    if let Some(f) = plan.input(name) {
                        got.push((format!("file:{name}"), file_artifact(step, port, f)));
                    }
                }
                Ref::Out {
                    instance,
                    port: out,
                } => {
                    let src = plan.steps.iter().position(|x| x.id == *instance);
                    let made = src.and_then(|k| done[k].as_ref());
                    got.extend(
                        made.into_iter()
                            .flatten()
                            .filter(|(p, _)| p == out)
                            .flat_map(|(_, v)| v.clone()),
                    );
                }
            }
        }
        all.push((port.clone(), got));
    }
    all
}

fn references(step: &Step, algorithm: Option<&str>) -> Vec<String> {
    let own = algorithm.and_then(|id| step.algorithms.iter().find(|a| a.id == id));
    let mut keys: Vec<String> = Vec::new();
    for r in step
        .references
        .iter()
        .chain(own.into_iter().flat_map(|a| a.references.iter()))
    {
        if !keys.iter().any(|k| k == r.citation_key) {
            keys.push(r.citation_key.to_string());
        }
    }
    keys
}

/// What the outcomes made, port by port in declaration order, and the keys of all of it. `names` holds the plan input of each outcome (`per_item` jobs): a
/// `many` output is keyed by it, else by its position.
fn gather(
    inst: &Instance,
    step: &Step,
    outcomes: &[Outcome],
    names: &[Option<String>],
) -> (Made, Vec<String>) {
    let mut made: Made = Vec::new();
    let mut generated = Vec::new();
    for port in &step.outs {
        let mut list: Vec<Keyed> = Vec::new();
        for (o, item) in outcomes.iter().zip(names) {
            let mine = o.outputs.iter().filter(|(n, _)| n == port.name);
            let several = mine.clone().count() > 1;
            for (k, (name, a)) in mine.enumerate() {
                let key = if port.many {
                    match item {
                        Some(i) if several => format!("{}.{name}[{i}#{k}]", inst.id),
                        Some(i) => format!("{}.{name}[{i}]", inst.id),
                        None => format!("{}.{name}[{}]", inst.id, list.len()),
                    }
                } else {
                    format!("{}.{name}", inst.id)
                };
                list.push((key, a.clone()));
            }
        }
        generated.extend(list.iter().map(|k| k.0.clone()));
        made.push((port.name.to_string(), list));
    }
    (made, generated)
}

/// Why an instance stopped, as the record writes it.
fn error_record(e: &Error) -> ErrorRecord {
    let params = e.params.iter().map(|(k, v)| (k.to_string(), v.clone()));
    ErrorRecord {
        error_key: e.key.to_string(),
        params: params.collect(),
    }
}

/// The activity of an instance and, when it did not fail, what it made.
fn settle(
    inst: &Instance,
    step: &Step,
    resolved: &Resolved,
    used: Vec<String>,
    results: Vec<(Option<String>, Result<Outcome>)>,
) -> (Activity, Option<Made>) {
    let algorithm = resolved
        .algorithm
        .as_ref()
        .map(|(id, v)| format!("{id}@{v}"));
    let mut act = Activity {
        instance: inst.id.clone(),
        step: inst.step_ref(),
        references: references(step, resolved.algorithm.as_ref().map(|a| a.0.as_str())),
        algorithm,
        preset: resolved.preset.clone(),
        params: resolved.params.clone(),
        params_sha256: resolved.params_sha256(),
        estimates: Vec::new(),
        files: Vec::new(),
        used,
        generated: Vec::new(),
        ms: 0.0,
        heap_peak_bytes: None,
        mem: None,
        warnings: Vec::new(),
        error: None,
        cached: !results.is_empty(),
    };
    let mut outcomes = Vec::new();
    let mut names = Vec::new();
    for (item, r) in results {
        match r {
            Ok(o) => {
                outcomes.push(o);
                names.push(item);
            }
            Err(e) => {
                act.cached = false;
                act.error = Some(error_record(&e));
                return (act, None);
            }
        }
    }
    let (made, generated) = gather(inst, step, &outcomes, &names);
    act.generated = generated;
    act.files = file_notes(&outcomes, &names);
    for o in &outcomes {
        act.ms += o.ms;
        act.cached &= o.cached;
        if o.preset.is_some() {
            act.preset = o.preset.clone();
        }
        act.heap_peak_bytes = act.heap_peak_bytes.max(o.heap_peak_bytes);
        act.mem = merged(act.mem.take(), o.mem.clone());
        for w in &o.warnings {
            if !act.warnings.contains(w) {
                act.warnings.push(w.clone());
            }
        }
        for (name, value, proof) in &o.estimates {
            if !act.estimates.iter().any(|e| e.0 == *name) {
                act.estimates.push((name.clone(), proof.clone()));
                match act.params.iter_mut().find(|p| p.0 == *name) {
                    Some(p) => p.1 = value.clone(),
                    None => act.params.push((name.clone(), value.clone())),
                }
            }
        }
    }
    crate::sortc::slice_by(&mut act.params, |a, b| a.0.cmp(&b.0));
    (act, Some(made))
}

/// The memory of two items of a per-item step: the larger estimate, and `slabs` if any item took it.
fn merged(a: Option<Mem>, b: Option<Mem>) -> Option<Mem> {
    match (a, b) {
        (Some(a), Some(b)) => Some(Mem {
            mb: a.mb.max(b.mb),
            via: if a.via == "slabs" || b.via == "slabs" {
                "slabs"
            } else {
                b.via
            },
        }),
        (a, b) => a.or(b),
    }
}

/// What each per-file job said of its file, by the name of the input (a job with no name says nothing under one).
fn file_notes(outcomes: &[Outcome], names: &[Option<String>]) -> Vec<(String, Value)> {
    let said = outcomes.iter().zip(names);
    said.filter_map(|(o, n)| {
        o.file
            .as_ref()
            .map(|v| (n.clone().unwrap_or_default(), v.clone()))
    })
    .collect()
}

/// Runs `plan` with `workers` threads (1 = in order; the record is the same for any number). The plan is checked first; an instance that fails stops its
/// descendants only. The jobs run in the order of the scheduler, the record is written afterwards in the order of the levels of the plan.
pub fn execute(plan: &Plan, registry: &[Step], runner: &dyn Runner, workers: usize) -> Result<Run> {
    let problems = check(plan, registry, Platform::current());
    if let Some(p) = problems.first() {
        return Err(Error::new("err.flow.check")
            .with("count", problems.len())
            .with("instance", &p.instance)
            .with("error_key", p.error_key));
    }
    let (ids, results) = live::run_jobs(plan, registry, runner, workers)?;
    record(plan, registry, &ids, results)
}

/// The record of a run: what the jobs `results` made, written level by level in the order of the plan, so it does not depend on who finished first.
/// `ids` are the jobs of each instance, in the order of its items.
pub fn record(
    plan: &Plan,
    registry: &[Step],
    ids: &[Vec<JobId>],
    mut results: Results,
) -> Result<Run> {
    let waves = levels(plan)?;
    let mut run = Run::default();
    let mut done: Vec<Option<Made>> = vec![None; plan.steps.len()];
    for wave in waves {
        // resolve and read in file order, settle in file order
        let mut ready = Vec::new();
        for k in wave {
            let inst = &plan.steps[k];
            let step = registry
                .iter()
                .find(|s| s.id == inst.step && s.version == inst.version)
                .ok_or_else(|| Error::new("err.flow.step.unknown"))?;
            let sources: Vec<usize> = inst
                .sources()
                .iter()
                .filter_map(|id| plan.steps.iter().position(|x| x.id == *id))
                .collect();
            if sources.iter().any(|&s| done[s].is_none()) {
                run.skipped.push(inst.id.clone());
                continue;
            }
            let resolved = canonical(inst, step)?;
            ready.push((k, step, resolved, read(plan, step, inst, &done)));
        }
        for (k, step, resolved, rd) in &ready {
            let inst = &plan.steps[*k];
            let mut used: Vec<String> = Vec::new();
            for (key, a) in rd.iter().flat_map(|(_, v)| v) {
                if !run.entities.iter().any(|e| e.0 == *key) {
                    run.entities.push((key.clone(), a.clone()));
                }
                if !used.contains(key) {
                    used.push(key.clone());
                }
            }
            let mut mine: Vec<(Option<String>, Result<Outcome>)> = ids[*k]
                .iter()
                .map(|j| {
                    let r = results.remove(j);
                    (
                        j.item.clone(),
                        r.unwrap_or_else(|| Err(Error::new("err.flow.worker"))),
                    )
                })
                .collect();
            if mine.is_empty() {
                mine.push((None, Err(Error::new("err.flow.worker"))));
            }
            let (act, made) = settle(inst, step, resolved, used, mine);
            for (key, a) in made.iter().flatten().flat_map(|(_, v)| v) {
                run.entities.push((key.clone(), a.clone()));
            }
            run.activities.push(act);
            done[*k] = made;
        }
    }
    Ok(run)
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
