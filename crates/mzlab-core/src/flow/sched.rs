//! The live scheduler of a workflow: a state machine over the jobs of an `mzflow/2` plan (one per instance, or one per item of a `per_item` step) that knows
//! which jobs exist, in which state they are and which are ready, and reacts to a plan that changes. Pure: no threads, files, clock or network; the executor
//! (threads, Workers) asks `next`, reports `started`/`progress`/`finished`/`cached` and reads `events_since`. Design: `docs/studi/flow-live/README.md`.

use crate::error::Error;
use crate::flow::{check, Plan, Platform};
use crate::step::Step;
use events::plan_event;
use graph::{build, Node};
use std::collections::{HashMap, HashSet, VecDeque};

#[path = "sched_events.rs"]
mod events;
#[path = "sched_graph.rs"]
mod graph;
#[path = "sched_view.rs"]
mod view;

pub use events::{NodeView, State};
pub use graph::JobId;

impl Sched {
    /// `identities`: for a plan input that carries no `sha256`, the text that stands for its content (name, size, mtime) as the caller knows it.
    pub fn new(
        plan: &Plan,
        registry: &[Step],
        program_version: &str,
        identities: &[(String, String)],
    ) -> Sched {
        let mut s = Sched {
            registry: registry.to_vec(),
            program: program_version.to_string(),
            identities: identities.to_vec(),
            plan: Plan::default(),
            jobs: Vec::new(),
            problems: Vec::new(),
            resources: Vec::new(),
            ghosts: Vec::new(),
            gone: Vec::new(),
            rev: 0,
            seq: 0,
            log: VecDeque::new(),
            paused: false,
            shown_nodes: Vec::new(),
        };
        s.apply(plan);
        s
    }

    /// The plan changed (a new version of it): keeps the jobs whose key is the same, makes the others stale or waiting, drops the jobs that are gone.
    /// Returns what to cancel.
    pub fn update(&mut self, plan: &Plan) -> Vec<Command> {
        self.rev += 1;
        self.apply(plan)
    }

    /// Sets the fingerprint of a plan input that has no `sha256` (takes effect at the next `update`).
    pub fn set_identity(&mut self, name: &str, text: &str) {
        match self.identities.iter_mut().find(|i| i.0 == name) {
            Some(i) => i.1 = text.to_string(),
            None => self.identities.push((name.to_string(), text.to_string())),
        }
    }

    /// Declares the exclusive resources a job holds while it runs (`dataset:<path>`, `opfs:<file>`): two jobs with one in common never start together.
    pub fn set_resources(&mut self, id: &JobId, resources: Vec<String>) {
        self.resources.retain(|r| r.0 != *id);
        self.resources.push((id.clone(), resources));
    }

    fn apply(&mut self, plan: &Plan) -> Vec<Command> {
        let identity = |n: &str| identity_of(plan, &self.identities, n);
        let nodes = match build(plan, &self.registry, &self.program, &identity) {
            Ok(n) => n,
            Err(e) => {
                self.plan = plan.clone();
                self.push(|seq, rev| plan_event(seq, rev, e.key));
                return Vec::new();
            }
        };
        self.problems = Vec::new();
        for p in check(plan, &self.registry, Platform::current()) {
            if !self.problems.iter().any(|x| x.0 == p.instance) {
                let by = if p.error_key == "err.flow.unavailable" {
                    "platform.computer"
                } else {
                    "plan.problem"
                };
                self.problems.push((p.instance, by));
            }
        }
        let mut old: Vec<Option<Job>> = std::mem::take(&mut self.jobs)
            .into_iter()
            .map(Some)
            .collect();
        let mut cmds = Vec::new();
        for node in nodes {
            let found = old
                .iter_mut()
                .find(|o| o.as_ref().is_some_and(|j| j.node.id == node.id));
            let job = match found.and_then(Option::take) {
                Some(mut j) => {
                    if j.node.key != node.key {
                        if matches!(j.state, State::Running { .. }) {
                            cmds.push(Command::Cancel(node.id.clone()));
                            j.zombie = Some(j.node.key.clone());
                        }
                        j.state = State::Waiting;
                        j.ms = 0.0;
                    }
                    j.node = node;
                    j
                }
                None => Job {
                    node,
                    state: State::Waiting,
                    has_output: false,
                    go: false,
                    ms: 0.0,
                    zombie: None,
                    held: None,
                    shown: None,
                },
            };
            self.jobs.push(job);
        }
        for j in old.into_iter().flatten() {
            if matches!(j.state, State::Running { .. }) {
                cmds.push(Command::Cancel(j.node.id.clone()));
                self.ghosts.push((j.node.id.clone(), j.node.key.clone()));
            }
            self.gone.push((j.node.id, j.node.key));
        }
        self.plan = plan.clone();
        self.settle();
        self.flush();
        cmds
    }

