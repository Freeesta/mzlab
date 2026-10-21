//! The native executor of `table.drift@1`: the correction of the drift of the areas along the order of injection (`drift::correct`, the QC files of each
//! batch of the design). The `.mzfeat` is never changed: the output is `deriva/1` (the summary of the engine) plus the factor of every cell. Called by `flow.rs`.

use crate::flow::{read_one, write_made, FileRunner};
use crate::flow_stats::{columns, samples_of};
use mzlab_core::disegno;
use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::json::Out;
use mzlab_core::step::DataType;
use mzlab_feat::drift::{batches_of, correct, Method};
use mzlab_feat::store::{origin_from_meta, read_table};

/// The curve of the job: a line, or a local line with the span of the step.
fn method_of(job: &Job) -> Method {
    let param = |name: &str| job.resolved.params.iter().find(|(k, _)| k == name);
    let span = param("span").and_then(|(_, v)| v.f64()).unwrap_or(0.75);
    match param("method").and_then(|(_, v)| v.str()) {
        Some("loess") => Method::Loess(span),
        _ => Method::Linear,
    }
}

/// The `deriva/1` text of `table` under the QC files of `samples` and the batches of `design`, with `"factor"`: one list of factors per row.
pub fn drift_text(job: &Job, table: &[u8], samples: &[u8], design: &[u8]) -> Result<String> {
    let (t, meta) = read_table(table)?;
    let inputs = origin_from_meta(&meta).inputs;
    if inputs.len() != t.n_files {
        return Err(Error::new("error.badView"));
    }
    let cols = columns(&inputs, &samples_of(samples)?);
    let batch = batches_of(&cols, &disegno::from_json(design)?);
    let d = correct(&t, &cols, &batch, method_of(job))?;
    let mut text = d.summary_json(&cols);
    text.pop();
    let mut o = Out::default();
    o.raw(",\"factor\":[");
    for r in 0..t.len() {
        o.raw(if r > 0 { "," } else { "" })
            .floats(&d.factor[r * t.n_files..(r + 1) * t.n_files]);
    }
    o.raw("]}");
    Ok(text + &o.0)
}

pub(crate) fn drift_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let table = read_one(r, job, "table")?;
    let (samples, design) = (read_one(r, job, "samples")?, read_one(r, job, "design")?);
    let text = drift_text(job, &table.1, &samples.1, &design.1)?;
    write_made(r, job, text, DataType::Table)
}
