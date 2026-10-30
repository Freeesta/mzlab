//! The nature of the files of a series for `flow run` (`mzlab_feat::nature`, the rule the browser runner shares): `tp.generate` and `tp.link@2` use it for
//! what the plan leaves unwritten (`polarity`, `nominal`), so a negative or a unit-resolution series needs no setting by hand.

use std::path::PathBuf;

use mzlab_core::flow::Job;
use mzlab_core::json::Value;
pub use mzlab_feat::nature::Nature;

use crate::flow_base::open;

/// The nature of the series of `paths` (a file that does not open counts for nothing).
pub fn of_files(paths: &[PathBuf]) -> Nature {
    let ix: Vec<_> = paths.iter().filter_map(|p| open(p).ok()).collect();
    mzlab_feat::nature::of_indexes(&ix)
}

/// Whether the instance of the plan writes the parameter (a value the descriptor fills in is not a choice of the plan).
pub fn written(job: &Job, name: &str) -> bool {
    job.instance.params.iter().any(|(k, _)| k == name)
}

/// The estimates of `tp.generate`: the polarity and the unit resolution of the files, when the plan does not write them.
pub fn estimates(job: &Job, nature: &Nature) -> Vec<(String, Value, Value)> {
    mzlab_feat::nature::estimates(&|n| written(job, n), nature)
}