    /// Derives the state of every job that is not running, done or failed from the jobs it reads; jobs are in dependency order.
    fn settle(&mut self) {
        let pos: HashMap<JobId, usize> = self
            .jobs
            .iter()
            .enumerate()
            .map(|(i, j)| (j.node.id.clone(), i))
            .collect();
        for i in 0..self.jobs.len() {
            let (mut failed, mut ok) = (false, true);
            for d in &self.jobs[i].node.deps {
                match pos.get(d).map(|&p| &self.jobs[p].state) {
                    Some(State::Done | State::Reused) => {}
                    Some(
                        State::Error { .. }
                        | State::Refused { .. }
                        | State::Skipped
                        | State::Cancelled,
                    ) => failed = true,
                    _ => ok = false,
                }
            }
            let by = self
                .problems
                .iter()
                .find(|p| p.0 == self.jobs[i].node.id.instance)
                .map(|p| p.1);
            let j = &mut self.jobs[i];
            j.go = ok && !failed;
            if matches!(
                j.state,
                State::Running { .. }
                    | State::Done
                    | State::Reused
                    | State::Error { .. }
                    | State::Refused { .. }
                    | State::Cancelled
            ) {
                continue;
            }
            j.state = if j.node.invalid {
                State::Blocked {
                    by: "plan.problem".into(),
                }
            } else if let Some(by) = by {
                State::Blocked { by: by.into() }
            } else if let Some(by) = &j.held {
                State::Blocked { by: by.clone() }
            } else if failed {
                State::Skipped
            } else if j.has_output {
                State::Stale
            } else if ok {
                State::Ready
            } else {
                State::Waiting
            };
        }
    }

    fn at(&self, id: &JobId) -> Option<usize> {
        self.jobs.iter().position(|j| j.node.id == *id)
    }

    fn startable(&self, j: &Job) -> bool {
        let id = &j.node.id;
        j.go && matches!(j.state, State::Ready | State::Stale)
            && j.zombie.is_none()
            && !self.ghosts.iter().any(|g| g.0 == *id)
    }

