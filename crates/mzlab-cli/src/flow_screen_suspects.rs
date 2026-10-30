//! The native executor of `screen.suspects@1`: the file wrapper of `mzlab_feat::flow_screen_suspects` (the one implementation the browser runs too).

use crate::flow::{read_all, read_one, write_made, FileRunner};
use crate::flow_read::{blobs, named};
use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;
use mzlab_feat::flow_screen_suspects::{screen_text, Params};

pub(crate) fn screen_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let params = Params::from_params(&job.resolved.params);
    let has = |port: &str| job.inputs.iter().any(|(n, v)| n == port && !v.is_empty());
    let table = read_one(r, job, "table")?.1;
    let lists = read_all(r, job, "suspects")?;
    let spectra = if has("spectra") {
        Some(read_one(r, job, "spectra")?.1)
    } else {
        None
    };
    let formulas = if has("formulas") {
        read_all(r, job, "formulas")?
    } else {
        Vec::new()
    };
    let text = screen_text(
        &params,
        &table,
        &named(&lists),
        spectra.as_deref(),
        &blobs(&formulas),
    )?;
    write_made(r, job, text, DataType::Candidates)
}

#[cfg(test)]
#[path = "flow_screen_suspects_plan_tests.rs"]
mod plan_tests;
#[cfg(test)]
#[path = "flow_screen_suspects_tests.rs"]
mod tests;
