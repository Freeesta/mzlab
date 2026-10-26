//! The native executor of `tp.review@1`: the decisions of the chemist (`tpdec/1`) on the rows of the table of the products, checked with the rules of the
//! levels against what the interpretation proposes (`interp::review`, the code the `.wasm` `interp` runs). Writes `review/1`.

use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;

use crate::flow::{read_one, write_made, Exec, FileRunner};

/// `review/1` text of the `.mzfeat` `tp` (with the block `interp`) and the `tpdec/1` file `decisions`.
pub fn review_text(tp: &[u8], decisions: &[u8]) -> Result<String> {
    mzlab_feat_wasm::interp::review_text(tp, &String::from_utf8_lossy(decisions))
}

pub(crate) const EXECUTORS: &[(&str, Exec)] = &[("tp.review", Exec::File("review", review_job))];

fn review_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (tp, decisions) = (read_one(r, job, "tp")?, read_one(r, job, "decisions")?);
    let text = review_text(&tp.1, &decisions.1)?;
    write_made(r, job, text, DataType::Transform)
}

#[cfg(test)]
#[path = "flow_review_tests.rs"]
mod tests;
