//! The native executor of `table.plausible@1`: the `plausible/1` of a `.mzfeat` (`plausible::plausible_text`), the same text as the quality-control step of the
//! page and of the browser executor. The parameters of the step are the limits of `plausible::Limits`. Called by `flow.rs`.

use crate::flow::{read_one, write_made, Exec, FileRunner};
use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;

/// The one step of this file, chained by `flow::all_executors`.
pub(crate) const EXECUTORS: &[(&str, Exec)] =
    &[("table.plausible", Exec::File("report", plausible_job))];

/// The `plausible/1` text of `table` under the parameters of the job.
pub fn plausible_text(job: &Job, table: &[u8]) -> Result<String> {
    mzlab_feat::plausible::plausible_text(&job.resolved.params, table)
}

pub(crate) fn plausible_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let table = read_one(r, job, "table")?;
    write_made(r, job, plausible_text(job, &table.1)?, DataType::Report)
}

#[cfg(test)]
#[path = "flow_plausible_tests.rs"]
mod tests;
