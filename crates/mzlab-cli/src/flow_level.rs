//! The native executor of `annot.level@1`: the file wrapper of the rule in `mzlab-feat` (`flow_level`, the one implementation the browser runs too).

use crate::flow::{read_all, read_one, write_made, FileRunner};
use crate::flow_read::blobs;
use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;

#[cfg(test)]
pub use mzlab_feat::flow_level::{assess_with, Analog, Cand, Evidence, Std, Suspect};
use mzlab_feat::flow_level::{level_text_with, Inputs, Params};

pub(crate) fn level_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (hits, formulas) = (
        read_all(r, job, "candidates")?,
        read_all(r, job, "formulas")?,
    );
    let has = |port: &str| job.inputs.iter().any(|(n, v)| n == port && !v.is_empty());
    let standards = if has("standards") {
        Some(read_one(r, job, "standards")?.1)
    } else {
        None
    };
    let spectra = if has("spectra") {
        Some(read_one(r, job, "spectra")?.1)
    } else {
        None
    };
    let (hits, formulas) = (blobs(&hits), blobs(&formulas));
    let text = level_text_with(
        &Params::from_params(&job.resolved.params),
        &Inputs {
            hits: &hits,
            formulas: &formulas,
            standards: standards.as_deref(),
            spectra: spectra.as_deref(),
        },
    )?;
    write_made(r, job, text, DataType::Annot)
}
