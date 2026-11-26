//! wasm-bindgen bridge of the ion-scale method values of `raw xic --ions` (`mzlab_core::ionfacts`): the values the `method/1` text of a file gives, which the
//! page appends to `api/rawxic`. A `.wasm` of its own (inside `filemethod` it grew that module +7.8 %, over the 5 % rule); fetched on the first request
//! (`mzlab/web/rust-filemethod.js`). Errors are JSON `{"error_key", "params"}` strings, as in the engine.
#![forbid(unsafe_code)]
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "ionfacts/1";

#[wasm_bindgen]
pub fn version() -> String {
    BRIDGE.to_string()
}

/// The message of a panic goes to a global of the worker (`__mzlabPanic`) before the abort, as in the engine.
#[wasm_bindgen(start)]
pub fn start() {
    std::panic::set_hook(Box::new(|info| {
        let _ = js_sys::Reflect::set(
            &js_sys::global(),
            &"__mzlabPanic".into(),
            &info.to_string().into(),
        );
    }));
}

/// The ion-scale method values of `method` (the `method/1` text, empty when the file carries none) for `scan_type` (`""`: every kind): the
/// `{"query","read"}` document of `mzlab_core::ionfacts::read_json`, whose `query` the page appends to `api/rawxic` (`ions=1`).
#[wasm_bindgen]
pub fn ion_facts(method: &str, scan_type: &str) -> Result<String, JsValue> {
    mzlab_core::ionfacts::read_json(method, scan_type).map_err(|e| JsValue::from_str(&e.to_json()))
}
