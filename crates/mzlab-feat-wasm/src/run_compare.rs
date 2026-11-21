//! `table.compare` in the browser (`.wasm` `flowcompare`): the `compare/1` text of `mzlab_feat::flow_compare`, the function `flow run` calls.
//! A module of its own: the reference engine and the shared name parser would take `flowstat` past its growth rule; loaded only when the step runs.

use mzlab_feat::flow_ms2::json_params;
use wasm_bindgen::prelude::*;

/// The `compare/1` text of `table.compare` on `table` (the bytes of a `.mzfeat`, named `name`) with an optional `samples/1` document (empty = none). Errors are JSON `{"error_key","params"}` texts.
#[wasm_bindgen]
pub fn flow_compare(
    params_json: &str,
    name: &str,
    table: &[u8],
    samples: &[u8],
) -> std::result::Result<String, JsValue> {
    let params = json_params(params_json).map_err(crate::js_err)?;
    let samples = (!samples.is_empty()).then_some(samples);
    mzlab_feat::flow_compare::compare_text(&params, name, table, samples).map_err(crate::js_err)
}
