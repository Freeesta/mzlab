//! The native executor of `table.plausible@1`: the `plausible/1` of a `.mzfeat` (`plausible::to_json`), the same text as the quality-control step of the
//! page. The parameters of the step are the limits of `plausible::Limits`. Called by `flow.rs`.

use crate::flow::{read_one, write_made, Exec, FileRunner};
use mzlab_core::error::Result;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;
use mzlab_feat::plausible::{to_json, Limits};
use mzlab_feat::recipe::is_unit_text;
use mzlab_feat::store::read_table;

/// The one step of this file, chained by `flow::all_executors`.
pub(crate) const EXECUTORS: &[(&str, Exec)] =
    &[("table.plausible", Exec::File("report", plausible_job))];

/// The limits the job sets; a parameter without a value keeps the default of the engine.
fn limits(job: &Job) -> Limits {
    let get = |k: &str| {
        job.resolved
            .params
            .iter()
            .find(|(n, _)| n == k)
            .and_then(|(_, v)| v.f64())
    };
    let d = Limits::default();
    Limits {
        mz_min: get("mz_min").unwrap_or(d.mz_min),
        mz_max: get("mz_max").unwrap_or(d.mz_max),
        rt_max: get("rt_max").unwrap_or(d.rt_max),
        width_max: get("width_max").unwrap_or(d.width_max),
        ppm_max: get("ppm_max").unwrap_or(d.ppm_max),
        mz_tol_da: get("mz_tol_da"),
    }
}

/// The `plausible/1` text of `table` under the parameters of the job.
pub fn plausible_text(job: &Job, table: &[u8]) -> Result<String> {
    let (t, meta) = read_table(table)?;
    let unit = meta
        .get("recipe")
        .and_then(|v| v.str())
        .is_some_and(is_unit_text);
    Ok(to_json(&t, unit, &limits(job)))
}

pub(crate) fn plausible_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let table = read_one(r, job, "table")?;
    write_made(r, job, plausible_text(job, &table.1)?, DataType::Report)
}

#[cfg(test)]
#[path = "flow_plausible_tests.rs"]
mod tests;
