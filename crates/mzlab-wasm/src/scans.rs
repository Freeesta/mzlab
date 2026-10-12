//! Single scans of a centroid file read from the mzML by offset (`mzlab_core::hrscans`).

use crate::{error_json, Engine, JsReader};
use js_sys::Function;
use mzlab_core::routes::Query;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Engine {
    /// `/api/spectra`, `scan` and `scanavg` of a centroid file: the scans come from the mzML (`read`), by the offsets of the index. Empty = Python's.
    pub fn mzml_route(&mut self, slot: u32, path: &str, query: &str, read: Function) -> Result<String, JsValue> {
        let Some(r) = self.idx.get_mut(&slot) else { return Ok(String::new()) };
        r.mzml_route(path, &Query::parse(query), &mut JsReader { read }).map(|a| a.unwrap_or_default()).map_err(|e| error_json(&e))
    }
}
