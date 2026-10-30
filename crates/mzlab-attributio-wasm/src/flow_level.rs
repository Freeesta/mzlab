//! `annot.level` in the browser: the export of the `attributio-all` .wasm that runs the rule `flow run` runs.

use wasm_bindgen::prelude::*;

use crate::js_err;

/// `annot.level` in the browser: the text `flow run` writes (`mzlab_feat::flow_level::level_text_json`); an empty slice is a port not wired (the `candidates` port takes the `candidates/1` and the `screen/1`).
#[wasm_bindgen]
pub fn flow_level(
    params_json: &str,
    hits: &[u8],
    screen: &[u8],
    formulas: &[u8],
    standards: &[u8],
    spectra: &[u8],
) -> Result<String, JsValue> {
    mzlab_feat::flow_level::level_text_json(
        params_json,
        (hits, screen),
        (formulas, standards, spectra),
    )
    .map_err(js_err)
}
