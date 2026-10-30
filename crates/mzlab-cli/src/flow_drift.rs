//! The native executor of `table.drift@1`: the correction of the drift of the areas along the order of injection. The `.mzfeat` is never changed: the output is `deriva/1`
//! (the summary of the engine) plus the factor of every cell. The text is `mzlab_feat::flow_drift::drift_text`, shared with the browser; this file only reads and writes the files. Called by `flow.rs`.

use crate::flow::{read_one, write_made, FileRunner};
use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;
use mzlab_feat::flow_drift::drift_text;

pub(crate) fn drift_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let table = read_one(r, job, "table")?;
    let (samples, design) = (read_one(r, job, "samples")?, read_one(r, job, "design")?);
    let text = drift_text(&job.resolved.params, &table.1, &samples.1, &design.1)?;
    write_made(r, job, text, DataType::Table)
}
