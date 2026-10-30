//! The native executors of the regulatory screening: `screen.suspects@1` (mass path) and `annot.regulatory@1` (`list_membership`, structure path), file wrappers of
//! `mzlab_feat::flow_screen_suspects` and `mzlab_feat::flow_regulatory` (the one implementation each that the browser runs too).

use crate::flow::{read_all, read_one, write_made, FileRunner};
use crate::flow_read::{blobs, named};
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;
use mzlab_feat::flow_regulatory::{regulatory_text, Params as RegParams};
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

pub(crate) fn regulatory_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let alg = job
        .resolved
        .algorithm
        .as_ref()
        .map_or("list_membership", |a| a.0.as_str());
    // suspect_screen and ms2_confirm stay Attributio campaigns: screen.suspects is the mass path of a flow
    if alg != "list_membership" {
        return Err(Error::new("err.flow.regulatory.algorithm").with("algorithm", alg));
    }
    let annot = read_one(r, job, "annot")?.1;
    let package = read_one(r, job, "lists")?.1;
    let text = regulatory_text(
        &RegParams::from_params(&job.resolved.params),
        &annot,
        &package,
    )?;
    write_made(r, job, text, DataType::Annot)
}

#[cfg(test)]
#[path = "flow_screen_suspects_plan_tests.rs"]
mod plan_tests;
#[cfg(test)]
#[path = "flow_screen_suspects_reg_tests.rs"]
mod reg_tests;
#[cfg(test)]
#[path = "flow_screen_suspects_tests.rs"]
mod tests;
