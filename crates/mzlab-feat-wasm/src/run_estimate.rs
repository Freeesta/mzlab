//! `features.estimate@1` of one indexed file, for `flow run` and for the page: the values the Pipeline panel proposes (noise floor, minimum height, m/z
//! tolerance, peak width, RT tolerance, dead time, mass defect), measured by `run::preview`. The text is `estimates/1`, the preview without the histograms
//! and the example traces; each value keeps the numbers it comes from. What the plan does not write (polarity, axis, the unit profile) comes from the
//! file, the rule of `mzlab_feat::nature`. Called by `mzlab-cli` (`flow_estimate.rs`) and by the bridge (`flow_estimate` of `lib.rs`).

use mzlab_core::error::Result;
use mzlab_core::flow::value_text;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::{parse, Value};
use mzlab_feat::nature::{estimate, of_indexes, polarity_estimate};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::steps::recipe_map;

use crate::run::preview;

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

/// What the file decided: name, value and proof.
pub type Estimates = Vec<(String, Value, Value)>;

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

/// The recipe a file is measured with: the profile and the polarity of the file (`nature`), then `given`, the parameters the plan writes. A value the
/// descriptor fills in is not a choice of the plan: a negative or unit-resolution file is measured as one. With the estimates of what the file decided.
pub fn recipe_of<R: ReadAt>(
    index: &mut Index<R>,
    given: &[(String, Value)],
) -> Result<(Recipe, Estimates)> {
    let written = |name: &str| given.iter().any(|(k, _)| k == name);
    let nature = of_indexes(std::slice::from_ref(index));
    let mut estimates = Vec::new();
    let polarity = match nature.polarity.filter(|_| !written("polarity")) {
        Some(p) => {
            estimates.push(polarity_estimate(p));
            Some(p)
        }
        None => Recipe::default().detect.polarity,
    };
    let mut recipe = Recipe::for_index_estimated_in(index, polarity)?;
    if recipe.detect.axis.is_da() && !written("axis") {
        estimates.push(estimate(
            "axis",
            Value::Str("Da".into()),
            "unit resolution of the files",
        ));
    }
    recipe_map::apply(&mut recipe, "features.detect", given)?;
    Ok((recipe, estimates))
}

/// `features.estimate` of one file named `name` (the name of its source): the `estimates/1` text and what the file decided.
pub fn estimate_of<R: ReadAt>(
    index: &mut Index<R>,
    name: &str,
    given: &[(String, Value)],
) -> Result<(String, Estimates)> {
    let (recipe, estimates) = recipe_of(index, given)?;
    let p = preview(index, &recipe, false)?;
    Ok((estimate_text(&recipe, name, &p)?, estimates))
}
