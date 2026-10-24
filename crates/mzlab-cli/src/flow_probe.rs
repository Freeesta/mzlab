//! `probe_plan`: how many steps of a plan the cache of a run folder would give back, without running or writing anything. The desktop route
//! `flow/reuse` uses it so the Flow page can say "N steps reused, M to compute" before Run.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use mzlab_core::flow::parse;

use crate::flow::{steps, DoneFn, FileRunner};
use crate::flow_run::{fill_inputs, prepare, problem_text};

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
