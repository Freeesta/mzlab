//! `molecules/1` for the app (`mzlab_core::molecules::route`).

use wasm_bindgen::prelude::*;

/// `op` is `carica`, `cerca`, `get`, `elenca`, `da_csv`, `valida`, `a_csv`, `a_json` or `digest`; JSON in, JSON out (CSV text for `a_csv`), an error is JSON with a key.
#[wasm_bindgen]
pub fn molecules(op: &str, input: &str) -> Result<String, JsValue> {
    mzlab_core::molecules::route(op, input).map_err(|e| crate::error_json(&e))
}
