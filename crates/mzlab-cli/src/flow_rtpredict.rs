//! The native executor of `rt.predict@1`: `mzlab_feat::flow_rtpredict::rtpredict_text` on the candidates (`candidates/1`), the standards of the campaign and the
//! optional observed RTs; the `.wasm` `attributio` calls the same function, so the browser writes the same bytes. Called by `flow.rs`.

use crate::flow::{all_inputs, read_one, write_made, Exec, FileRunner};
use crate::flow_read::load;
use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;

/// The executor of the step, read by the runner after its own list.
pub(crate) const EXECUTORS: &[(&str, Exec)] =
    &[("rt.predict", Exec::File("candidates", rtpredict_job))];

/// Runs the step of `job` on the candidates JSON, the standards CSV (`smiles,rt`) and the optional observed RTs (`query,rt`).
pub fn rtpredict_text(
    job: &Job,
    candidates: &[u8],
    standards: &[u8],
    queries: &[u8],
) -> Result<String> {
    let algorithm = job
        .resolved
        .algorithm
        .as_ref()
        .map_or("logp_window", |a| a.0.as_str());
    mzlab_feat::flow_rtpredict::rtpredict_text(
        &job.resolved.params,
        algorithm,
        candidates,
        standards,
        queries,
    )
}

pub(crate) fn rtpredict_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (cands, standards) = (
        read_one(r, job, "candidates")?,
        read_one(r, job, "standards")?,
    );
    let queries = match all_inputs(r, job, "queries").into_iter().next() {
        Some(p) => load(p)?.1,
        None => Vec::new(),
    };
    let text = rtpredict_text(job, &cands.1, &standards.1, &queries)?;
    write_made(r, job, text, DataType::Candidates)
}

#[cfg(test)]
#[path = "flow_rtpredict_tests.rs"]
mod tests;
