//! `table_slots`: the `feature-slots/1` of one row (what fills each slot of the feature view), in a module of its own because `lib.rs` is at its line limit.

use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::{core, js_err, table_reader};

/// The `feature-slots/1` of the row with `id`: per slot of the feature view its state, evidence and source, or the step that fills it. `output` names the table (empty: none).
#[wasm_bindgen]
pub fn table_slots(
    table_read: Function,
    size: f64,
    samples_json: &str,
    id: u32,
    output: &str,
) -> std::result::Result<String, JsValue> {
    core::slots_json(
        &mut table_reader(table_read),
        size as u64,
        samples_json,
        id,
        output,
    )
    .map_err(js_err)
}
