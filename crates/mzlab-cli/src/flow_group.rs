//! The native executor of `features.group@1`: the `ionnet/1` of a per-ion `.mzfeat` (`mzlab_feat::ionnet::group_text`, the function the `flowtab`
//! `.wasm` calls in the browser), within the memory budget of the runner; its warnings go to the record. This file only reads and writes the files.
//! Called by `flow.rs`.

use crate::flow::{read_one, write_made, Exec, FileRunner};
use crate::flow_made::Made;
use mzlab_core::budget::MemBudget;
use mzlab_core::error::Result;
use mzlab_core::flow::Job;
use mzlab_core::step::DataType;
use mzlab_feat::ionnet::group_text;

/// The step of this file, chained by `flow::all_executors`.
pub(crate) const EXECUTORS: &[(&str, Exec)] = &[("features.group", Exec::Made(group_job))];

pub(crate) fn group_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let table = read_one(r, job, "table")?;
    let g = group_text(
        &job.resolved.params,
        &table.1,
        &MemBudget::mb(r.budget.job_mb()),
    )?;
    let mut made = Made::new("groups", write_made(r, job, g.text, DataType::Report)?);
    made.warnings = g.warnings.into_iter().map(String::from).collect();
    Ok(made)
}
