//! A plan that runs on this computer and takes changes while it runs (`flow::live::LiveRun`): the scheduler of `mzlab_core::flow::sched` decides which job
//! is ready, a pool of threads runs the jobs with the `FileRunner` of `flow.rs` (the loop is `flow::work`, the very one of `execute`), and `update` swaps
//! the plan under it: a job whose `plan_key` is unchanged goes on, one whose key changed is cancelled through its flag and runs again. Used by the desktop
//! app (routes `flow/live/<op>`) and by `mzlab-cli flow run --jsonl`. Design and rules: `docs/agenti/flow-live.md`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime};

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::sched::{Command, Done, JobId, Landed, Sched, State};
use mzlab_core::flow::{
    parse, record, sched_act, sched_ask, sched_cancels, work, Core, Ctx, FileInput, Hooks,
    Outcome, Plan, Platform, Run,
};
use mzlab_core::json::Value;
use mzlab_core::step::{DataType, Step};

use crate::flow::{executable, FileRunner};
use crate::flow_pace::{Gate, Report, Watch};

/// The least time between two progress events of one job.
const BEAT: Duration = Duration::from_millis(250);

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Threads of a live run on this computer: all cores but one, at most four (as the run of the desktop app).
pub fn default_workers() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get().saturating_sub(1).clamp(1, 4))
}

/// What the runner and the jobs share with the run: the cancel flag of each running job, where they report and the scheduler.
struct Link {
    core: Arc<Mutex<Core>>,
    flags: Mutex<HashMap<JobId, Arc<AtomicBool>>>,
    beat: Arc<Mutex<HashMap<JobId, Instant>>>,
}

impl Watch for Link {
    fn gate(&self, instance: &str, item: Option<&str>) -> Gate {
        let id = JobId::new(instance, item);
        let stop = lock(&self.flags).get(&id).cloned();
        let (core, beat) = (self.core.clone(), self.beat.clone());
        let report: Report = Arc::new(move |frac| {
            let now = Instant::now();
            {
                let mut b = lock(&beat);
                if frac < 1.0 && b.get(&id).is_some_and(|t| now.duration_since(*t) < BEAT) {
                    return;
                }
                b.insert(id.clone(), now);
            }
            lock(&core).sched.progress(&id, frac);
        });
        Gate {
            stop,
            report: Some(report),
        }
    }
}

/// The result of a job as it was made, and the files it is in as they were: kept under the `plan_key`, so that a plan that goes back to a key
/// already made does not run the job again.
struct Kept {
    outcome: Outcome,
    files: Vec<(PathBuf, u64, SystemTime)>,
}

fn stamp(p: &Path) -> Option<(u64, SystemTime)> {
    let m = std::fs::metadata(p).ok()?;
    Some((m.len(), m.modified().ok()?))
}

struct Shared {
    core: Arc<Mutex<Core>>,
    cv: Condvar,
    runner: FileRunner,
    link: Arc<Link>,
    registry: Vec<Step>,
    kept: Mutex<HashMap<String, Kept>>,
    /// Jobs a caller runs itself (`LiveRun::lend`).
    lent: Mutex<HashSet<JobId>>,
    stop: AtomicBool,
}

impl Hooks for Shared {
    fn live(&self) -> bool {
        !self.stop.load(Ordering::SeqCst)
    }

    fn reuse(&self, key: &str) -> Option<Outcome> {
        let mut kept = lock(&self.kept);
        let k = kept.get(key)?;
        if k.files.iter().all(|(p, n, t)| stamp(p) == Some((*n, *t))) {
            return Some(k.outcome.clone());
        }
        kept.remove(key);
        None
    }

    fn leave(&self, id: &JobId) -> bool {
        lock(&self.lent).contains(id)
    }

    fn begin(&self, id: &JobId) {
        lock(&self.link.flags).insert(id.clone(), Arc::new(AtomicBool::new(false)));
    }

    fn end(&self, id: &JobId, key: &str, r: &Result<Outcome>, _landed: &Landed) {
        lock(&self.link.flags).remove(id);
        lock(&self.link.beat).remove(id);
        // a result of a key that is no longer expected is a true result of that key
        if let Ok(o) = r {
            let files = o.outputs.iter().map(|(_, a)| self.runner.path(a));
            let files = files
                .filter_map(|p| stamp(&p).map(|(n, t)| (p, n, t)))
                .collect();
            let outcome = Outcome {
                cached: true,
                ..o.clone()
            };
            lock(&self.kept).insert(key.to_string(), Kept { outcome, files });
        }
    }
}

