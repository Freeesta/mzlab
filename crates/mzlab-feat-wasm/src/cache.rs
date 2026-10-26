//! The cache of the detections: the name of the stored `.mzfeatf` of a file (`<sha256 of the .mzidx>.<fingerprint of what detection depends on>`) and the check of
//! a stored file against it. `lib.rs` exports `file_key` and `check_stored`; `feat-run.js` keeps the files in OPFS under that name.

use js_sys::Function;
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::json::Value;
use mzlab_feat::series::source_sha256;
use wasm_bindgen::prelude::*;

use crate::run::recipe_for;
use crate::{js_err, open_one};

pub(crate) fn key_of(sha: &str, hash: &str) -> String {
    if sha.is_empty() {
        String::new()
    } else {
        format!("{sha}.{hash}")
    }
}

/// Name of the cached `.mzfeatf` of a file, empty when the index has no SHA-256 (nothing to key on). The recipe is the one `detect` would use (`recipe_for`), so
/// the file name, the alignment and the proofs never change the key.
pub fn file_key<R: ReadAt>(text: &str, index: &mut Index<R>) -> Result<String> {
    let r = recipe_for(text, index)?;
    Ok(key_of(&source_sha256(index), &r.file_hash()))
}

/// Checks that the bytes of a cached `.mzfeatf` are the file `key` names (SHA-256 of the source and, but for the untargeted engine, the fingerprint of the
/// recipe, both in the meta) and returns the name of the source in the meta. A file of a newer format is `error.newerFormat`; any other mismatch `error.badMzfeatf`.
pub fn check_stored(bytes: &[u8], key: &str) -> Result<String> {
    let bad = || Error::new("error.badMzfeatf");
    let (sha, hash) = key.split_once('.').ok_or_else(bad)?;
    let (meta, _) = mzlab_feat::store::read_meta_mz(bytes)?;
    let input = meta
        .get("inputs")
        .map(Value::arr)
        .and_then(<[Value]>::first)
        .ok_or_else(bad)?;
    let same_sha = input.get("sha256").and_then(Value::str) == Some(sha);
    let same_recipe = hash.starts_with('u')
        || meta
            .get("recipe_hash")
            .and_then(Value::str)
            .is_some_and(|h| h.starts_with(hash));
    if !same_sha || !same_recipe {
        return Err(bad());
    }
    Ok(input
        .get("name")
        .and_then(Value::str)
        .unwrap_or("")
        .to_string())
}

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
