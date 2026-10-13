//! `campioni/2` for the Data view (`mzlab_core::campioni::route`).

use wasm_bindgen::prelude::*;

/// `op` is `proponi`, `valida`, `da_legacy` or `per_tp`; JSON in, JSON out, an error is JSON with a key.
#[wasm_bindgen]
pub fn campioni(op: &str, input: &str) -> Result<String, JsValue> {
    mzlab_core::campioni::route(op, input).map_err(|e| crate::error_json(&e))
}
