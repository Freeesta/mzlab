//! `ricerca/1` export: the numeric match of the search box (`mzlab_core::ricerca`); an error is JSON with a key.

use crate::error_json;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn ricerca_numero(req: &str) -> Result<String, JsValue> {
    mzlab_core::ricerca::match_number(req).map_err(|e| error_json(&e))
}
