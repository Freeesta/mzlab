//! The native executor of `table.filter@1`: the view (`view_json`, the one the page asks of the table `.wasm`) of a `.mzfeat` under the rules the
//! parameters of the step spell, in the language of the engine (`filtra`/`normalizza`). The roles come from the samples file, `campioni/1` or `campioni/2`.

use crate::flow::{read_one, write_made, FileRunner};
use mzlab_core::error::Result;
use mzlab_core::flow::Artifact;
use mzlab_core::flow::Job;
use mzlab_core::step::DataType;
use mzlab_feat::store::Slice;
use mzlab_feat_table_wasm::core::view_json;
use mzlab_feat_table_wasm::filterspec;

/// The samples as `campioni/1` text, whichever of the two schemas the file has (the rule is shared with the browser).
pub(crate) fn samples_text(bytes: &[u8]) -> Result<String> {
    mzlab_feat_table_wasm::pol::samples_v1(bytes)
}

/// The `view` text of the table under the rules of the job.
pub fn filter_text(job: &Job, table: &[u8], samples: &[u8]) -> Result<String> {
    let samples =
        &mzlab_feat_table_wasm::pol::samples_like(&mut Slice(table), table.len() as u64, samples)?;
    let samples = samples.as_bytes();
    let spec = filterspec::spec_with_samples(&job.resolved.params, samples);
    let samples = samples_text(samples)?;
    view_json(&mut Slice(table), table.len() as u64, &samples, &spec)
}

pub(crate) fn filter_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (table, samples) = (read_one(r, job, "table")?, read_one(r, job, "samples")?);
    let text = filter_text(job, &table.1, &samples.1)?;
    write_made(r, job, text, DataType::Table)
}
