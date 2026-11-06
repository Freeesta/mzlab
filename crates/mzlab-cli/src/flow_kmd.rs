//! The native executor of `table.kmd@1`: the Kendrick mass defect of every row of a `.mzfeat` and the candidate series of a repeating unit
//! (`stat::kmd_json`, the port of `kmd-calc.js`). Only the m/z and the RT of the rows are read; the text is shared with the browser. Called by `flow.rs`.

use crate::flow::{read_one, write_made, FileRunner};
use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;

/// The `kmd` text of the rows of `table` under the parameters of the job (`mzlab_feat::flow_stat::kmd_text`, shared with the browser).
pub fn kmd_text(job: &Job, table: &[u8]) -> Result<String> {
    mzlab_feat::flow_stat::kmd_text(&job.resolved.params, table)
}

pub(crate) fn kmd_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let table = read_one(r, job, "table")?;
    write_made(r, job, kmd_text(job, &table.1)?, DataType::Report)
}
