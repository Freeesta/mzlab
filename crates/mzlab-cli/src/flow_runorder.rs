//! The native executor of `qc.run_order@1`: the facts and warnings of the injection sequence of the sample table. The text is `mzlab_feat::runorder::runorder_text`, shared with the browser;
//! this file only reads and writes the files. Called by `flow.rs`.

use crate::flow::{one_input, read_one, write_made, Exec, FileRunner};
use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;
use mzlab_feat::runorder::runorder_text;

pub(crate) const EXECUTORS: &[(&str, Exec)] =
    &[("qc.run_order", Exec::File("runorder", runorder_job))];

fn runorder_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let samples = read_one(r, job, "samples")?;
    let design = job
        .inputs
        .iter()
        .any(|(p, _)| p == "design")
        .then(|| read_one(r, job, "design"))
        .transpose()?;
    let text = runorder_text(
        &job.resolved.params,
        &samples.1,
        design.as_ref().map(|d| d.1.as_slice()),
    )?;
    write_made(r, job, text, DataType::Qc)
}
