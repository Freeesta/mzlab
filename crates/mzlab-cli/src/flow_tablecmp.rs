//! The native executor of `table.compare@1`: every feature of a `.mzfeat` against the reference and blank columns (`mzlab_core::statcmp`), written as `compare/1`.
//! The samples file is optional (it gives roles and times; without it the times come from the file names). The text is the one `table compare` prints and the
//! browser writes (`mzlab_feat::flow_compare`). Called by `flow.rs`.

use crate::flow::{read_one, write_made, Exec, FileRunner};
use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;

/// The step of this file, chained into the list of `flow.rs`.
pub(crate) const EXECUTORS: &[(&str, Exec)] =
    &[("table.compare", Exec::File("report", compare_job))];

fn compare_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let table = read_one(r, job, "table")?;
    let wired = job
        .inputs
        .iter()
        .any(|(p, v)| p == "samples" && !v.is_empty());
    let samples = if wired {
        Some(read_one(r, job, "samples")?)
    } else {
        None
    };
    let text = mzlab_feat::flow_compare::compare_text(
        &job.resolved.params,
        &table.0,
        &table.1,
        samples.as_ref().map(|s| s.1.as_slice()),
    )?;
    write_made(r, job, text, DataType::Report)
}
