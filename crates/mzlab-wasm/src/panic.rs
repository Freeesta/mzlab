//! The message of a panic goes to a global of the worker (`__mzlabPanic`) before the abort, so the supervisor can keep it as the last panic.

/// Runs when the module is instantiated.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    install();
}

#[cfg(target_arch = "wasm32")]
fn install() {
    std::panic::set_hook(Box::new(|info| {
        let _ = js_sys::Reflect::set(
            &js_sys::global(),
            &"__mzlabPanic".into(),
            &info.to_string().into(),
        );
    }));
}
