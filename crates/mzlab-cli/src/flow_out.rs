//! The run folder of `mzlab-cli flow run`: `plan.json` (the plan as run), the files of the steps, `prov.json` and `corsa.json`, and the exit code.
//! Called by `flow_run.rs` once the plan, the settings and the inputs are checked; `probe_plan` counts the cache hits of a plan (`flow/reuse`).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use mzlab_core::flow::{parse, plan_json, plan_order, Plan, Run};

use crate::flow::live::LiveRun;
use crate::flow::{steps, DoneFn, FileRunner, Meter};
use mzlab_core::budget::sha256_hex;
use mzlab_core::error::Error;

use crate::flow_json::{fail, Style, JSONL_MARK};
use crate::flow_run::{fill_inputs, prepare, problem_text, prov_text, run_checked};
use crate::Outcome;

/// Runs `ready` (the checked, expanded form of `plan`) with the inputs `found` and writes the run folder `out`; the steps whose result is already in
/// the cache of `out` are taken again, unless `fresh`; `budget_mb` fixes the memory of the alignment.
pub(crate) fn write_run(
    plan: &Plan,
    ready: &Plan,
    found: HashMap<String, PathBuf>,
    out: PathBuf,
    workers: usize,
    meter: Option<Meter>,
    (fresh, budget_mb, st): (bool, Option<f64>, Style),
) -> Outcome {
    if let Err(e) = std::fs::create_dir_all(&out) {
        let err = Error::new("err.cli.out").with("path", out.display());
        let msg = format!("{}: {e}", out.display());
        return fail(
            st,
            2,
            &err,
            &msg,
            "--out must be a folder that can be written",
        );
    }
    let text = plan_json(plan);
    let mut order: Vec<String> = plan_order(ready)
        .unwrap_or_default()
        .into_iter()
        .map(|k| ready.steps[k].id.clone())
        .collect();
    let steps = order.len();
    if st.jsonl {
        order.push(JSONL_MARK.into());
    }
    let runner = FileRunner::located(&out, found, order, meter, fresh).with_budget(budget_mb);
    let written = std::fs::write(out.join("plan.json"), &text);
    let ran = written.map_err(crate::flow::io).and_then(|_| {
        if st.events {
            run_events(ready, text.as_bytes(), runner, &out, workers)
        } else {
            run_checked(ready, text.as_bytes(), &runner, workers)
        }
    });
    let (run, prov) = match ran {
        Ok(r) => r,
        Err(e) => {
            return fail(
                st,
                3,
                &e,
                &e.to_string(),
                "fix the plan in the Flow page or with --set",
            )
        }
    };
    if let Err(e) = std::fs::write(out.join("prov.json"), &prov) {
        let err = Error::new("err.cli.out").with("path", out.join("prov.json").display());
        return fail(
            st,
            2,
            &err,
            &format!("prov.json: {e}"),
            "--out must be a folder that can be written",
        );
    }
    let corsa = crate::flow_report::report(&out)
        .and_then(|c| std::fs::write(out.join("corsa.json"), c).map_err(crate::flow::io));
    if let Err(e) = corsa {
        let err = Error::new("err.cli.out").with("path", out.join("corsa.json").display());
        let hint = "prov.json and the files of the steps are written";
        return fail(st, 1, &err, &format!("corsa.json: {e}"), hint);
    }
    let mut o = summary(&run, steps, &out);
    if st.json {
        let plan_sha = sha256_hex(text.as_bytes());
        o.stdout = crate::flow_json::run_json(o.code, &out, &plan_sha, &run).into_bytes();
    }
    o
}

/// The exit code and the last lines on stderr: every step done, or the ones that failed or did not run.
fn summary(run: &Run, steps: usize, out: &Path) -> Outcome {
    let failed: Vec<&str> = run
        .activities
        .iter()
        .filter(|a| a.error.is_some())
        .map(|a| a.instance.as_str())
        .collect();
    let mut stderr = String::new();
    let code = if failed.is_empty() && run.skipped.is_empty() {
        stderr += &format!("done: {steps} steps; outputs in {}\n", out.display());
        0
    } else {
        stderr += &format!(
            "error: failed {}; not run {}\n",
            failed.join(", "),
            run.skipped.join(", ")
        );
        stderr += &format!(
            "hint: the other outputs, prov.json and corsa.json are in {}\n",
            out.display()
        );
        1
    };
    Outcome {
        code,
        stdout: Vec::new(),
        stderr,
    }
}

