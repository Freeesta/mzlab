//! The native executor of `features.estimate@1`: for each indexed file the values the Pipeline panel proposes (noise floor, minimum height, m/z
//! tolerance, peak width, RT tolerance, dead time, mass defect), measured by `run::preview`, the engine the page calls. The file written is
//! `estimates/1`, the preview without the histograms and the example traces; each value keeps the numbers it comes from. What the plan does not write
//! (polarity, axis, the unit profile) comes from the file, the rule of `mzlab_feat::nature`. Called by `flow.rs`.

use mzlab_core::error::Result;
use mzlab_core::flow::value_text;
use mzlab_core::flow::Job;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{parse, Value};
use mzlab_core::step::DataType;
use mzlab_feat::nature::{estimate, of_indexes, polarity_estimate};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::steps::recipe_map;
use mzlab_feat_wasm::run;

use crate::flow::{io, FileRunner, Made};
use crate::flow_base::{open, out_path};
use crate::flow_nature::written;

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
pub fn estimate_text(recipe: &Recipe, name: &str, preview: &str) -> Result<String> {
    let preview = parse(preview.as_bytes())?;
    let axis = if recipe.detect.axis.is_da() {
        "Da"
    } else {
        "ppm"
    };
    let mut doc = vec![
        ("schema".to_string(), Value::Str("estimates/1".into())),
        ("file".to_string(), Value::Str(name.to_string())),
        (
            "polarity".to_string(),
            Value::Num(recipe.detect.polarity.map_or(0.0, f64::from)),
        ),
        ("axis".to_string(), Value::Str(axis.into())),
    ];
    for k in KEPT {
        doc.extend(preview.get(k).map(|v| (k.to_string(), v.clone())));
    }
    Ok(value_text(&Value::Obj(doc)))
}

/// What the file decided: name, value and proof.
type Estimates = Vec<(String, Value, Value)>;

/// The recipe a file is measured with: the profile and the polarity of the file (`nature`), then the parameters the plan writes. A value the descriptor
/// fills in is not a choice of the plan: a negative or unit-resolution file is measured as one. With the estimates of what the file decided.
fn recipe_of<R: ReadAt>(job: &Job, index: &mut Index<R>) -> Result<(Recipe, Estimates)> {
    let nature = of_indexes(std::slice::from_ref(index));
    let mut estimates = Vec::new();
    let polarity = match nature.polarity.filter(|_| !written(job, "polarity")) {
        Some(p) => {
            estimates.push(polarity_estimate(p));
            Some(p)
        }
        None => Recipe::default().detect.polarity,
    };
    let mut recipe = Recipe::for_index_estimated_in(index, polarity)?;
    if recipe.detect.axis.is_da() && !written(job, "axis") {
        estimates.push(estimate(
            "axis",
            Value::Str("Da".into()),
            "unit resolution of the files",
        ));
    }
    let given: Vec<(String, Value)> = job
        .resolved
        .params
        .iter()
        .filter(|(k, _)| written(job, k))
        .cloned()
        .collect();
    recipe_map::apply(&mut recipe, "features.detect", &given)?;
    Ok((recipe, estimates))
}

/// `features.estimate`: one `estimates/1` file per indexed file, measured on the file's own data (its own profile and polarity, then what the plan writes).
pub(crate) fn estimate_job(r: &FileRunner, job: &Job) -> Result<Made> {
    let input = &job.inputs[0].1[0];
    let mut index = open(&r.path(input))?;
    let (recipe, estimates) = recipe_of(job, &mut index)?;
    let preview = run::preview(&mut index, &recipe, false)?;
    let text = estimate_text(&recipe, &r.source_name(input), &preview)?;
    let out = out_path(r, job, "json");
    std::fs::write(&out, text).map_err(io)?;
    Ok(Made {
        estimates,
        ..Made::new("estimates", r.keep(out, DataType::Qc)?)
    })
}
