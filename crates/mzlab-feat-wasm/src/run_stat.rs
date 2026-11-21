//! The statistics steps of the Flow in the browser (`.wasm` `flowstat`): `table.stats`, `table.pca` and `table.kmd`, the texts of `mzlab_feat::flow_stat`, the functions `flow run` calls.
//! A module of its own (the PCA engine and Welch/ANOVA do not fit `flowtab`), loaded by the page only when one of these steps runs.

use mzlab_feat::flow_ms2::json_params;
use wasm_bindgen::prelude::*;

/// The `stat/1` text of `table.stats` on `table` (the bytes of a `.mzfeat`) under `samples` and the comparisons of `design`; `params_json` is the step's parameters merged with its defaults.
/// Errors are JSON `{"error_key","params"}` texts.
#[wasm_bindgen]
pub fn flow_stats(
    params_json: &str,
    table: &[u8],
    samples: &[u8],
    design: &[u8],
) -> std::result::Result<String, JsValue> {
    let params = json_params(params_json).map_err(crate::js_err)?;
    mzlab_feat::flow_stat::stats_text(&params, table, samples, design).map_err(crate::js_err)
}

/// The answer of the PCA engine for `table.pca` on `table` under the roles of `samples`. Errors are JSON `{"error_key","params"}` texts.
#[wasm_bindgen]
pub fn flow_pca(
    params_json: &str,
    table: &[u8],
    samples: &[u8],
) -> std::result::Result<String, JsValue> {
    let params = json_params(params_json).map_err(crate::js_err)?;
    mzlab_feat::flow_stat::pca_text(&params, table, samples).map_err(crate::js_err)
}

/// The Kendrick text of `table.kmd` on the rows of `table`. Errors are JSON `{"error_key","params"}` texts.
#[wasm_bindgen]
pub fn flow_kmd(params_json: &str, table: &[u8]) -> std::result::Result<String, JsValue> {
    let params = json_params(params_json).map_err(crate::js_err)?;
    mzlab_feat::flow_stat::kmd_text(&params, table).map_err(crate::js_err)
}
