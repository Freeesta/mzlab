//! The native executor of `structure.reactivity@1`: `reatt/1` of the parent (`mzlab_chem::flow::reactivity_text`, the same text as the `.wasm` `chem` in
//! the browser), with the solvent, the conformers, the form of the parent and the seed of the step as its options.

use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;

use crate::flow::{read_one, write_made, FileRunner};

/// `reatt/1` text of the parent file under the options of the step.
pub fn reactivity_text(job: &Job, parent: &[u8]) -> Result<String> {
    mzlab_chem::flow::reactivity_text(&job.resolved.params, parent)
}

/// `structure.reactivity`: `reatt/1` written next to the other outputs of the run.
pub(crate) fn reactivity_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let parent = read_one(r, job, "parent")?;
    write_made(r, job, reactivity_text(job, &parent.1)?, DataType::Annot)
}
