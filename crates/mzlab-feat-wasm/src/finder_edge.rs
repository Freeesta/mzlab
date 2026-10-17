//! The stateless functions of mzFinder on the identity of the ions (`identity_graph`, `polarity_pairs`, `origin_evidence`), a `.wasm` of its own (feature
//! `finder_edge`): the worker loads it only when the page asks for one of them. Errors are JavaScript `Error`s with the English text.

use mzlab_feat::legacy;
use wasm_bindgen::prelude::*;

fn user(e: String) -> JsValue {
    js_sys::Error::new(&e).into()
}

/// `api.identity_graph(payload)`.
#[wasm_bindgen]
pub fn finder_identity_graph(payload: &str) -> Result<String, JsValue> {
    legacy::identity_graph(payload).map_err(user)
}

/// `api.polarity_pairs(payload)`.
#[wasm_bindgen]
pub fn finder_polarity_pairs(payload: &str) -> Result<String, JsValue> {
    legacy::polarity_pairs(payload).map_err(user)
}

/// `api.origin_evidence(payload)`: the evidence of the report «where does this ion come from?».
#[wasm_bindgen]
pub fn finder_origin_evidence(payload: &str) -> Result<String, JsValue> {
    legacy::edge::origin_evidence(payload).map_err(user)
}
