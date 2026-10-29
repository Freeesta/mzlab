//! `spectra.ms2` in the browser: the MGF of the MS2 scans of one mzML (`mzlab_feat::flow_ms2::ms2_text`, the function `flow run` calls). The page calls it once per file, in the order
//! of the plan, and joins the texts, which is what the function does for several files. The feature table of the step is the CSV of `export.table_csv` (`features`).

#[cfg(feature = "flow_ms2")]
use mzlab_feat::flow_ms2::{ms2_text, Ms2Params};
use wasm_bindgen::prelude::*;

/// The MGF of `mzml` (named `name`, the stem names the spectra), under the parameters of the step as JSON merged with its defaults. Errors are JSON `{"error_key","params"}` texts.
#[cfg(feature = "flow_ms2")]
#[wasm_bindgen]
pub fn flow_ms2(
    params_json: &str,
    name: &str,
    mzml: &[u8],
    features: Option<Vec<u8>>,
) -> std::result::Result<String, JsValue> {
    let p = Ms2Params::from_json(params_json).map_err(crate::js_err)?;
    ms2_text(&p, &[(name, mzml)], features.as_deref()).map_err(crate::js_err)
}

// `formula.ms1` (the sibling of this step, same .wasm): declared here so that lib.rs does not grow.
#[cfg(feature = "flow_formula")]
#[path = "run_formula.rs"]
pub mod run_formula;
