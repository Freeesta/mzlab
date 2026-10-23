//! `api/xicbin` of the engine: the XIC of the index as doubles into a buffer of the page (`[rt(n), y(n)]`).

use super::{error_json, Engine};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Engine {
    /// Length of the buffer `index_xic_into` needs (twice the scans of the level; 0 if the slot has no index).
    pub fn index_xic_len(&self, slot: u32, level: u8) -> u32 {
        self.idx
            .get(&slot)
            .map_or(0, |r| 2 * r.xic_len(level) as u32)
    }

    /// The XIC of one level from the index into `out` as `[rt(n), y(n)]`, the numbers of `index_xic` as doubles. Returns n.
    pub fn index_xic_into(
        &mut self,
        slot: u32,
        level: u8,
        mz: f64,
        tol: f64,
        out: &mut [f64],
    ) -> Result<u32, JsValue> {
        let Some(r) = self.idx.get_mut(&slot) else {
            return Ok(0);
        };
        r.xic_into(mz, tol, level, out)
            .map(|n| n as u32)
            .map_err(|e| error_json(&e))
    }
}