/// How a job that a caller runs itself stands.
pub enum Lend {
    /// Running, with the share done when it is known.
    Running(Option<f64>),
    /// Done: the files it made (output port, path, type).
    Done(Vec<(&'static str, PathBuf, DataType)>),
    Failed(Error),
}

/// A plan being run; dropped, it cancels what runs and waits for the threads.
pub struct LiveRun {
    shared: Arc<Shared>,
    threads: Vec<JoinHandle<()>>,
    dir: PathBuf,
    /// The plan as it runs (inputs hashed, comparisons expanded).
    plan: Mutex<Plan>,
}

/// The plan of `text` as it runs: inputs hashed (those of `known` keep their hash), comparisons expanded, every step with a native executor.
fn ready_plan(text: &[u8], dir: &Path, known: &[FileInput], reg: &[Step]) -> Result<Plan> {
    let mut plan = parse(text)?;
    for f in &mut plan.inputs {
        match known.iter().find(|k| k.name == f.name) {
            Some(k) if f.sha256.is_empty() || f.sha256 == k.sha256 => {
                (f.sha256, f.bytes) = (k.sha256.clone(), k.bytes);
            }
            _ => {
                let path = dir.join(crate::flow_set::dataset_name(&f.name));
                let (bytes, sha) = crate::flow_set::digest(&path).map_err(|e| e.with("input", &f.name))?;
                if !f.sha256.is_empty() && f.sha256 != sha {
                    return Err(Error::new("err.flow.input.sha").with("input", &f.name));
                }
                (f.sha256, f.bytes) = (sha, Some(bytes));
            }
        }
    }
    let check = |problems: Vec<mzlab_core::flow::Problem>| {
        let list: Vec<String> = problems.iter().map(crate::flow_run::problem_text).collect();
        Error::new("err.flow.check").with("problems", list.join(" "))
    };
    let problems = crate::flow_compare::check(&plan, reg, Platform::current());
    if !problems.is_empty() {
        return Err(check(problems));
    }
    let plan = crate::flow_compare::expand(plan, reg)?;
    if let Some(s) = plan.steps.iter().find(|s| !executable(&s.step)) {
        let e = Error::new("err.flow.step.native").with("step", s.step.clone());
        return Err(e);
    }
    Ok(plan)
}

impl LiveRun {
    /// Runs the plan of `text` (the inputs are the files of `dir` named in it; what the steps make goes to `work`, whose cache gives back the steps that
    /// are as they were) with `workers` threads (none: the scheduler only answers, nothing runs). `registry` is `flow::steps()`.
    pub fn start(
        text: &[u8],
        registry: Vec<Step>,
        dir: &Path,
        work: &Path,
        workers: usize,
    ) -> Result<LiveRun> {
        let plan = ready_plan(text, dir, &[], &registry)?;
        std::fs::create_dir_all(work).map_err(|e| Error::new("err.file.write").with("detail", e.to_string()))?;
        let runner = FileRunner::cached(dir, work, None, Some(false));
        LiveRun::with_runner(plan, registry, runner, dir, workers)
    }

    /// `start` for a plan that is checked and expanded, with the runner it was prepared for.
    pub(crate) fn with_runner(
        plan: Plan,
        registry: Vec<Step>,
        mut runner: FileRunner,
        dir: &Path,
        workers: usize,
    ) -> Result<LiveRun> {
        let sched = Sched::new(&plan, &registry, env!("CARGO_PKG_VERSION"), &[]);
        let ctx = Ctx::new(&plan, &registry, &sched).ok();
        let core = Arc::new(Mutex::new(Core::new(sched, ctx)));
        let link = Arc::new(Link {
            core: core.clone(),
            flags: Mutex::new(HashMap::new()),
            beat: Arc::new(Mutex::new(HashMap::new())),
        });
        runner = runner.with_watch(link.clone());
        let shared = Arc::new(Shared {
            core,
            cv: Condvar::new(),
            runner,
            link,
            registry,
            kept: Mutex::new(HashMap::new()),
            lent: Mutex::new(HashSet::new()),
            stop: AtomicBool::new(false),
        });
        let threads = (0..workers)
            .map(|_| {
                let s = shared.clone();
                std::thread::spawn(move || work(&s.runner, &s.core, &s.cv, &*s))
            })
            .collect();
        Ok(LiveRun {
            shared,
            threads,
            dir: dir.to_path_buf(),
            plan: Mutex::new(plan),
        })
    }

    /// The plan of `text` replaces the one that runs; answers the jobs to cancel as `api/flow/sched/update` does. A job with the same `plan_key` goes on;
    /// one with another is cancelled (its flag goes up and it ends at the next piece), kept as stale or waiting, and runs again.
    pub fn update(&self, text: &[u8]) -> Result<String> {
        let known = lock(&self.plan).inputs.clone();
        let plan = ready_plan(text, &self.dir, &known, &self.shared.registry)?;
        let mut g = lock(&self.shared.core);
        let cmds = g.sched.update(&plan);
        g.ctx = Ctx::new(&plan, &self.shared.registry, &g.sched).ok().map(Arc::new);
        self.raise(&cmds);
        *lock(&self.plan) = plan;
        drop(g);
        self.shared.cv.notify_all();
        Ok(sched_cancels(&cmds))
    }

    fn raise(&self, cmds: &[Command]) {
        let flags = lock(&self.shared.link.flags);
        for Command::Cancel(id) in cmds {
            if let Some(f) = flags.get(id) {
                f.store(true, Ordering::SeqCst);
            }
        }
    }

    /// A call that changes the scheduler (`pause`, `resume`, `retry`, `unblock`, `block`, as the route `api/flow/sched/<op>` of the page); None for another op.
    pub fn act(&self, op: &str, body: &Value) -> Option<String> {
        let r = sched_act(&mut lock(&self.shared.core).sched, op, body);
        self.shared.cv.notify_all();
        r
    }

    /// A question to the scheduler (`jobs`, `nodes`, `events`, `rev`); None for another op.
    pub fn ask(&self, op: &str, body: &Value) -> Option<String> {
        sched_ask(&lock(&self.shared.core).sched, op, body)
    }

    pub fn pause(&self) {
        lock(&self.shared.core).sched.pause();
    }

    pub fn resume(&self) {
        lock(&self.shared.core).sched.resume();
        self.shared.cv.notify_all();
    }

    /// Pauses and raises the flag of every running job: they end at the next piece as cancelled (`retry` runs them again).
    pub fn cancel_all(&self) {
        lock(&self.shared.core).sched.pause();
        for f in lock(&self.shared.link.flags).values() {
            f.store(true, Ordering::SeqCst);
        }
    }

    /// The `flowlive/1` events after `seq`, as a JSON array.
    pub fn events_since(&self, seq: u64) -> String {
        lock(&self.shared.core).sched.events_since(seq)
    }

    /// The same events, one text each with its `seq`.
    pub fn event_lines(&self, seq: u64) -> Vec<(u64, String)> {
        lock(&self.shared.core).sched.event_texts(seq)
    }

    /// Waits until no job runs and none is ready or stale (all done, failed, blocked or paused); false if `limit` passes first.
    pub fn wait_idle(&self, limit: Duration) -> bool {
        let end = Instant::now() + limit;
        let mut g = lock(&self.shared.core);
        loop {
            let busy = g
                .sched
                .jobs()
                .iter()
                .any(|(_, s)| matches!(s, State::Running { .. } | State::Ready | State::Stale));
            if !busy {
                return true;
            }
            let Some(left) = end.checked_duration_since(Instant::now()) else {
                return false;
            };
            let wait = left.min(Duration::from_millis(50));
            g = self
                .shared
                .cv
                .wait_timeout(g, wait)
                .unwrap_or_else(|e| e.into_inner())
                .0;
        }
    }

    /// The record of the run (what `flow::execute` gives) and the cache of the work folder written; for a plan whose jobs are all finished.
    pub fn finish(&self) -> Result<Run> {
        self.shared.runner.save_cache()?;
        let plan = lock(&self.plan).clone();
        let g = lock(&self.shared.core);
        let ctx = g.ctx.clone().ok_or_else(|| Error::new("err.flow.sched"))?;
        record(&plan, &self.shared.registry, ctx.ids(), g.results.clone())
    }

    /// The jobs of the instances of the step `step`.
    pub fn jobs_of(&self, step: &str) -> Vec<JobId> {
        let plan = lock(&self.plan);
        let g = lock(&self.shared.core);
        let Some(ctx) = &g.ctx else { return Vec::new() };
        let on = plan.steps.iter().zip(ctx.ids());
        on.filter(|(i, _)| i.step.split('@').next() == Some(step))
            .flat_map(|(_, ids)| ids.iter().cloned())
            .collect()
    }

    /// The jobs `ids` are run by the caller (the conversion queue of the desktop app): the workers leave them alone, `lend` tells how they stand.
    pub fn claim(&self, ids: &[JobId]) {
        lock(&self.shared.lent).extend(ids.iter().cloned());
    }

    /// A job the caller no longer runs: the workers take it again.
    pub fn release(&self, id: &JobId) {
        lock(&self.shared.lent).remove(id);
        self.shared.cv.notify_all();
    }

    /// How a claimed job stands: shown running while it is, `reused` (when it was ready already) or `done` with the files it made. False when the job
    /// is not in a state to take that: running here, or finished.
    pub fn lend(&self, id: &JobId, how: Lend) -> bool {
        let sh = &self.shared;
        let r = match how {
            Lend::Done(files) => {
                let outputs: Result<Vec<_>> = files
                    .iter()
                    .map(|(port, p, ty)| Ok((port.to_string(), sh.runner.keep(p.clone(), *ty)?)))
                    .collect();
                outputs.map(|outputs| Outcome { outputs, cached: true, ..Outcome::default() })
            }
            Lend::Failed(e) => Err(e),
            Lend::Running(frac) => {
                let mut g = lock(&sh.core);
                let up = is_running(&g.sched, id) || g.sched.started(id);
                if let (true, Some(f)) = (up, frac) {
                    g.sched.progress(id, f);
                }
                return up && lock(&sh.lent).contains(id);
            }
        };
        self.settle(id, r)
    }

    fn settle(&self, id: &JobId, r: Result<Outcome>) -> bool {
        let sh = &self.shared;
        let mut g = lock(&sh.core);
        let running = is_running(&g.sched, id);
        let mine = lock(&sh.lent).remove(id);
        if running && !mine {
            return false;
        }
        let told = match &r {
            Ok(o) => Ok(Done { ms: o.ms }),
            Err(e) => Err(e.clone()),
        };
        let landed = if running {
            g.sched.finished(id, told) == Landed::Accepted
        } else if r.is_ok() {
            g.sched.cached(id)
        } else {
            g.sched.started(id) && g.sched.finished(id, told) == Landed::Accepted
        };
        if landed {
            g.results.insert(id.clone(), r);
        }
        drop(g);
        sh.cv.notify_all();
        landed
    }
}

fn is_running(s: &Sched, id: &JobId) -> bool {
    s.jobs().iter().any(|(j, st)| j == id && matches!(st, State::Running { .. }))
}

impl Drop for LiveRun {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        self.cancel_all();
        self.shared.cv.notify_all();
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
        let _ = self.shared.runner.save_cache();
    }
}

/// Runs the checked plan `ready` like `run_checked` and prints the `flowlive/1` events on stdout, one line each, as they happen (`flow run --jsonl`);
/// returns the run and its `prov/1` text (`text` is the plan as it was read).
pub(crate) fn run_events(
    ready: &Plan,
    text: &[u8],
    runner: FileRunner,
    dir: &Path,
    workers: usize,
) -> Result<(Run, String)> {
    use std::io::Write;
    let live = LiveRun::with_runner(ready.clone(), crate::flow::steps(), runner, dir, workers)?;
    let mut seq = 0;
    loop {
        let idle = live.wait_idle(Duration::from_millis(100));
        let mut out = std::io::stdout().lock();
        for (n, line) in live.event_lines(seq) {
            let _ = writeln!(out, "{line}");
            seq = n;
        }
        let _ = out.flush();
        if idle {
            let run = live.finish()?;
            let prov = crate::flow_run::prov_text(text, &run);
            return Ok((run, prov));
        }
    }
}

#[cfg(test)]
#[path = "flow_live_tests.rs"]
mod tests;
