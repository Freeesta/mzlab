//! The `flowlive/1` events of the live scheduler (`docs/schemi/flowlive-1.json`): the states, the aggregate of a node and the JSON text of one event.
//! Written with `json::Out`, no serde; the text of an event is fixed when it is made, so the log never changes after the fact.

use super::graph::JobId;
use crate::json::Out;

/// Where a job is. `Stale` = the job has an output of an older key (shown dimmed) and is run again as soon as its inputs are done.
#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Waiting,
    Ready,
    Running {
        frac: Option<f64>,
    },
    Done,
    Reused,
    Stale,
    Cancelled,
    Error {
        key: String,
        params: Vec<(String, String)>,
    },
    Refused {
        key: String,
        params: Vec<(String, String)>,
    },
    Blocked {
        by: String,
    },
    Skipped,
    Paused,
}

impl State {
    pub fn name(&self) -> &'static str {
        match self {
            State::Waiting => "waiting",
            State::Ready => "ready",
            State::Running { .. } => "running",
            State::Done => "done",
            State::Reused => "reused",
            State::Stale => "stale",
            State::Cancelled => "cancelled",
            State::Error { .. } => "error",
            State::Refused { .. } => "refused",
            State::Blocked { .. } => "blocked",
            State::Skipped => "skipped",
            State::Paused => "paused",
        }
    }
    /// Badness of a state for the aggregate of a node: the worst job names the node.
    pub(super) fn rank(&self) -> u8 {
        match self {
            State::Error { .. } => 11,
            State::Refused { .. } => 10,
            State::Blocked { .. } => 9,
            State::Skipped => 8,
            State::Running { .. } => 7,
            State::Stale => 6,
            State::Paused => 5,
            State::Ready => 4,
            State::Cancelled => 3,
            State::Waiting => 2,
            State::Reused => 1,
            State::Done => 0,
        }
    }
    /// Share of the work done, as the job itself knows it.
    pub(super) fn frac(&self) -> Option<f64> {
        match self {
            State::Done | State::Reused => Some(1.0),
            State::Running { frac } => *frac,
            _ => Some(0.0),
        }
    }
    fn error(&self) -> Option<(&str, &[(String, String)])> {
        match self {
            State::Error { key, params } | State::Refused { key, params } => Some((key, params)),
            _ => None,
        }
    }
}

/// The card of an instance: the worst state of its jobs, the mean share done, how many jobs are done of how many, the summed milliseconds.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeView {
    pub instance: String,
    pub state: State,
    pub frac: Option<f64>,
    pub parte: usize,
    pub di: usize,
    pub ms: f64,
}

fn head(o: &mut Out, seq: u64, rev: u64, kind: &str) {
    o.raw("{\"format\":\"flowlive/1\",\"seq\":")
        .int(seq as i64)
        .raw(",\"rev\":")
        .int(rev as i64)
        .raw(",\"kind\":")
        .str(kind);
}

fn blocked_by(o: &mut Out, state: &State) {
    o.raw(",\"blocked_by\":");
    match state {
        State::Blocked { by } => o.str(by),
        _ => o.raw("null"),
    };
}

fn error(o: &mut Out, state: &State) {
    o.raw(",\"error\":");
    let Some((key, params)) = state.error() else {
        o.raw("null");
        return;
    };
    o.raw("{\"error_key\":").str(key).raw(",\"params\":{");
    for (i, (k, v)) in params.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.str(k).raw(":").str(v);
    }
    o.raw("}}");
}

/// A job event: `ms` is None until the job is done.
pub(super) fn job_event(
    seq: u64,
    rev: u64,
    id: &JobId,
    state: &State,
    ms: Option<f64>,
    key: &str,
) -> String {
    let mut o = Out::default();
    head(&mut o, seq, rev, "job");
    o.raw(",\"instance\":").str(&id.instance).raw(",\"item\":");
    match &id.item {
        Some(n) => o.str(n),
        None => o.raw("null"),
    };
    o.raw(",\"state\":").str(state.name()).raw(",\"frac\":");
    match state {
        State::Running { frac } => o.opt(*frac),
        _ => o.opt(state.frac()),
    };
    o.raw(",\"ms\":").opt(ms).raw(",\"key\":").str(key);
    blocked_by(&mut o, state);
    error(&mut o, state);
    o.raw("}");
    o.0
}

pub(super) fn node_event(seq: u64, rev: u64, n: &NodeView) -> String {
    let mut o = Out::default();
    head(&mut o, seq, rev, "node");
    o.raw(",\"instance\":")
        .str(&n.instance)
        .raw(",\"item\":null,\"state\":");
    o.str(n.state.name()).raw(",\"frac\":").opt(n.frac);
    o.raw(",\"parte\":")
        .int(n.parte as i64)
        .raw(",\"di\":")
        .int(n.di as i64);
    o.raw(",\"ms\":").opt(Some(n.ms));
    blocked_by(&mut o, &n.state);
    error(&mut o, &n.state);
    o.raw("}");
    o.0
}

/// The event of a plan that cannot be scheduled at all (a cycle): `error_key` says why.
pub(super) fn plan_event(seq: u64, rev: u64, error_key: &str) -> String {
    let mut o = Out::default();
    head(&mut o, seq, rev, "plan");
    o.raw(",\"instance\":\"\",\"state\":\"blocked\",\"blocked_by\":\"plan.problem\",\"error\":{\"error_key\":")
        .str(error_key)
        .raw(",\"params\":{}}}");
    o.0
}
