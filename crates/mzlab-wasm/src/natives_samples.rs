//! `native_samples`: the samples of a dataset, each with why it has nothing to show (`readers::list_samples`, the listing the app and `mzlab-cli samples list` give too).

use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::natives::Members;

/// The samples of a dataset (`names`, `sizes`, `read(member, offset, length)`): the JSON `samples_list/1` of `mzlab_core::readers::list_samples`. With `listing`
/// a `.wiff` is listed from its container and index alone (its `.wiff.scan` is not read: counts are the index's); the page's default reads the block heads.
#[wasm_bindgen]
pub fn native_samples(
    names: js_sys::Array,
    sizes: &[f64],
    read: Function,
    listing: Option<bool>,
) -> Result<String, JsValue> {
    let mut l = Members::new(names, sizes, read)?.open_as(false, listing.unwrap_or(false))?;
    Ok(mzlab_core::readers::list_samples(&mut *l))
}

/// The ids `text` (`1,3,5`, `2-4`) selects among the `present` ids of a dataset, as `{"samples":[..]}` in the order of the file: what `--sample` of the
/// CLI answers (`readers::selection`). An id the file does not have is an error naming the ids it has.
#[wasm_bindgen]
pub fn sample_selection(present: &[u32], text: &str) -> Result<String, JsValue> {
    mzlab_core::readers::selection::selection_json(present, text).map_err(|e| crate::error_json(&e))
}

/// The ids the chooser ticks at the start (`readers::selection::suggest`): all when the section bytes `sizes` of the samples `ids` are small against `budget_mb`, none
/// otherwise; `{"samples":[..]}`.
#[wasm_bindgen]
pub fn sample_default(ids: &[u32], sizes: &[f64], budget_mb: f64) -> String {
    let bytes = |x: f64| {
        if x.is_finite() && x > 0.0 {
            x as u64
        } else {
            0
        }
    };
    let sizes: Vec<u64> = sizes.iter().map(|x| bytes(*x)).collect();
    mzlab_core::readers::selection::suggest_json(
        ids,
        &sizes,
        mzlab_core::readers::selection::budget_bytes(budget_mb),
    )
}
