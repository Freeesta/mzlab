//! `mz-match/1` export: the numeric match of the search box (`mzlab_core::search`); an error is JSON with a key.

use crate::error_json;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn search_number(req: &str) -> Result<String, JsValue> {
    mzlab_core::search::match_number(req).map_err(|e| error_json(&e))
}

/// `reflist/1`: a CSV of reference ions (`mzlab_core::reflist`) as items and line-numbered errors.
#[wasm_bindgen]
pub fn reflist_parse(req: &str) -> Result<String, JsValue> {
    mzlab_core::reflist::route(req).map_err(|e| error_json(&e))
}

/// `reflist-match/1`: the entries of a set of reference ions inside a window of an m/z (`mzlab_core::reflist::lookup`).
#[wasm_bindgen]
pub fn reflist_match(req: &str) -> Result<String, JsValue> {
    mzlab_core::reflist::lookup::route(req).map_err(|e| error_json(&e))
}
