//! The native executor of `features.group@1`: the `ionnet/1` of a per-ion `.mzfeat` (`mzlab_feat::ionnet::group_text`, the function the `flowtab`
//! `.wasm` calls in the browser). This file only reads and writes the files. Called by `flow.rs`.

use crate::flow::{read_one, write_made, Exec, FileRunner};
use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;
use mzlab_feat::ionnet::group_text;

/// The step of this file, chained by `flow::all_executors`.
pub(crate) const EXECUTORS: &[(&str, Exec)] =
    &[("features.group", Exec::File("groups", group_job))];

pub(crate) fn group_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let table = read_one(r, job, "table")?;
    let text = group_text(&job.resolved.params, &table.1)?;
    write_made(r, job, text, DataType::Report)
}
