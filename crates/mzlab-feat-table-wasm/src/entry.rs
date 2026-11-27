//! The export `table_entry` of the table bridge: one entry of the list of feature tables (`tables/1`), the same bytes as `mzlab-cli flow tables`.

use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::{js_err, table_reader};

/// The `tables/1` entry of a table (`mzlab_feat::tables::entry`, what `mzlab-cli flow tables` lists): its facts from the meta, then the step that made it
/// when `made_json` (`{instance, step, status?, ms?, warnings?}`) is not empty.
#[wasm_bindgen]
pub fn table_entry(
    table_read: Function,
    size: f64,
    name: &str,
    made_json: &str,
) -> std::result::Result<String, JsValue> {
    let made = if made_json.is_empty() {
        None
    } else {
        let v = mzlab_core::json::parse(made_json.as_bytes()).map_err(js_err)?;
        mzlab_feat::tables::Made::from_json(&v)
    };
    mzlab_feat::tables::entry(
        name,
        &mut table_reader(table_read),
        size as u64,
        made.as_ref(),
    )
    .map_err(js_err)
}
