//! The native executor of `features.estimate@1`: for each indexed file the `estimates/1` text of `mzlab_feat_wasm::run_estimate` (the function the page's
//! `flow_estimate` calls, so both write the same bytes) from the parameters the plan writes; what the file decided goes to the estimates of prov. Called by `flow.rs`.

use mzlab_core::error::Result;
use mzlab_core::flow::Job;
use mzlab_core::json::Value;
use mzlab_core::step::DataType;
use mzlab_feat_wasm::run_estimate::estimate_of;

use crate::flow::{io, FileRunner, Made};
use crate::flow_base::{open, out_path};
use crate::flow_nature::written;

/// `features.estimate`: one `estimates/1` file per indexed file, measured on the file's own data (its own profile and polarity, then what the plan writes).
pub(crate) fn estimate_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let input = &job.inputs[0].1[0];
    let mut index = open(&r.path(input))?;
    // a default the descriptor fills in is not a choice of the plan
    let given: Vec<(String, Value)> = job
        .resolved
        .params
        .iter()
        .filter(|(k, _)| written(job, k))
        .cloned()
        .collect();
    let (text, estimates) = estimate_of(&mut index, &r.source_name(input), &given)?;
    let out = out_path(r, job, "json");
    std::fs::write(&out, text).map_err(io)?;
    Ok(Made {
        estimates,
        ..Made::new("estimates", r.keep(out, DataType::Qc)?)
    })
}
