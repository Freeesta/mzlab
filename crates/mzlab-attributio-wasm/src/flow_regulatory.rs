//! `annot.regulatory` in the browser: the export of the `attributio-all` .wasm that runs the function `flow run` runs.

use wasm_bindgen::prelude::*;

use crate::js_err;

/// `annot.regulatory` (`list_membership`) in the browser: the `annot/1` text with the list memberships that `flow run` writes
/// (`mzlab_feat::flow_regulatory::regulatory_text`); `package` is the raw `normative.mzsus`, `imported_names_json` and `imported` the imported list files
/// (a JSON list of names and the bytes joined as `u64 length (little-endian) + bytes` per file; empty: none).
#[wasm_bindgen]
pub fn flow_regulatory(
    params_json: &str,
    annot: &[u8],
    package: &[u8],
    imported_names_json: &str,
    imported: &[u8],
) -> Result<String, JsValue> {
    let p = mzlab_feat::flow_regulatory::Params::from_json(params_json).map_err(js_err)?;
    let files =
        mzlab_feat::flow_screen_suspects::unpack(imported_names_json, imported).map_err(js_err)?;
    let named: Vec<(&str, &[u8])> = files.iter().map(|(n, b)| (n.as_str(), *b)).collect();
    mzlab_feat::flow_regulatory::regulatory_text(&p, annot, package, &named).map_err(js_err)
}
