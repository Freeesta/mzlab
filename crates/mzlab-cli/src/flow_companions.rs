//! The native executor of `features.companions@1`: the `companions/1` of a unit-mass `.mzfeat` (`mzlab_feat::companions::companions_text`, the function the
//! `flowtab` `.wasm` calls in the browser), its warnings going to the record. This file only reads and writes the files. Called by `flow.rs`.

use crate::flow::{read_one, write_made, Exec, FileRunner};
use crate::flow_made::Made;
use mzlab_core::error::Result;
use mzlab_core::flow::Job;
use mzlab_core::step::DataType;
use mzlab_feat::companions::companions_text;

/// The step of this file, chained by `flow::all_executors`.
pub(crate) const EXECUTORS: &[(&str, Exec)] =
    &[("features.companions", Exec::Made(companions_job))];

pub(crate) fn companions_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let table = read_one(r, job, "table")?;
    let c = companions_text(&job.resolved.params, &table.1)?;
    let mut made = Made::new("companions", write_made(r, job, c.text, DataType::Report)?);
    made.warnings = c.warnings.into_iter().map(String::from).collect();
    Ok(made)
}
