//! JS exports of `cache`; the plain functions are `mzlab_ops::run::feat::cache`.

use crate::cache::check_stored;
#[cfg(feature = "detect")]
use crate::cache::file_key;
use crate::{js_err, open_one};
use js_sys::Function;
use wasm_bindgen::prelude::*;

#[cfg(feature = "detect")]
/// `"<sha256>.<fingerprint>"` of a `.mzidx` (`size` bytes read with `read`) for `recipe`, or `""` without a SHA-256 (no cache). Reads only the head of the index.
#[wasm_bindgen(js_name = file_key)]
pub fn file_key_of(
    read: Function,
    size: f64,
    _name: &str,
    recipe: &str,
) -> std::result::Result<String, JsValue> {
    let mut ix = open_one(&read, 0, size).map_err(js_err)?;
    file_key(recipe, &mut ix).map_err(js_err)
}

#[cfg(feature = "untargeted")]
/// `file_key_of` of the untargeted engine (`params` is its recipe).
#[wasm_bindgen]
pub fn file_key_untargeted(
    read: Function,
    size: f64,
    _name: &str,
    params: &str,
) -> std::result::Result<String, JsValue> {
    let ix = open_one(&read, 0, size).map_err(js_err)?;
    crate::untargeted::file_key(params, &ix).map_err(js_err)
}

/// Checks the bytes of a cached `.mzfeatf` against its `key` and returns the name of the source in its meta.
#[wasm_bindgen(js_name = check_stored)]
pub fn check_stored_js(bytes: &[u8], key: &str) -> std::result::Result<String, JsValue> {
    check_stored(bytes, key).map_err(js_err)
}
