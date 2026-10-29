//! The native executor of `annot.level@1`: the file wrapper of the rule in `mzlab-feat` (`flow_level`, the one implementation the browser runs too).

use crate::flow::{read_all, write_made, FileRunner};
use crate::flow_read::blobs;
use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;

use mzlab_feat::flow_level::level_text;
#[cfg(test)]
pub use mzlab_feat::flow_level::{assess, Analog, Cand, Evidence};

pub(crate) fn level_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (hits, formulas) = (
        read_all(r, job, "candidates")?,
        read_all(r, job, "formulas")?,
    );
    let text = level_text(&blobs(&hits), &blobs(&formulas))?;
    write_made(r, job, text, DataType::Annot)
}
