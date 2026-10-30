//! `annot.regulatory` in the browser: the export of the `attributio-all` .wasm that runs the function `flow run` runs.

use wasm_bindgen::prelude::*;

use crate::js_err;

/// `annot.regulatory` (`list_membership`) in the browser: the `annot/1` text with the list memberships that `flow run` writes
/// (`mzlab_feat::flow_regulatory::regulatory_text`); `package` is the raw `normative.mzsus`.
#[wasm_bindgen]
pub fn flow_regulatory(params_json: &str, annot: &[u8], package: &[u8]) -> Result<String, JsValue> {
    let p = mzlab_feat::flow_regulatory::Params::from_json(params_json).map_err(js_err)?;
    mzlab_feat::flow_regulatory::regulatory_text(&p, annot, package).map_err(js_err)
}
