//! The jobs of `execute`: the live scheduler decides which job is ready, a pool of `workers` threads runs them as soon as the jobs they read are done (a
//! `per_item` step starts on a file when that file is made upstream, not when the whole level is). Results are kept by job and gathered by `run.rs` in
//! the order of the plan, so the record does not depend on who finished first.

use super::{file_artifact, Artifact, Job, Outcome, Runner};
use crate::budget::sha256_hex;
use crate::error::{Error, Result};
use crate::flow::sched::{Done, JobId, Limits, Sched};
use crate::flow::{canonical, Instance, Plan, Ref, Resolved};
use crate::step::Step;
use std::collections::HashMap;
use std::sync::{Condvar, Mutex, MutexGuard};

/// Results by job.
pub(super) type Results = HashMap<JobId, Result<Outcome>>;

struct Core {
    sched: Sched,
    results: Results,
    running: usize,
}

struct Ctx<'a> {
    plan: &'a Plan,
    steps: Vec<&'a Step>,
    resolved: Vec<Resolved>,
    /// The jobs of each instance, in the order of its items.
    ids: Vec<Vec<JobId>>,
    roots: Vec<String>,
}

fn locked(m: &Mutex<Core>) -> MutexGuard<'_, Core> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The characters of a file name that stay in a file-name root.
fn clean(name: &str) -> String {
    // `NAME@ID` names a sample of a dataset: the root keeps the id
    if let Some((base, id)) = name.rsplit_once('@') {
        return format!("{}-{}", clean(base), clean(id));
    }
    let stem = name.rsplit_once('.').map_or(name, |s| s.0);
    let root: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if root.is_empty() {
        "x".into()
    } else {
        root
    }
}

/// A file-name root for every input of the plan: its name without extension; when two inputs share a root, both get a short hash of their full name.
pub fn file_roots(plan: &Plan) -> Vec<String> {
    let roots: Vec<String> = plan.inputs.iter().map(|f| clean(&f.name)).collect();
    plan.inputs
        .iter()
        .zip(&roots)
        .map(|(f, r)| {
            if roots.iter().filter(|x| *x == r).count() > 1 {
                format!("{r}-{}", &sha256_hex(f.name.as_bytes())[..8])
            } else {
                r.clone()
            }
        })
        .collect()
}

/// Runs every job of `plan` that can run; returns the jobs of each instance and what they made or how they failed.
pub(super) fn run_jobs(
    plan: &Plan,
    registry: &[Step],
    runner: &dyn Runner,
    workers: usize,
) -> Result<(Vec<Vec<JobId>>, Results)> {
    let sched = Sched::new(plan, registry, env!("CARGO_PKG_VERSION"), &[]);
    let all = sched.jobs();
    let mut steps = Vec::new();
    let mut resolved = Vec::new();
    let mut ids = Vec::new();
    for inst in &plan.steps {
        let step = registry
            .iter()
            .find(|s| s.id == inst.step && s.version == inst.version)
            .ok_or_else(|| Error::new("err.flow.step.unknown"))?;
        steps.push(step);
        resolved.push(canonical(inst, step)?);
        ids.push(
            all.iter()
                .filter(|(j, _)| j.instance == inst.id)
                .map(|(j, _)| j.clone())
                .collect(),
        );
    }
    let ctx = Ctx {
        plan,
        steps,
        resolved,
        ids,
        roots: file_roots(plan),
    };
    let core = Mutex::new(Core {
        sched,
        results: HashMap::new(),
        running: 0,
    });
    let cv = Condvar::new();
    #[cfg(not(target_family = "wasm"))]
    if workers > 1 {
        std::thread::scope(|s| {
            for _ in 0..workers {
                s.spawn(|| ctx.work(runner, &core, &cv));
            }
        });
        let results = locked(&core).results.drain().collect();
        return Ok((ctx.ids, results));
    }
    let _ = workers;
    ctx.work(runner, &core, &cv);
    let results = locked(&core).results.drain().collect();
    Ok((ctx.ids, results))
}

