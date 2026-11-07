//! The native executor of `table.pca@1`: the exploratory PCA of the areas of a `.mzfeat` (`stat::pca_json`, the engine of the route `api/stat/pca`), asked
//! as the Pca page asks; the text is `mzlab_feat::flow_stat::pca_text`, shared with the browser, this file only reads and writes. Called by `flow.rs`.

use crate::flow::{read_one, write_made, FileRunner};
use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;

/// The answer of the PCA engine for the areas of `table` and the roles of `samples` (`mzlab_feat::flow_stat::pca_text`, shared with the browser).
pub fn pca_text(job: &Job, table: &[u8], samples: &[u8]) -> Result<String> {
    mzlab_feat::flow_stat::pca_text(&job.resolved.params, table, samples)
}

pub(crate) fn pca_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (table, samples) = (read_one(r, job, "table")?, read_one(r, job, "samples")?);
    let text = pca_text(job, &table.1, &samples.1)?;
    write_made(r, job, text, DataType::Report)
}
