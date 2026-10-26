//! The native executor of `tp.filter@2`: the registry of the filters `filtri/1` of the table that `tp.link` interpreted, under the thresholds of the step
//! (`interp::refilter`). With the default thresholds it is the registry the interpretation stored in the meta of the `.mzfeat`.

use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;

use crate::flow::{read_one, write_made, FileRunner};

fn param(job: &Job, name: &str) -> Option<f64> {
    job.resolved
        .params
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| v.f64())
}

/// `filtri/1` text of the `.mzfeat` `tp` (with the block `interp`) and the `ipotesi/1` `hypotheses`: the code the `.wasm` `interp` runs.
pub fn refilter_text(job: &Job, tp: &[u8], hypotheses: &[u8]) -> Result<String> {
    mzlab_feat_wasm::interp::refilter_text(
        &|n| param(job, n),
        tp,
        &String::from_utf8_lossy(hypotheses),
    )
}

pub(crate) fn refilter_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (tp, hypotheses) = (read_one(r, job, "tp")?, read_one(r, job, "hypotheses")?);
    let text = refilter_text(job, &tp.1, &hypotheses.1)?;
    write_made(r, job, text, DataType::Table)
}

/// `tp.review` reads the same table as `tp.filter`, so its executor sits beside it.
#[path = "flow_review.rs"]
pub(crate) mod review;