    /// The jobs to start now, most urgent first: the items already under way go deeper before a new item is opened, then the order of the inputs of the
    /// plan, then the order of the plan. A job is left out when it does not fit the free slots or memory, or shares an exclusive resource with a job that
    /// runs or was chosen before it. Nothing is started by this call: `started` does it.
    pub fn next<F: Fn(&JobId) -> f64>(&self, limits: &Limits, estimate: F) -> Vec<JobSpec> {
        if self.paused {
            return Vec::new();
        }
        let live = |s: &State| matches!(s, State::Running { .. } | State::Done | State::Reused);
        let touched: HashSet<&str> = self
            .jobs
            .iter()
            .filter(|j| live(&j.state))
            .filter_map(|j| j.node.id.item.as_deref())
            .collect();
        let mut held: Vec<String> = Vec::new();
        for (id, res) in &self.resources {
            let running = self.at(id).is_some_and(|p| {
                matches!(self.jobs[p].state, State::Running { .. }) || self.jobs[p].zombie.is_some()
            });
            if running || self.ghosts.iter().any(|g| g.0 == *id) {
                held.extend(res.iter().cloned());
            }
        }
        let mut order: Vec<usize> = (0..self.jobs.len())
            .filter(|&p| self.startable(&self.jobs[p]))
            .collect();
        crate::sortc::slice_by_key(&mut order, |&p| {
            let n = &self.jobs[p].node;
            let started = n.id.item.as_deref().is_some_and(|i| touched.contains(i));
            (
                !started,
                if started { usize::MAX - n.depth } else { 0 },
                n.item_order,
                n.inst,
                p,
            )
        });
        let (mut slots, mut mem) = (limits.slots, limits.mem_mb);
        let mut out = Vec::new();
        for p in order {
            let id = &self.jobs[p].node.id;
            let mb = estimate(id);
            let res: Vec<String> = self
                .resources
                .iter()
                .find(|r| r.0 == *id)
                .map(|r| r.1.clone())
                .unwrap_or_default();
            if slots == 0 || mb > mem || res.iter().any(|r| held.contains(r)) {
                continue;
            }
            slots -= 1;
            mem -= mb;
            held.extend(res.iter().cloned());
            out.push(JobSpec {
                id: id.clone(),
                key: self.jobs[p].node.key.clone(),
                mb,
                resources: res,
            });
        }
        out
    }

    /// The executor started the job; false when it is not one `next` could give.
    pub fn started(&mut self, id: &JobId) -> bool {
        let Some(p) = self.at(id) else { return false };
        if !self.startable(&self.jobs[p]) {
            return false;
        }
        self.jobs[p].state = State::Running { frac: None };
        self.after();
        true
    }

    /// The share done of a running job (0 to 1); other jobs ignore it.
    pub fn progress(&mut self, id: &JobId, frac: f64) {
        if let Some(p) = self.at(id) {
            if let State::Running { frac: f } = &mut self.jobs[p].state {
                *f = Some(frac.clamp(0.0, 1.0));
                self.after();
            }
        }
    }

    /// A job ended. `error.featMemory` is a refusal (`refused`), `err.flow.cancelled` a cancellation, any other error is an `error`.
    pub fn finished(&mut self, id: &JobId, result: Result<Done, Error>) -> Landed {
        if let Some(p) = self.at(id) {
            if let Some(key) = self.jobs[p].zombie.take() {
                self.after();
                return Landed::Discarded { key };
            }
            if matches!(self.jobs[p].state, State::Running { .. }) {
                let j = &mut self.jobs[p];
                match result {
                    Ok(d) => {
                        j.state = State::Done;
                        j.has_output = true;
                        j.ms = d.ms;
                    }
                    Err(e) => {
                        let params = e
                            .params
                            .iter()
                            .map(|(k, v)| (k.to_string(), v.clone()))
                            .collect();
                        let key = e.key.to_string();
                        j.state = match e.key {
                            "err.flow.cancelled" => State::Cancelled,
                            "error.featMemory" => State::Refused { key, params },
                            _ => State::Error { key, params },
                        };
                    }
                }
                self.after();
                return Landed::Accepted;
            }
        }
        match self.ghosts.iter().position(|g| g.0 == *id) {
            Some(g) => {
                let key = self.ghosts.remove(g).1;
                Landed::Discarded { key }
            }
            None => Landed::Unknown,
        }
    }

    /// The `plan_key` of the job is in the cache: the job is `reused` without running. False when it is running or unknown.
    pub fn cached(&mut self, id: &JobId) -> bool {
        let Some(p) = self.at(id) else { return false };
        let j = &mut self.jobs[p];
        if matches!(j.state, State::Running { .. }) || j.zombie.is_some() || j.node.invalid {
            return false;
        }
        j.state = State::Reused;
        j.has_output = true;
        j.ms = 0.0;
        self.after();
        true
    }

