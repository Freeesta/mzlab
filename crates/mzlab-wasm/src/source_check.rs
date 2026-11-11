// Whether the index open in a slot was built from a file the page holds (`Index::built_from`: size and probe, the first and last 64 KB):
// the warmed files of the pointed folder (rust-scalda.js) are claimed only when it was, so a file replaced with the same name, size and date is indexed again.
use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::{error_json, Engine, JsReader};

#[wasm_bindgen]
impl Engine {
    /// `read(offset, length)` gives the bytes of the file of `size` bytes; false when no index is open in `slot`.
    pub fn index_built_from(
        &mut self,
        slot: u32,
        size: f64,
        read: Function,
    ) -> Result<bool, JsValue> {
        let Some(r) = self.idx.get(&slot) else {
            return Ok(false);
        };
        r.idx
            .built_from(&mut JsReader { read }, size as u64)
            .map_err(|e| error_json(&e))
    }
}
