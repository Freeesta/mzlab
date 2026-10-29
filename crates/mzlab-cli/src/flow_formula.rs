//! The native executors of `formula.ms1@1` and `formula.ms2@1`: the file wrappers of the texts in `mzlab-feat` (`flow_formula_ms1`, `flow_formula_ms2`, the one implementation the
//! browser runs too). Both write JSON `formulas/1`.

use mzlab_core::error::Result;
use mzlab_core::flow::Job;
use mzlab_feat::flow_ms2::Named;

/// `formula.ms1`: for each feature the MS1 scan at the apex, in the file where the feature is largest, goes to `formula_id::identify`.
pub fn ms1_text(job: &Job, features: &[u8], files: &[Named]) -> Result<String> {
    mzlab_feat::flow_formula_ms1::ms1_text(&job.resolved.params, features, files)
}

/// `formula.ms2`: every candidate formula whose ion fits the precursor of a spectrum explains its cleaned peaks with sub-formulas.
pub fn ms2_text(job: &Job, spectra: Named, formulas: &[&[u8]]) -> Result<String> {
    let alg = job.resolved.algorithm.as_ref().map_or("", |a| a.0.as_str());
    mzlab_feat::flow_formula_ms2::ms2_text(alg, &job.resolved.params, spectra, formulas)
}