    /// Holds a job that is not running (a permission is missing: `permission.memory`) until `unblock`.
    pub fn block(&mut self, id: &JobId, by: &str) -> bool {
        let Some(p) = self.at(id) else { return false };
        if matches!(self.jobs[p].state, State::Running { .. }) {
            return false;
        }
        self.jobs[p].held = Some(by.to_string());
        self.after();
        true
    }

    pub fn unblock(&mut self, id: &JobId) {
        if let Some(p) = self.at(id) {
            self.jobs[p].held = None;
            self.after();
        }
    }

    /// Puts a failed, refused or cancelled job back in line.
    pub fn retry(&mut self, id: &JobId) {
        if let Some(p) = self.at(id) {
            if matches!(
                self.jobs[p].state,
                State::Error { .. } | State::Refused { .. } | State::Cancelled
            ) {
                self.jobs[p].state = State::Waiting;
                self.after();
            }
        }
    }

    /// No new job starts while paused; the ones running go on.
    pub fn pause(&mut self) {
        self.paused = true;
        self.flush();
    }

    pub fn resume(&mut self) {
        self.paused = false;
        self.flush();
    }

    fn after(&mut self) {
        self.settle();
        self.flush();
    }
}

/// What the executor has free right now.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub slots: usize,
    pub mem_mb: f64,
}

/// A job to start: `mb` is the executor's estimate, `resources` the exclusive ones it declared for the job.
#[derive(Debug, Clone, PartialEq)]
pub struct JobSpec {
    pub id: JobId,
    pub key: String,
    pub mb: f64,
    pub resources: Vec<String>,
}

/// What `update` asks the executor to do; the new work comes from `next`.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Cancel(JobId),
}

/// What the executor measured for a job that finished.
#[derive(Debug, Clone, Copy)]
pub struct Done {
    pub ms: f64,
}

/// What became of a result: a result of a key that is no longer expected is not used, but it is a true result of that key (the caller caches it).
#[derive(Debug, Clone, PartialEq)]
pub enum Landed {
    Accepted,
    Discarded { key: String },
    Unknown,
}

struct Job {
    node: Node,
    state: State,
    /// There is an output, of the current key or of an older one.
    has_output: bool,
    /// Every job it reads is done or reused.
    go: bool,
    ms: f64,
    /// The key of a run that was cancelled but has not reported yet.
    zombie: Option<String>,
    /// Held by the caller (a permission is missing).
    held: Option<String>,
    shown: Option<(State, String, u64)>,
}

/// The scheduler; see the module header.
pub struct Sched {
    registry: Vec<Step>,
    program: String,
    identities: Vec<(String, String)>,
    plan: Plan,
    jobs: Vec<Job>,
    /// First problem of each instance, from `check`: (instance, blocked-by).
    problems: Vec<(String, &'static str)>,
    resources: Vec<(JobId, Vec<String>)>,
    /// Cancelled runs of jobs that are gone from the plan, with their key.
    ghosts: Vec<(JobId, String)>,
    gone: Vec<(JobId, String)>,
    rev: u64,
    seq: u64,
    log: VecDeque<(u64, String)>,
    paused: bool,
    shown_nodes: Vec<NodeView>,
}

/// Events kept in the log; a reader that is behind gets what is left.
const LOG_CAP: usize = 8192;

/// The fingerprint of a plan input: its `sha256`, else the text the caller gave for it.
fn identity_of(plan: &Plan, identities: &[(String, String)], name: &str) -> String {
    match plan.input(name) {
        Some(f) if !f.sha256.is_empty() => f.sha256.clone(),
        _ => identities
            .iter()
            .find(|i| i.0 == name)
            .map(|i| i.1.clone())
            .unwrap_or_default(),
    }
}

#[cfg(test)]
#[path = "sched_tests.rs"]
pub(in crate::flow) mod tests;
