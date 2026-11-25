//! wasm-bindgen bridge of `file method` (`mzlab_core::filemethod`): the `file_method/1` document of a sample from the `method/1` text and the `items/1`
//! text the page already holds. A `.wasm` of its own so that no other module grows; fetched on the first request (`load("filemethod")` of
//! `mzlab/web/rust-filemethod.js`). Errors are JSON `{"error_key", "params"}` strings, as in the engine.
#![forbid(unsafe_code)]
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "filemethod/1";

#[wasm_bindgen]
pub fn version() -> String {
    BRIDGE.to_string()
}

/// The message of a panic goes to a global of the worker (`__mzlabPanic`) before the abort, as in the engine.
#[wasm_bindgen(start)]
pub fn start() {
    std::panic::set_hook(Box::new(|info| {
        let _ = js_sys::Reflect::set(&js_sys::global(), &"__mzlabPanic".into(), &info.to_string().into());
    }));
}

/// `file_method/1` of sample `sample` (negative: an mzML, no sample id); `method` is the `method/1` text, empty when the file carries none.
#[wasm_bindgen]
pub fn file_method(sample: i32, method: &str, items: &str) -> Result<String, JsValue> {
    let id = u32::try_from(sample).ok();
    let m = (!method.is_empty()).then_some(method);
    mzlab_core::filemethod::document(id, m, items).map_err(|e| JsValue::from_str(&e.to_json()))
}
