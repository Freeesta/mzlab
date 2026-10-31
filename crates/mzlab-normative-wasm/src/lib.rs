//! wasm-bindgen bridge of `mzlab_feat::normative::official`: the user's own CSV of the ECHA Candidate List (`svhc`) or of OEHHA's Proposition 65 list (`ca_prop65`) and the
//! CAS sidecar of the site become a private `.mzsus` with the same bytes as `mzlab-cli normative import-official`. A `.wasm` of its own, loaded only by the Library window.
//! Errors are JSON `{"error_key", "params"}` strings.
use mzlab_core::json::Out;
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "mzfeat-normative/1";

#[wasm_bindgen]
pub fn version() -> String {
    format!("{BRIDGE} feat={}", mzlab_feat::FEAT_VERSION)
}

/// The answer of `mzlab_feat::normative::official::import_official`: `u32` little-endian length of the summary JSON, the summary, then the `.mzsus`.
/// `label` is the JSON `{"file", "date"}`.
#[wasm_bindgen]
pub fn import_official(
    reader: &str,
    csv: &[u8],
    sidecar: &str,
    label: &str,
) -> Result<Vec<u8>, JsValue> {
    mzlab_feat::normative::official::import_official(reader, csv, sidecar, label).map_err(|e| {
        let mut o = Out::default();
        o.raw("{\"error_key\":\"err.normative.parse\",\"params\":{\"message\":")
            .str(&format!("{}: {}", e.list, e.msg))
            .raw("}}");
        JsValue::from_str(&o.0)
    })
}

/// The lists of an imported `.mzsus` as a JSON array (see `mzlab_feat::normative::official::lists_of`); an empty string when it is not a readable file.
#[wasm_bindgen]
pub fn lists_of(raw: &[u8]) -> String {
    mzlab_feat::normative::official::lists_of(raw).unwrap_or_default()
}
