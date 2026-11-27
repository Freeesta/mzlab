//! wasm-bindgen bridge of the views of `file method` (`mzlab_core::methodview::attach`): the `lc` and `dad` parts of `file_method/1`, added to the document of the
//! `.wasm` `filemethod`. A `.wasm` of its own so that `filemethod` (tight cap) does not carry the view code; fetched on the first request by
//! `mzlab/web/rust-filemethod.js`. Errors are JSON `{"error_key", "params"}` strings, as in the engine.
#![forbid(unsafe_code)]
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "methodview/1";

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

/// `head` (the document of `file_method`) with its `lc` and `dad` views, the bytes `mzlab-cli file method` prints; `method` is the `method/1` text, empty when
/// the file carries none.
#[wasm_bindgen]
pub fn attach_view(head: &str, method: &str) -> Result<String, JsValue> {
    let m = (!method.is_empty()).then_some(method);
    mzlab_core::methodview::attach(head, m).map_err(|e| JsValue::from_str(&e.to_json()))
}
