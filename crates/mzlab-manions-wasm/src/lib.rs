//! wasm-bindgen bridge of the ion scale of the integrations table (`mzlab_core::manint::table_text_ions`, `mzlab-cli peak table --ions`): the same
//! table as `manual_table` of the `rotte` module with the columns `IONS_HEAD` after the others. A `.wasm` of its own so `rotte`, which is nearly full,
//! never links the ion code; fetched on the first request that asks for the ion scale (`mzlab/web/rust-manions.js`). Errors are "E" + JSON, as in `rotte`.
#![forbid(unsafe_code)]
use wasm_bindgen::prelude::*;

/// Stable version of the bridge: the page checks that the loaded `.wasm` is the one it was built with.
const BRIDGE: &str = "manions/1";

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

/// `manual_table` with the ion scale: `ions` = `{"samples":[{"scan_type"?, "scan_from"?, method values?, "traces"?}]}` (`manint_ions`). "E" + JSON = the
/// user's error.
#[wasm_bindgen]
pub fn manual_table_ions(files: &str, windows: &str, sums: bool, ions: &str) -> String {
    match mzlab_core::manint::table_text_ions(files, windows, sums, ions) {
        Ok(csv) => csv,
        Err(e) => format!("E{}", e.to_json()),
    }
}

/// What the call told the engine per sample (`manint::choices_text`): `[{"sample","scan_type","scan_type_from","facts","trace_factors","needs"}]`.
#[wasm_bindgen]
pub fn manual_ion_choices(files: &str, ions: &str) -> String {
    match mzlab_core::manint::choices_text(files, ions) {
        Ok(json) => json,
        Err(e) => format!("E{}", e.to_json()),
    }
}
