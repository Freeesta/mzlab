//! The native executor of `tp.position@1`: the position of the modification of every row that `tp.link` interpreted (the key `site` of `interp/1`),
//! again under the thresholds of the step (`interp::reposition`). With the default thresholds it is the site the interpretation stored; the votes of the
//! MSn tree are not in the file, so a row voted by the tree keeps its site whatever the thresholds.

use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;

use crate::flow::{read_one, write_made, FileRunner};
use crate::flow_link::parent_of;

fn param(job: &Job, name: &str) -> Option<f64> {
    job.resolved
        .params
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| v.f64())
}

/// `sites/1` text of the `.mzfeat` `tp` (with the block `interp`) for the parent file `parent`: the code the `.wasm` `interp` runs.
pub fn position_text(job: &Job, tp: &[u8], parent: &[u8]) -> Result<String> {
    let p = parent_of(parent)?;
    mzlab_feat_wasm::interp::position_text(&|n| param(job, n), tp, &p.smiles, &p.formula)
}

/// `tp.position`: `sites/1` written next to the other outputs of the run.
pub(crate) fn position_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (tp, parent) = (read_one(r, job, "tp")?, read_one(r, job, "parent")?);
    let text = position_text(job, &tp.1, &parent.1)?;
    write_made(r, job, text, DataType::Transform)
}
