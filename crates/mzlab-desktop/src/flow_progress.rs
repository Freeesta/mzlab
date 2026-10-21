//! The jobs a `flow/run` has finished so far, for the route `flow/progress` that the Flow page asks while a plan runs (one run at a time).
//! `flow_run` calls `start` and gives the result to the runner; the route answers with `snapshot`.

use mzlab_core::Result;
use serde_json::{json, Value};

use crate::engine::State;
use crate::rpc::{text, Answer};

static JOBS: std::sync::Mutex<Vec<Value>> = std::sync::Mutex::new(Vec::new());

/// Empties the list and gives the runner the function that fills it after each job.
pub(crate) fn start() -> mzlab_cli::flow::DoneFn {
    JOBS.lock().unwrap_or_else(|e| e.into_inner()).clear();
    std::sync::Arc::new(|id, item, ms, cached, err| {
        let job = json!({ "id": id, "item": item, "ms": ms, "cached": cached, "error": err });
        JOBS.lock().unwrap_or_else(|e| e.into_inner()).push(job);
    })
}

impl State {
    /// `{}` -> `{jobs: [{id, item, ms, cached, error}]}`: the jobs of the run in progress (or the last one) that have finished.
    pub(crate) fn flow_progress(&self, _p: &Value) -> Result<Answer> {
        let jobs = JOBS.lock().unwrap_or_else(|e| e.into_inner()).clone();
        text(json!({ "jobs": jobs }).to_string())
    }
}
