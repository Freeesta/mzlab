//! `/api/fileinfo` for the page: the head of the mzML (read through JS) and the index of the file.

use super::{error_json, Engine};
use js_sys::Function;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl Engine {
    /// `/api/fileinfo` of the file at `slot`: the head of its mzML (`size` bytes, read through `read`) and the index (`name` = the file name).
    pub fn fileinfo(&mut self, slot: u32, name: &str, size: f64, read: Function) -> Result<String, JsValue> {
        let Some(r) = self.idx.get_mut(&slot) else {
            return Err(error_json(&mzlab_core::Error::new("error.badIndex")));
        };
        let mut m = self.nat.mzml(slot, read, size)?;
        let head = m.info_head().map_err(|e| error_json(&e))?;
        mzlab_core::fileroutes::fileinfo(r, &mut m, name, &head).map_err(|e| error_json(&e))
    }

    /// `api/canali` of the file at `slot`: the channels of its scans and of the SRM/SIC chromatograms of its mzML (`read`, `size` bytes), as JSON.
    pub fn canali(&mut self, slot: u32, name: &str, size: f64, read: Function) -> Result<String, JsValue> {
        let Some(r) = self.idx.get_mut(&slot) else {
            return Ok(String::new());
        };
        let end = r.idx.scans.iter().map(|s| s.end).max().unwrap_or(0);
        let mut m = self.nat.mzml(slot, read, size)?;
        let chroms = m.chromatograms(end).map_err(|e| error_json(&e))?;
        let facts = mzlab_core::iteminfo::facts_from(&mut m, end, chroms.len() as i64, true).map_err(|e| error_json(&e))?;
        r.pon_cromatogrammi(name, &chroms, &facts);
        Ok(r.canali().json())
    }
}

/// Bytes of the linear memory of this WASM instance (0 off wasm32): the CI guards its peak per phase (`tests/test_guardie_memoria.py`).
#[wasm_bindgen]
pub fn memory_bytes() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        (core::arch::wasm32::memory_size(0) * 65536) as f64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0.0
    }
}
