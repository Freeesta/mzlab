//! The native executor of `spectra.match@1`: the file wrapper of the matcher in `mzlab-lib` (`flowmatch`, the one implementation the browser runs too). It reads the
//! `Job` into the parameters the matcher takes and sends the `tree` algorithm, which reads `msnlib/1` texts, to `flow_match_tree`.

use mzlab_core::error::Result;
use mzlab_core::flow::Job;
use mzlab_lib::flowmatch::MatchJob;
pub use mzlab_lib::flowmatch::{library_spectra, query_id, read_spectra, Named};

pub(crate) fn num(job: &Job, name: &str) -> f64 {
    job.resolved
        .params
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| v.f64())
        .unwrap_or(0.0)
}

/// Runs the step of `job` on the query and library files and returns the text of the `candidates` file: one object per candidate, queries in file order,
/// each query's candidates best score first, at most `top_n` of them.
pub fn match_text(job: &Job, query: Named, library: Named) -> Result<String> {
    let alg = job
        .resolved
        .algorithm
        .as_ref()
        .map_or("spectral_entropy", |a| a.0.as_str());
    let decoys = job
        .resolved
        .params
        .iter()
        .any(|(k, v)| k == "decoys" && v.str() == Some("spectrum"));
    if alg == "tree" && !decoys {
        return crate::flow_match_tree::tree_text(job, query, library);
    }
    let m = MatchJob {
        step: &job.step.id,
        algorithm: Some(alg),
        params: &job.resolved.params,
    };
    mzlab_lib::flowmatch::match_text(&m, query, library)
}

#[cfg(test)]
#[path = "flow_match_decoy_tests.rs"]
mod decoy_tests;