/// `run_plan` that keeps the cache of `work` (`Some(fresh)`; `fresh` recomputes every step): a step whose key is there is taken again (`cached`).
pub fn run_plan_cached(
    text: &[u8],
    dir: &Path,
    work: &Path,
    workers: usize,
    meter: Option<Meter>,
    fresh: Option<bool>,
) -> mzlab_core::Result<(Run, String)> {
    run_plan_notify(text, dir, work, workers, meter, fresh, None)
}

/// `run_plan_cached` that also calls `done` after each job, for a caller that shows the run while it goes.
pub fn run_plan_notify(
    text: &[u8],
    dir: &Path,
    work: &Path,
    workers: usize,
    meter: Option<Meter>,
    fresh: Option<bool>,
    done: Option<DoneFn>,
) -> mzlab_core::Result<(Run, String)> {
    let mut plan = parse(text)?;
    fill_inputs(&mut plan, |n| dir.join(crate::flow_set::dataset_name(n)))?;
    let plan = prepare(plan, &steps())?.map_err(|problems| {
        let list: Vec<String> = problems.iter().map(problem_text).collect();
        mzlab_core::Error::new("err.flow.check").with("problems", list.join(" "))
    })?;
    let runner = FileRunner::cached(dir, work, meter, fresh).with_done(done);
    run_checked(&plan, text, &runner, workers)
}

/// Runs the checked plan `ready` like `run_checked` and prints the `flowlive/1` events on stdout, one line each, as they happen (`flow run --jsonl`);
/// returns the run and its `prov/1` text (`text` is the plan as it was read).
pub(crate) fn run_events(
    ready: &Plan,
    text: &[u8],
    runner: FileRunner,
    dir: &Path,
    workers: usize,
) -> mzlab_core::Result<(Run, String)> {
    use std::io::Write;
    let live = LiveRun::with_runner(ready.clone(), steps(), runner, dir, (workers, &|_| false))?;
    let mut seq = 0;
    loop {
        let idle = live.wait_idle(std::time::Duration::from_millis(100));
        let mut out = std::io::stdout().lock();
        for (n, line) in live.event_lines(seq) {
            let _ = writeln!(out, "{line}");
            seq = n;
        }
        let _ = out.flush();
        if idle {
            let run = live.finish()?;
            let prov = prov_text(text, &run);
            return Ok((run, prov));
        }
    }
}

/// `(reused, to_compute)` in steps of the plan (not jobs: a per-file step counts once). A step is reused when every one of its jobs is in the
/// cache; the steps after a miss cannot be known to be reused, so they count as to compute. The index of the cache is never written.
pub fn probe_plan(text: &[u8], dir: &Path, work: &Path) -> mzlab_core::Result<(usize, usize)> {
    let mut plan = parse(text)?;
    fill_inputs(&mut plan, |n| dir.join(crate::flow_set::dataset_name(n)))?;
    let plan = prepare(plan, &steps())?.map_err(|problems| {
        let list: Vec<String> = problems.iter().map(problem_text).collect();
        mzlab_core::Error::new("err.flow.check").with("problems", list.join(" "))
    })?;
    let seen = Arc::new(Mutex::new(HashMap::<String, bool>::new()));
    let log = seen.clone();
    let done: DoneFn = Arc::new(move |id, _, _, cached, _| {
        let mut m = log.lock().unwrap_or_else(|e| e.into_inner());
        let all = m.entry(id.to_string()).or_insert(true);
        *all &= cached;
    });
    let mut runner = FileRunner::cached(dir, work, None, Some(false)).with_done(Some(done));
    runner.probe = true;
    // not `run_checked`: it saves the cache, and a probe leaves the index as it is
    mzlab_core::flow::execute(&plan, &steps(), &runner, 1)?;
    let seen = seen.lock().unwrap_or_else(|e| e.into_inner());
    let reused = seen.values().filter(|c| **c).count();
    let total = plan
        .steps
        .iter()
        .map(|s| s.id.as_str())
        .collect::<HashSet<_>>()
        .len();
    Ok((reused, total.saturating_sub(reused)))
}
