//! `native_samples`: the samples of a dataset, each with why it has nothing to show (`readers::samples_json`, the listing the app gives too).

use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::natives::Members;

/// The samples of a dataset (`names`, `sizes`, `read(member, offset, length)`): the JSON of `mzlab_core::readers::samples_json`.
#[wasm_bindgen]
pub fn native_samples(
    names: js_sys::Array,
    sizes: &[f64],
    read: Function,
) -> Result<String, JsValue> {
    let mut l = Members::new(names, sizes, read)?.open(false)?;
    Ok(mzlab_core::readers::samples_json(&mut *l))
}
