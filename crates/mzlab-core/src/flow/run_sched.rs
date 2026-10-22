//! The jobs of `execute`: the live scheduler decides which job is ready, a pool of `workers` threads runs them as soon as the jobs they read are done (a
//! `per_item` step starts on a file when that file is made upstream, not when the whole level is). Results are kept by job and gathered by `run.rs` in
//! the order of the plan, so the record does not depend on who finished first. `work` (pick a job, run it, report it) is also the loop of a live run
//! that takes plan changes (`mzlab-cli` `flow::live`), through `Hooks`.

use super::{file_artifact, Artifact, Job, Outcome, Runner};
use crate::budget::sha256_hex;
use crate::error::{Error, Result};
use crate::flow::sched::{Done, JobId, Landed, Limits, Sched};
use crate::flow::{canonical, Instance, Plan, Ref, Resolved};
use crate::step::Step;
use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

/// Results by job.
pub type Results = HashMap<JobId, Result<Outcome>>;
/// What a job reads: for each input port, the artifacts it gets.
type Ins = Vec<(String, Vec<Artifact>)>;

/// What a live run adds to the loop of `work`; `()` is a plain `execute`.
pub trait Hooks: Sync {
    /// A live run waits for work when nothing is ready; a plain one leaves.
    fn live(&self) -> bool {
        false
    }
    /// A result already made for a job with this `plan_key`: the job is `reused` without running.
    fn reuse(&self, _key: &str) -> Option<Outcome> {
        None
    }
    /// A job that someone else runs: `work` does not start it.
    fn leave(&self, _id: &JobId) -> bool {
        false
    }
    /// A job starts (called with the lock held: only record it).
    fn begin(&self, _id: &JobId) {}
    /// A job ended with `r` under `key`; `landed` says whether the scheduler used it (a discarded result is true for its key).
    fn end(&self, _id: &JobId, _key: &str, _r: &Result<Outcome>, _landed: &Landed) {}
}

impl Hooks for () {}

/// Runs the jobs of `core` on the calling thread: takes the next job the scheduler offers, runs it, reports; waits while others may still make a job
/// ready, leaves when nothing is left (a live run: when it is told to stop).
pub fn work(runner: &dyn Runner, core: &Mutex<Core>, cv: &Condvar, hooks: &dyn Hooks) {
    loop {
        let mut g = locked(core);
        let (id, inputs, ctx) = loop {
            if let Some(p) = pick(&mut g, hooks) {
                break p;
            }
            if g.running == 0 && !hooks.live() {
                cv.notify_all();
                return;
            }
            g = cv.wait(g).unwrap_or_else(|e| e.into_inner());
        };
        let key = g.sched.key(&id).unwrap_or("").to_string();
        g.running += 1;
        hooks.begin(&id);
        drop(g);
        let job = ctx.job(&id, inputs);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runner.run(&job)))
            .unwrap_or_else(|_| Err(Error::new("err.flow.worker")));
        let mut g = locked(core);
        g.running -= 1;
        let told = match &r {
            Ok(o) => Ok(Done { ms: o.ms }),
            Err(e) => Err(e.clone()),
        };
        let landed = g.sched.finished(&id, told);
        hooks.end(&id, &key, &r, &landed);
        if landed == Landed::Accepted {
            g.results.insert(id, r);
        }
        cv.notify_all();
    }
}

/// The most urgent job that is ready (and not left to someone else), marked as started, with what it reads and the plan it belongs to. A job whose key has a result already made is
/// `reused` and the next one is looked at.
fn pick(g: &mut Core, hooks: &dyn Hooks) -> Option<(JobId, Ins, Arc<Ctx>)> {
    let ctx = g.ctx.clone()?;
    let limits = Limits {
        slots: usize::MAX,
        mem_mb: f64::INFINITY,
    };
    loop {
        let specs = g.sched.next(&limits, |_| 0.0);
        let spec = specs.into_iter().find(|s| !hooks.leave(&s.id))?;
        let reused = hooks.reuse(&spec.key).filter(|_| g.sched.cached(&spec.id));
        match reused {
            Some(o) => {
                g.results.insert(spec.id, Ok(o));
            }
            None => {
                g.sched.started(&spec.id);
                let inputs = ctx.inputs(&spec.id, &g.results);
                return Some((spec.id, inputs, ctx));
            }
        }
    }
}

