//! Execution of a checked plan: instances in plan order, the independent ones (same level, and the items of a `per_item` step) on `workers` threads, the
//! outputs gathered by index so that the record does not depend on who finished first. A `Runner` runs one job; the library has no files and no clock.
//! An error stops only the descendants of the instance that failed: its activity carries the error, the descendants are listed in `Run::skipped`.

use super::canon::{canonical, Resolved};
use super::check::{check, Platform};
use super::order::levels;
use super::prov::{Activity, ErrorRecord};
use super::{Instance, Plan, Ref};
use crate::error::{Error, Result};
use crate::json::Value;
use crate::step::{DataType, Step};

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
    /// The output came from the cache key (`cache_key`), the step did not run.
    pub cached: bool,
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

fn run_all(runner: &dyn Runner, jobs: &[Job], workers: usize) -> Vec<Result<Outcome>> {
    #[cfg(not(target_family = "wasm"))]
    if workers > 1 && jobs.len() > 1 {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Mutex;
        let next = AtomicUsize::new(0);
        let slots: Mutex<Vec<Option<Result<Outcome>>>> =
            Mutex::new((0..jobs.len()).map(|_| None).collect());
        std::thread::scope(|s| {
            for _ in 0..workers.min(jobs.len()) {
                s.spawn(|| loop {
                    let k = next.fetch_add(1, Ordering::Relaxed);
                    let Some(job) = jobs.get(k) else { break };
                    let r = runner.run(job);
                    slots.lock().unwrap_or_else(|e| e.into_inner())[k] = Some(r);
                });
            }
        });
        let slots = slots.into_inner().unwrap_or_else(|e| e.into_inner());
        return slots
            .into_iter()
            .map(|r| r.unwrap_or_else(|| Err(Error::new("err.flow.worker"))))
            .collect();
    }
    let _ = workers;
    jobs.iter().map(|j| runner.run(j)).collect()
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
                        let want = step.ins.iter().find(|p| p.name == port).map(|p| p.ty);
                        let ty =
                            f.ty.as_deref()
                                .and_then(DataType::from_name)
                                .or(want)
                                .unwrap_or(DataType::Raw);
                        let a = Artifact {
                            sha256: f.sha256.clone(),
                            bytes: f.bytes.unwrap_or(0),
                            ty,
                            label: Some(f.name.clone()),
                        };
                        got.push((format!("file:{name}"), a));
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

/// The jobs of an instance: one, or one per item of the first `many` port it reads when the step is `per_item`.
fn jobs<'a>(
    inst: &'a Instance,
    step: &'a Step,
    resolved: &'a Resolved,
    read: &[(String, Vec<Keyed>)],
) -> Vec<Job<'a>> {
    let plain = |r: &[(String, Vec<Keyed>)]| {
        r.iter()
            .map(|(p, v)| (p.clone(), v.iter().map(|k| k.1.clone()).collect()))
            .collect::<Vec<_>>()
    };
    let driver = step
        .ins
        .iter()
        .filter(|p| p.many)
        .find_map(|p| read.iter().position(|(n, v)| n == p.name && !v.is_empty()));
    match driver {
        Some(d) if step.per_item => (0..read[d].1.len())
            .map(|i| {
                let mut inputs = plain(read);
                inputs[d].1 = vec![read[d].1[i].1.clone()];
                Job {
                    instance: inst,
                    step,
                    resolved,
                    inputs,
                    item: Some(i),
                }
            })
            .collect(),
        _ => vec![Job {
            instance: inst,
            step,
            resolved,
            inputs: plain(read),
            item: None,
        }],
    }
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

/// The activity of an instance and, when it did not fail, what it made.
fn settle(
    inst: &Instance,
    step: &Step,
    resolved: &Resolved,
    used: Vec<String>,
    results: Vec<Result<Outcome>>,
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
        used,
        generated: Vec::new(),
        ms: 0.0,
        heap_peak_bytes: None,
        error: None,
        cached: !results.is_empty(),
    };
    let mut outcomes = Vec::new();
    for r in results {
        match r {
            Ok(o) => outcomes.push(o),
            Err(e) => {
                act.cached = false;
                act.error = Some(ErrorRecord {
                    error_key: e.key.to_string(),
                    params: e
                        .params
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.clone()))
                        .collect(),
                });
                return (act, None);
            }
        }
    }
    let mut made: Made = Vec::new();
    for port in &step.outs {
        let mut list: Vec<Keyed> = Vec::new();
        for o in &outcomes {
            for (name, a) in o.outputs.iter().filter(|(n, _)| n == port.name) {
                let key = if port.many {
                    format!("{}.{name}[{}]", inst.id, list.len())
                } else {
                    format!("{}.{name}", inst.id)
                };
                list.push((key, a.clone()));
            }
        }
        act.generated.extend(list.iter().map(|k| k.0.clone()));
        made.push((port.name.to_string(), list));
    }
    for o in &outcomes {
        act.ms += o.ms;
        act.cached &= o.cached;
        act.heap_peak_bytes = act.heap_peak_bytes.max(o.heap_peak_bytes);
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

/// Runs `plan` with `workers` threads (1 = in order; the record is the same for any number). The plan is checked first; an instance that fails stops its
/// descendants only.
pub fn execute(plan: &Plan, registry: &[Step], runner: &dyn Runner, workers: usize) -> Result<Run> {
    let problems = check(plan, registry, Platform::current());
    if let Some(p) = problems.first() {
        return Err(Error::new("err.flow.check")
            .with("count", problems.len())
            .with("instance", &p.instance)
            .with("error_key", p.error_key));
    }
    let mut run = Run::default();
    let mut done: Vec<Option<Made>> = vec![None; plan.steps.len()];
    for wave in levels(plan)? {
        // resolve and read in file order, run the whole wave together, settle in file order
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
        let all_jobs: Vec<Vec<Job>> = ready
            .iter()
            .map(|(k, step, resolved, rd)| jobs(&plan.steps[*k], step, resolved, rd))
            .collect();
        let counts: Vec<usize> = all_jobs.iter().map(Vec::len).collect();
        let flat: Vec<Job> = all_jobs.into_iter().flatten().collect();
        let mut results = run_all(runner, &flat, workers).into_iter();
        drop(flat);
        for ((k, step, resolved, rd), n) in ready.iter().zip(counts) {
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
            let mine: Vec<Result<Outcome>> = results.by_ref().take(n).collect();
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
