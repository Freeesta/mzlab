//! The run folder of `mzlab-cli flow run`: `plan.json` (the plan as run), the files of the steps, `prov.json` and `corsa.json`, and the exit code.
//! Called by `flow_run.rs` once the plan, the settings and the inputs are checked.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use mzlab_core::flow::{plan_json, plan_order, Plan, Run};

use crate::flow::{FileRunner, Meter};
use crate::flow_run::{fail, run_checked};
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
    (fresh, budget_mb): (bool, Option<f64>),
) -> Outcome {
    if let Err(e) = std::fs::create_dir_all(&out) {
        return fail(
            2,
            &format!("{}: {e}", out.display()),
            "--out must be a folder that can be written",
        );
    }
    let text = plan_json(plan);
    let order: Vec<String> = plan_order(ready)
        .unwrap_or_default()
        .into_iter()
        .map(|k| ready.steps[k].id.clone())
        .collect();
    let steps = order.len();
    let runner = FileRunner::located(&out, found, order, meter, fresh).with_budget(budget_mb);
    let written = std::fs::write(out.join("plan.json"), &text);
    let (run, prov) = match written
        .map_err(crate::flow::io)
        .and_then(|_| run_checked(ready, text.as_bytes(), &runner, workers))
    {
        Ok(r) => r,
        Err(e) => {
            return fail(
                3,
                &e.to_string(),
                "fix the plan in the Flow page or with --set",
            )
        }
    };
    if let Err(e) = std::fs::write(out.join("prov.json"), &prov) {
        return fail(
            2,
            &format!("prov.json: {e}"),
            "--out must be a folder that can be written",
        );
    }
    let corsa = crate::flow_report::report(&out)
        .and_then(|c| std::fs::write(out.join("corsa.json"), c).map_err(crate::flow::io));
    if let Err(e) = corsa {
        return fail(
            1,
            &format!("corsa.json: {e}"),
            "prov.json and the files of the steps are written",
        );
    }
    summary(&run, steps, &out)
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
