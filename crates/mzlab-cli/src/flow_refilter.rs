//! The native executor of `tp.filter@2`: the registry of the filters `filtri/1` of the table that `tp.link` interpreted, under the thresholds of the step
//! (`interp::refilter`). With the default thresholds it is the registry the interpretation stored in the meta of the `.mzfeat`.

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::step::DataType;
use mzlab_feat::interp::filtri::DecoyParams;
use mzlab_feat::interp::json::filtri_json;
use mzlab_feat::interp::refilter::{refilter, FilterParams};
use mzlab_feat::legacy::series;
use mzlab_feat::store::Slice;

use crate::flow::{read_one, write_made, FileRunner};

fn param(job: &Job, name: &str) -> Option<f64> {
    job.resolved
        .params
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| v.f64())
}

/// The thresholds of the step over `FilterParams::default()`.
pub fn filter_params(job: &Job) -> FilterParams {
    let d = FilterParams::default();
    FilterParams {
        tol_min: param(job, "tol_min").unwrap_or(d.tol_min),
        tol_max: param(job, "tol_max").unwrap_or(d.tol_max),
        decoy: DecoyParams {
            step: param(job, "decoy_step").unwrap_or(d.decoy.step),
            k: param(job, "decoy_k").map_or(d.decoy.k, |v| v as u32),
            ppm: param(job, "decoy_ppm").unwrap_or(d.decoy.ppm),
        },
    }
}

/// `filtri/1` text of the `.mzfeat` `tp` (with the block `interp`) and the `ipotesi/1` `hypotheses`.
pub fn refilter_text(job: &Job, tp: &[u8], hypotheses: &[u8]) -> Result<String> {
    let text = String::from_utf8_lossy(hypotheses);
    let (hyps, _, _) =
        series::registry(&text).map_err(|e| Error::new("error.interp.failed").with("detail", e))?;
    let mut hyps: Vec<_> = hyps.into_iter().collect();
    hyps.sort_by_key(|h| h.0);
    let hyps: Vec<_> = hyps.into_iter().map(|h| h.1).collect();
    let reg = refilter(&mut Slice(tp), tp.len() as u64, &hyps, &filter_params(job))?;
    Ok(filtri_json(&reg))
}

/// `tp.filter`: `filtri/1` written next to the other outputs of the run.
pub(crate) fn refilter_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let (tp, hypotheses) = (read_one(r, job, "tp")?, read_one(r, job, "hypotheses")?);
    let text = refilter_text(job, &tp.1, &hypotheses.1)?;
    write_made(r, job, text, DataType::Table)
}
