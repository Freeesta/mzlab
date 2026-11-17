//! `rt.predict` in the browser: the export of the `attributio-all` .wasm that runs the function `flow run` runs.

use wasm_bindgen::prelude::*;

use crate::js_err;

/// `rt.predict` in the browser: the `candidates/1` text `flow run` writes (`mzlab_feat::flow_rtpredict::rtpredict_text`); `algorithm` is `logp_window` or `forest`
/// (empty = `logp_window`), `standards` the CSV `smiles,rt` and `queries` the CSV `query,rt` (empty: the RT of each hit).
#[wasm_bindgen]
pub fn flow_rtpredict(
    params_json: &str,
    algorithm: &str,
    candidates: &[u8],
    standards: &[u8],
    queries: &[u8],
) -> Result<String, JsValue> {
    let params = mzlab_feat::flow_ms2::json_params(params_json).map_err(js_err)?;
    let algorithm = if algorithm.is_empty() {
        "logp_window"
    } else {
        algorithm
    };
    mzlab_feat::flow_rtpredict::rtpredict_text(&params, algorithm, candidates, standards, queries)
        .map_err(js_err)
}
