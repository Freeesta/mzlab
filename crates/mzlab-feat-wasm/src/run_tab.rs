//! The Flow table steps that do not fit the `table` module, in the browser (`.wasm` `flowtab`): `table.drift` (`mzlab_feat::flow_drift::drift_text`, the function `flow run` calls).
//! Loaded by the page only when one of these steps runs.

use mzlab_feat::flow_ms2::json_params;
use wasm_bindgen::prelude::*;

/// The `deriva/1` text of `table` (the bytes of a `.mzfeat`) under the QC files of `samples` and the batches of `design`; `params_json` is the step's parameters merged with its defaults.
/// Errors are JSON `{"error_key","params"}` texts.
#[wasm_bindgen]
pub fn flow_drift(
    params_json: &str,
    table: &[u8],
    samples: &[u8],
    design: &[u8],
) -> std::result::Result<String, JsValue> {
    let params = json_params(params_json).map_err(crate::js_err)?;
    mzlab_feat::flow_drift::drift_text(&params, table, samples, design).map_err(crate::js_err)
}
