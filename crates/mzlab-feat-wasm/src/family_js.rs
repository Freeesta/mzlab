//! JS exports of `family`; the plain functions are `mzlab_ops::run::feat::family`.

use crate::family::record;
use crate::{js_err, open_one, run};
use js_sys::Function;
use mzlab_core::budget::MemBudget;
use wasm_bindgen::prelude::*;

/// The `famiglia/1` record of one file (`.mzidx` of `size` bytes read with `read`). `recipe` as in `detect_file`; the budget is the default of the WASM
/// (192 MB). JSON text, or the error `error.badRecipe` / `error.memoryBudget`.
#[wasm_bindgen]
pub fn family_record(
    read: Function,
    size: f64,
    name: &str,
    recipe: &str,
) -> std::result::Result<String, JsValue> {
    let mut ix = open_one(&read, 0, size).map_err(js_err)?;
    let r = run::recipe_for(recipe, &mut ix).map_err(js_err)?;
    let budget = MemBudget::from_request(f64::NAN);
    record(&mut ix, name, &r, &budget).map_err(js_err)
}
