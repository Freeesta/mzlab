//! wasm-bindgen bridge of the molecule kit of Draw on the mol/1 record (`mzlab-mol` linked once): `check` gives `check/1`, what is wrong in the
//! drawing, and `cip` gives `cip/1`, its R/S and E/Z. A `.wasm` of its own (`molkit`), so the grafo one does not carry the record; the CLI
//! (`mol-check`, `mol-cip`) and the app (`feat/grafo`, kinds `check` and `cip`) run the same functions. Errors are JSON `{"error_key", "params"}` strings.
use mzlab_core::error::Error;
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "molkit/1";

fn js_err(e: Error) -> JsValue {
    JsValue::from_str(&e.to_json())
}

#[wasm_bindgen]
pub fn version() -> String {
    BRIDGE.to_string()
}

/// `check/1` of a request `{"molfile"}`, the same bytes as `mzlab-cli mol-check`.
#[wasm_bindgen]
pub fn check(request: &str) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::check::check_json(request).map_err(js_err)
}

/// `cip/1` of a request `{"molfile"}`, the same bytes as `mzlab-cli mol-cip`.
#[wasm_bindgen]
pub fn cip(request: &str) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::cip::cip_json(request).map_err(js_err)
}
