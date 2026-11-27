//! wasm-bindgen bridge of `feature-slots/1` (`mzlab_feat::slots`): per slot of the feature view its state, evidence, source and the step that fills it. A `.wasm`
//! of its own (inside the table module it took +1.4 % over its cap); fetched by `TP_Mine/js/36_feature_qc.js` on the first row whose Slots tab is opened.
//! The `.mzfeat` is read by offsets (`read(0, offset, length) -> Uint8Array`), the `samples/1` comes as JSON text. Errors are JSON `{"error_key", "params"}` strings.
#![forbid(unsafe_code)]
use js_sys::{Function, Uint8Array};
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::ReadAt;
use mzlab_ops::run::table::core;
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "feature-slots/1";

#[wasm_bindgen]
pub fn version() -> String {
    BRIDGE.to_string()
}

/// The bytes of the `.mzfeat` held by JavaScript: `read(0, offset, length) -> Uint8Array`.
struct JsReader(Function);

impl ReadAt for JsReader {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        let io = || Error::new("error.io");
        let v = self
            .0
            .call3(
                &JsValue::NULL,
                &0u32.into(),
                &(off as f64).into(),
                &(buf.len() as f64).into(),
            )
            .map_err(|_| io())?;
        let a: Uint8Array = v.dyn_into().map_err(|_| io())?;
        if a.length() as usize != buf.len() {
            return Err(Error::new("error.badIndex"));
        }
        a.copy_to(buf);
        Ok(())
    }
}

/// The `feature-slots/1` of the row with `id`; `output` names the table (empty: none).
#[wasm_bindgen]
pub fn table_slots(
    table_read: Function,
    size: f64,
    samples_json: &str,
    id: u32,
    output: &str,
) -> std::result::Result<String, JsValue> {
    core::slots_json(&mut JsReader(table_read), size as u64, samples_json, id, output)
        .map_err(|e| JsValue::from_str(&e.to_json()))
}
