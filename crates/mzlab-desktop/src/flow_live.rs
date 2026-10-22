//! The routes `flow/live/<op>` of `rpc`: a plan that runs on this computer and takes changes while it runs (`mzlab_cli::flow::live::LiveRun`), with the
//! very bodies and answers of `api/flow/sched/<op>` of the site (`new` is `start`) and `cancel`. The conversions the background queue (`conv_coda.rs`)
//! makes are the `convert.mzml` jobs of the plan: the workers leave them to the queue, which tells how they stand (`sync_coda`). One run at a time.

use std::path::PathBuf;
use std::sync::Mutex;

use mzlab_cli::flow::live::{default_workers, Lend, LiveRun};
use mzlab_core::flow::parse;
use mzlab_core::step::DataType;
use mzlab_core::{Error, Result};
use serde_json::Value;

use crate::engine::State;
use crate::flow_stage::stage_inputs;
use crate::rpc::{io_error, text, Answer};

/// The run in progress and the folder its inputs are staged in.
struct Live {
    run: LiveRun,
    inputs: PathBuf,
}

static LIVE: Mutex<Option<Live>> = Mutex::new(None);

fn lock() -> std::sync::MutexGuard<'static, Option<Live>> {
    LIVE.lock().unwrap_or_else(|e| e.into_inner())
}

fn no_run() -> Error {
    Error::new("err.flow.sched")
}

fn plan_text(p: &Value) -> Result<String> {
    match &p["plan"] {
        Value::String(t) => Ok(t.clone()),
        Value::Null => Err(Error::new("err.flow.plan")),
        v => Ok(v.to_string()),
    }
}

impl State {
    /// One call: `start {plan, files?, fresh?}` runs the plan (the inputs are staged as `flow/run` does); `update {plan, files?}` replaces it and
    /// answers the jobs to cancel; `events {since}`, `nodes`, `jobs`, `rev`, `pause`, `resume`, `retry {job}`, `unblock {job}` are the calls of the
    /// scheduler of the site; `cancel` stops what runs (the jobs end at their next piece; `retry` runs them again); `drop` forgets the run.
    pub(crate) fn flow_live(&self, op: &str, p: &Value) -> Result<Answer> {
        match op {
            "start" | "new" => return self.live_start(p, default_workers()),
            "drop" => {
                *lock() = None;
                return text("{}");
            }
            _ => {}
        }
        let mut g = lock();
        let live = g.as_mut().ok_or_else(no_run)?;
        if op == "update" {
            return self.live_update(live, p);
        }
        if op == "cancel" {
            live.run.cancel_all();
            return text("{}");
        }
        self.sync_coda(&live.run);
        let body = mzlab_core::json::parse(p.to_string().as_bytes())?;
        let answer = live
            .run
            .act(op, &body)
            .or_else(|| live.run.ask(op, &body))
            .ok_or_else(|| Error::new("err.route.unavailable"))?;
        text(answer)
    }

    fn live_start(&self, p: &Value, workers: usize) -> Result<Answer> {
        let plan = plan_text(p)?;
        let parsed = parse(plan.as_bytes())?;
        // the last run ends first: its threads stop and its folders are free
        *lock() = None;
        let root = std::env::temp_dir().join("mzlab-flow").join("live");
        let (inputs, work) = (root.join("in"), root.join("out"));
        let _ = std::fs::remove_dir_all(&inputs);
        if p["fresh"].as_bool().unwrap_or(false) {
            let _ = std::fs::remove_dir_all(&work);
        }
        for d in [&inputs, &work] {
            std::fs::create_dir_all(d).map_err(|e| io_error(&e))?;
        }
        let names: Vec<&str> = parsed.inputs.iter().map(|f| f.name.as_str()).collect();
        self.stage(&names, p, &inputs)?;
        let run = LiveRun::start(
            plan.as_bytes(),
            mzlab_cli::flow::steps(),
            &inputs,
            &work,
            workers,
        )?;
        self.claim_coda(&run);
        *lock() = Some(Live { run, inputs });
        text("{}")
    }

    fn live_update(&self, live: &mut Live, p: &Value) -> Result<Answer> {
        let plan = plan_text(p)?;
        let parsed = parse(plan.as_bytes())?;
        // an input the run has not seen yet is staged; one that is staged stays as it is
        let new: Vec<&str> = parsed
            .inputs
            .iter()
            .map(|f| f.name.as_str())
            .filter(|n| {
                let name = crate::store::clean(n).unwrap_or(n);
                p["files"][name].is_string() || live.inputs.join(name).symlink_metadata().is_err()
            })
            .collect();
        self.stage(&new, p, &live.inputs)?;
        let cancels = live.run.update(plan.as_bytes())?;
        self.claim_coda(&live.run);
        text(cancels)
    }

    fn stage(&self, names: &[&str], p: &Value, dir: &std::path::Path) -> Result<()> {
        // a maker's file whose mzML the background queue has already made is staged as that mzML
        let held = |name: &str| {
            self.conv_ready(name).or_else(|| {
                self.store
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .get(name)
                    .map(|e| e.path)
            })
        };
        stage_inputs(names, &p["files"], &held, dir)
    }

    /// The conversions of the plan whose file the queue holds are the queue's: the workers leave them.
    fn claim_coda(&self, run: &LiveRun) {
        let held = self.coda.list();
        let rows = held["jobs"].as_array().map_or(&[][..], Vec::as_slice);
        for id in run.jobs_of("convert.mzml") {
            let row = rows.iter().find(|r| r["name"].as_str() == id.item.as_deref());
            match row.and_then(|r| r["state"].as_str()) {
                Some("cancelled") | None => run.release(&id),
                Some(_) => run.claim(&[id]),
            }
        }
    }

    /// Tells the run how the queue stands: a file being converted is a job that runs, one whose mzML is ready is a job done (its file is the output),
    /// one that failed is a job in error; one the queue has dropped goes back to the workers.
    fn sync_coda(&self, run: &LiveRun) {
        self.claim_coda(run);
        let held = self.coda.list();
        for row in held["jobs"].as_array().into_iter().flatten() {
            let Some(name) = row["name"].as_str() else { continue };
            for id in run.jobs_of("convert.mzml") {
                if id.item.as_deref() != Some(name) {
                    continue;
                }
                match row["state"].as_str() {
                    Some("run") => {
                        run.lend(&id, Lend::Running(None));
                    }
                    Some("done") => {
                        if let Some(path) = self.conv_ready(name) {
                            run.lend(&id, Lend::Done(vec![("mzml", path, DataType::Mzml)]));
                        }
                    }
                    Some("failed") => {
                        let why = row["error"].as_str().unwrap_or("");
                        let e = Error::new("err.flow.convert").with("detail", why);
                        run.lend(&id, Lend::Failed(e));
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "flow_live_tests.rs"]
mod tests;
