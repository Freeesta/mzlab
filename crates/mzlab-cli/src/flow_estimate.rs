//! The native executor of `features.estimate@1`: for each indexed file the values the Pipeline panel proposes (noise floor, minimum height, m/z
//! tolerance, peak width, RT tolerance, dead time, mass defect), measured by `run::preview`, the engine the page calls. The file written is
//! `estimates/1`, the preview without the histograms and the example traces; each value keeps the numbers it comes from. Called by `flow.rs`.

use mzlab_core::error::Result;
use mzlab_core::flow::value_text;
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::json::{parse, Value};
use mzlab_core::step::DataType;
use mzlab_feat::steps::recipe_map;
use mzlab_feat_wasm::run;

use crate::flow::{io, FileRunner};
use crate::flow_base::{open, out_path};

/// The members of the preview that are measurements; the histograms and the example traces are drawings for the panel.
const KEPT: &[&str] = &[
    "n",
    "noise",
    "min_height",
    "ppm",
    "fwhm",
    "rt_tol",
    "dead",
    "da",
    "defect",
];

/// The `estimates/1` text of one file: its name, the polarity and axis it was measured with, and the members of `KEPT` of the preview.
pub fn estimate_text(job: &Job, name: &str, preview: &str) -> Result<String> {
    let preview = parse(preview.as_bytes())?;
    let given = |k: &str| job.resolved.params.iter().find(|(n, _)| n == k);
    let mut doc = vec![
        ("schema".to_string(), Value::Str("estimates/1".into())),
        ("file".to_string(), Value::Str(name.to_string())),
    ];
    for k in ["polarity", "axis"] {
        doc.extend(given(k).cloned());
    }
    for k in KEPT {
        doc.extend(preview.get(k).map(|v| (k.to_string(), v.clone())));
    }
    Ok(value_text(&Value::Obj(doc)))
}

/// `features.estimate`: one `estimates/1` file per indexed file, measured on the file's own data (its own recipe, then the polarity and the axis of the plan).
pub(crate) fn estimate_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let input = &job.inputs[0].1[0];
    let mut index = open(&r.path(input))?;
    let mut recipe = run::recipe_for_estimated("", &mut index)?;
    let written: Vec<(String, Value)> = job.resolved.params.clone();
    recipe_map::apply(&mut recipe, "features.detect", &written)?;
    let preview = run::preview(&mut index, &recipe, false)?;
    let text = estimate_text(job, &r.source_name(input), &preview)?;
    let out = out_path(r, job, "json");
    std::fs::write(&out, text).map_err(io)?;
    r.keep(out, DataType::Qc)
}
