//! The Flow table steps that do not fit the `table` module, in the browser (`.wasm` `flowtab`): `table.drift` (`mzlab_feat::flow_drift::drift_text`), `qc.run_order` (`mzlab_feat::runorder::runorder_text`) and
//! `report.methods` (`mzlab_feat::methods::methods_text`) and `export.annotations` (`mzlab_feat::flow_export_annot`), the functions `flow run` calls.
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

/// The `runorder/1` text of the sample table `samples`; `design` holds the batches (`disegno/1`), empty for none (the batch of the sample table). `params_json` is the step's parameters merged with its defaults.
/// Errors are JSON `{"error_key","params"}` texts.
#[wasm_bindgen]
pub fn flow_runorder(
    params_json: &str,
    samples: &[u8],
    design: &[u8],
) -> std::result::Result<String, JsValue> {
    let params = json_params(params_json).map_err(crate::js_err)?;
    let design = (!design.is_empty()).then_some(design);
    mzlab_feat::runorder::runorder_text(&params, samples, design).map_err(crate::js_err)
}

/// The Markdown methods text of `run`, a `corsa/1` text.
#[wasm_bindgen]
pub fn flow_methods(params_json: &str, run: &[u8]) -> std::result::Result<String, JsValue> {
    let params = json_params(params_json).map_err(crate::js_err)?;
    mzlab_feat::methods::methods_text(&params, run).map_err(crate::js_err)
}

/// The CSV of `export.annotations` for `table` (the bytes of a `.mzfeat`); the upstream files are packed as `manifest` (JSON `[{"port","name","len"}]`) over the concatenated bytes `blob`.
/// Errors are JSON `{"error_key","params"}` texts.
#[wasm_bindgen]
pub fn flow_export_annot(
    params_json: &str,
    table: &[u8],
    manifest: &str,
    blob: &[u8],
) -> std::result::Result<String, JsValue> {
    mzlab_feat::flow_export_annot::export_text_packed(params_json, table, manifest, blob)
        .map_err(crate::js_err)
}
