//! wasm-bindgen bridge of the substructure search (`mzlab_feat::chem::substructure`, matcher `mzlab_mol::SubQuery`): a drawn moiety and a list
//! of structures give the `sub/1` of where it occurs. A `.wasm` of its own, so `grafo` does not carry the readers of `mzlab-mol`: the Draw page loads
//! it only when the user asks for the search. Errors are JSON `{"error_key", "params"}` strings.
use mzlab_core::error::Error;
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "sub/1";

fn js_err(e: Error) -> JsValue {
    JsValue::from_str(&e.to_json())
}

#[wasm_bindgen]
pub fn version() -> String {
    BRIDGE.to_string()
}

/// `sub/1` of a request `{"query", "targets", "stereo", "max"}` (SMILES or molfiles as text): every target that contains the query with its
/// matches (see `search_json`; the same bytes as `mzlab-cli mol-search --sub` and the desktop route `feat/grafo`, `kind: "sub"`).
#[wasm_bindgen]
pub fn substructure(request: &str) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::substructure::search_json(request).map_err(js_err)
}