/// The scheduler, what the finished jobs made and how many are running, under one lock.
pub struct Core {
    pub sched: Sched,
    pub results: Results,
    running: usize,
    /// The plan the jobs belong to; none while the plan cannot be run (no job starts).
    pub ctx: Option<Arc<Ctx>>,
}

impl Core {
    pub fn new(sched: Sched, ctx: Option<Ctx>) -> Core {
        Core {
            sched,
            results: HashMap::new(),
            running: 0,
            ctx: ctx.map(Arc::new),
        }
    }
}

/// A plan with its steps, parameters and jobs, the context in which the jobs run.
pub struct Ctx {
    plan: Plan,
    steps: Vec<Step>,
    resolved: Vec<Resolved>,
    /// The jobs of each instance, in the order of its items.
    ids: Vec<Vec<JobId>>,
    roots: Vec<String>,
}

impl Ctx {
    fn position(&self, instance: &str) -> Option<usize> {
        self.plan.steps.iter().position(|x| x.id == instance)
    }

    /// The job `id` as the runner gets it, with what it reads.
    fn job(&self, id: &JobId, inputs: Ins) -> Job<'_> {
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
            step: &self.steps[k],
            resolved: &self.resolved[k],
            inputs,
            item,
            item_name: id.item.clone(),
            item_root: root,
        }
    }
}

fn locked(m: &Mutex<Core>) -> MutexGuard<'_, Core> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Ctx {
    /// What the job reads on each port written in the file; the driving port of a `per_item` job holds its one item.
    fn inputs(&self, id: &JobId, results: &Results) -> Ins {
        let k = self.position(&id.instance).unwrap_or(0);
        let (inst, step) = (&self.plan.steps[k], &self.steps[k]);
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

impl Ctx {
    /// The context of `plan` as `sched` lays out its jobs; the plan must name steps of `registry` with parameters that resolve.
    pub fn new(plan: &Plan, registry: &[Step], sched: &Sched) -> Result<Ctx> {
        let all = sched.jobs();
        let mut ctx = Ctx {
            plan: plan.clone(),
            steps: Vec::new(),
            resolved: Vec::new(),
            ids: Vec::new(),
            roots: file_roots(plan),
        };
        for inst in &plan.steps {
            let step = registry
                .iter()
                .find(|s| s.id == inst.step && s.version == inst.version)
                .ok_or_else(|| Error::new("err.flow.step.unknown"))?;
            ctx.resolved.push(canonical(inst, step)?);
            ctx.steps.push(step.clone());
            let mine = all.iter().filter(|(j, _)| j.instance == inst.id);
            ctx.ids.push(mine.map(|(j, _)| j.clone()).collect());
        }
        Ok(ctx)
    }

    /// The jobs of each instance of the plan, in the order of its items.
    pub fn ids(&self) -> &[Vec<JobId>] {
        &self.ids
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

fn refs_of<'a>(inst: &'a Instance, port: &str) -> &'a [Ref] {
    inst.ins
        .iter()
        .find(|(p, _)| p == port)
        .map_or(&[][..], |(_, r)| r.as_slice())
}

/// Runs every job of `plan` that can run; returns the jobs of each instance and what they made or how they failed.
pub(super) fn run_jobs(
    plan: &Plan,
    registry: &[Step],
    runner: &dyn Runner,
    workers: usize,
) -> Result<(Vec<Vec<JobId>>, Results)> {
    let sched = Sched::new(plan, registry, env!("CARGO_PKG_VERSION"), &[]);
    let ctx = Ctx::new(plan, registry, &sched)?;
    let ids = ctx.ids.clone();
    let core = Mutex::new(Core::new(sched, Some(ctx)));
    let cv = Condvar::new();
    #[cfg(not(target_family = "wasm"))]
    if workers > 1 {
        std::thread::scope(|s| {
            for _ in 0..workers {
                s.spawn(|| work(runner, &core, &cv, &()));
            }
        });
        let results = locked(&core).results.drain().collect();
        return Ok((ids, results));
    }
    let _ = workers;
    work(runner, &core, &cv, &());
    let results = locked(&core).results.drain().collect();
    Ok((ids, results))
}