impl Ctx<'_> {
    fn position(&self, instance: &str) -> Option<usize> {
        self.plan.steps.iter().position(|x| x.id == instance)
    }

    /// One worker: takes the next job the scheduler offers, runs it, reports; waits while others may still make a job ready, leaves when nothing is left.
    fn work(&self, runner: &dyn Runner, core: &Mutex<Core>, cv: &Condvar) {
        loop {
            let mut g = locked(core);
            let (id, inputs) = loop {
                if let Some(p) = self.pick(&mut g) {
                    break p;
                }
                if g.running == 0 {
                    cv.notify_all();
                    return;
                }
                g = cv.wait(g).unwrap_or_else(|e| e.into_inner());
            };
            g.running += 1;
            drop(g);
            let job = self.job(&id, inputs);
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runner.run(&job)))
                .unwrap_or_else(|_| Err(Error::new("err.flow.worker")));
            let mut g = locked(core);
            g.running -= 1;
            let told = match &r {
                Ok(o) => Ok(Done { ms: o.ms }),
                Err(e) => Err(e.clone()),
            };
            g.sched.finished(&id, told);
            g.results.insert(id, r);
            cv.notify_all();
        }
    }

    /// The most urgent job that is ready, marked as started, with what it reads.
    fn pick(&self, g: &mut Core) -> Option<(JobId, Vec<(String, Vec<Artifact>)>)> {
        let limits = Limits {
            slots: 1,
            mem_mb: f64::INFINITY,
        };
        let id = g.sched.next(&limits, |_| 0.0).into_iter().next()?.id;
        g.sched.started(&id);
        let inputs = self.inputs(&id, &g.results);
        Some((id, inputs))
    }

    fn job(&self, id: &JobId, inputs: Vec<(String, Vec<Artifact>)>) -> Job<'_> {
        let k = self.position(&id.instance).unwrap_or(0);
        let item = id
            .item
            .as_ref()
            .and_then(|_| self.ids[k].iter().position(|j| j == id));
        let root = id
            .item
            .as_deref()
            .and_then(|n| self.plan.inputs.iter().position(|f| f.name == n))
            .map(|p| self.roots[p].clone());
        Job {
            instance: &self.plan.steps[k],
            step: self.steps[k],
            resolved: &self.resolved[k],
            inputs,
            item,
            item_name: id.item.clone(),
            item_root: root,
        }
    }

    /// What the job reads on each port written in the file; the driving port of a `per_item` job holds its one item.
    fn inputs(&self, id: &JobId, results: &Results) -> Vec<(String, Vec<Artifact>)> {
        let k = self.position(&id.instance).unwrap_or(0);
        let (inst, step) = (&self.plan.steps[k], self.steps[k]);
        let driver = id.item.as_ref().and_then(|_| {
            step.ins
                .iter()
                .filter(|p| p.many)
                .find(|p| !refs_of(inst, p.name).is_empty())
                .map(|p| p.name)
        });
        inst.ins
            .iter()
            .map(|(port, refs)| {
                let driving = driver == Some(port.as_str());
                let mut got: Vec<Artifact> = Vec::new();
                for r in refs {
                    match r {
                        Ref::File(n) => {
                            if let Some(f) = self.plan.input(n) {
                                if !driving || id.item.as_deref() == Some(n.as_str()) {
                                    got.push(file_artifact(step, port, f));
                                }
                            }
                        }
                        Ref::Out {
                            instance,
                            port: out,
                        } => {
                            let Some(s) = self.position(instance) else {
                                continue;
                            };
                            let per = self.ids[s].iter().any(|j| j.item.is_some());
                            for j in &self.ids[s] {
                                if driving && per && j.item != id.item {
                                    continue;
                                }
                                if let Some(Ok(o)) = results.get(j) {
                                    got.extend(
                                        o.outputs
                                            .iter()
                                            .filter(|(p, _)| p == out)
                                            .map(|(_, a)| a.clone()),
                                    );
                                }
                            }
                        }
                    }
                }
                (port.clone(), got)
            })
            .collect()
    }
}

fn refs_of<'a>(inst: &'a Instance, port: &str) -> &'a [Ref] {
    inst.ins
        .iter()
        .find(|(p, _)| p == port)
        .map_or(&[][..], |(_, r)| r.as_slice())
}
