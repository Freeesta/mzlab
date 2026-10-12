//! Single scans of a centroid file read from the mzML by offset (`mzlab_core::hrscans`).

use crate::{error_json, Engine, JsReader};
use js_sys::Function;
use mzlab_core::routes::Query;
use mzlab_core::specdiff::spectrum_diff;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Engine {
    /// `/api/spectra`, `scan` and `scanavg` of a centroid file: the scans come from the mzML (`read`), by the offsets of the index. Empty = Python's.
    pub fn mzml_route(&mut self, slot: u32, path: &str, query: &str, read: Function) -> Result<String, JsValue> {
        let Some(r) = self.idx.get_mut(&slot) else { return Ok(String::new()) };
        r.mzml_route(path, &Query::parse(query), &mut JsReader { read }).map(|a| a.unwrap_or_default()).map_err(|e| error_json(&e))
    }

    /// `/api/spectrum` with a background (`bgk`): the window minus the window of the background, from the index(es). Empty = Python's.
    pub fn spectrum_bg(&mut self, slot: u32, query: &str) -> Result<String, JsValue> {
        let q = Query::parse(query);
        let Some(mut main) = self.idx.remove(&slot) else { return Ok(String::new()) };
        let out = (|| -> Result<Option<String>, mzlab_core::Error> {
            let Some(req) = main.diff_request(&q)? else { return Ok(None) };
            let k = req.bg_k() as u32;
            if k == slot {
                return spectrum_diff(&mut main, None, &req);
            }
            self.idx.get_mut(&k).map_or(Ok(None), |bg| spectrum_diff(&mut main, Some(bg), &req))
        })();
        self.idx.insert(slot, main);
        out.map(|a| a.unwrap_or_default()).map_err(|e| error_json(&e))
    }
}
