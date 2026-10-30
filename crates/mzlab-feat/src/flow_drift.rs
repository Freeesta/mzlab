//! The text of `table.drift@1`: the correction of the drift of the areas along the order of injection (`drift::correct`, the QC files of each batch of the design).
//! Bytes in, text out: `flow run` (`mzlab-cli`) and the browser module `flowtab` (`mzlab-feat-wasm`) both call it, so they write the same `deriva/1` bytes.

use crate::drift::{batches_of, correct, Method};
use crate::samples::{columns, samples_of};
use crate::store::{origin_from_meta, read_table};
use mzlab_core::disegno;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{Out, Value};

/// The curve of the step: a line, or a local line with the span of the step.
fn method_of(params: &[(String, Value)]) -> Method {
    let param = |name: &str| params.iter().find(|(k, _)| k == name);
    let span = param("span").and_then(|(_, v)| v.f64()).unwrap_or(0.75);
    match param("method").and_then(|(_, v)| v.str()) {
        Some("loess") => Method::Loess(span),
        _ => Method::Linear,
    }
}

/// The `deriva/1` text of `table` under the QC files of `samples` and the batches of `design`, with `"factor"`: one list of factors per row. `params` are the parameters of the step.
pub fn drift_text(
    params: &[(String, Value)],
    table: &[u8],
    samples: &[u8],
    design: &[u8],
) -> Result<String> {
    let (t, meta) = read_table(table)?;
    let inputs = origin_from_meta(&meta).inputs;
    if inputs.len() != t.n_files {
        return Err(Error::new("error.badView"));
    }
    let cols = columns(&inputs, &samples_of(samples)?);
    let batch = batches_of(&cols, &disegno::from_json(design)?);
    let d = correct(&t, &cols, &batch, method_of(params))?;
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
