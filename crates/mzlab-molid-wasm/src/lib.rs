//! wasm-bindgen bridge of `mzlab_feat::chem::identity`: is a drawing in the molecule catalogue (`molid/1`). A `.wasm` of its own because it carries the
//! `mol/1` record (`mzlab-mol`: SMILES and molfile readers, canonical order, aromaticity), too large for the `grafo` module; the Draw page loads it on
//! the first lookup. Errors are JSON `{"error_key", "params"}` strings, as the other modules of mzFinder throw them.
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "molid/1";

#[wasm_bindgen]
pub fn version() -> String {
    format!("{BRIDGE} feat={}", mzlab_feat::FEAT_VERSION)
}

/// `molid/1` of a request `{"molfile"|"smiles", "catalogo"}`: the catalogue entries that are the drawing (exact) or share its skeleton (near, with
/// what differs); the same bytes as `mzlab-cli mol-id` and the app's `feat/grafo` `kind: "identity"`.
#[wasm_bindgen]
pub fn identity(request: &str) -> std::result::Result<String, JsValue> {
    mzlab_feat::chem::identity::identity_json(request).map_err(|e| JsValue::from_str(&e.to_json()))
}
