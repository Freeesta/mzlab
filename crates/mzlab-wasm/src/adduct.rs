//! `api/adduct_table` export: the adduct catalogue of the m/z calculator (`mzlab_core::adduct_table`); an error is JSON with a key.

use crate::error_json;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn adduct_table(formula: &str, obs: &str, tol: &str, delta: &str) -> Result<String, JsValue> {
    mzlab_core::adduct_table::table(formula, obs, tol, delta).map_err(|e| error_json(&e))
}

/// `api/ion_pattern`: the unit-resolution isotope pattern of a formula with an adduct of the catalogue (`mzlab_core::ionpattern`).
#[wasm_bindgen]
pub fn ion_pattern(formula: &str, adduct: &str) -> Result<String, JsValue> {
    mzlab_core::ionpattern::route(formula, adduct).map_err(|e| error_json(&e))
}
