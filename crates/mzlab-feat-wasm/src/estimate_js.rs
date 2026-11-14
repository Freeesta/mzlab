//! JS exports of `estimate`; the plain functions are `mzlab_ops::run::feat::estimate`.

#[cfg(feature = "detect")]
mod export {
    use js_sys::Array;
    use wasm_bindgen::prelude::*;

    /// The memory aligning the `.mzfeatf` of a series (an `Array` of `Uint8Array`) asks for under `budget_mb`: JSON `{"mb","via","files","rows"}`.
    #[wasm_bindgen]
    pub fn estimate_series(
        files: Array,
        recipe: &str,
        budget_mb: f64,
    ) -> std::result::Result<String, JsValue> {
        let r = crate::run::recipe_from_json(recipe).map_err(crate::js_err)?;
        mzlab_ops::run::feat::estimate::estimate_json(&mut crate::JsStored { files }, &r, budget_mb)
            .map_err(crate::js_err)
    }

    /// The memory detecting one index (`read(0, offset, length)`, `size` bytes) asks for under `budget_mb`, with the recipe of `recipe_for`: JSON `{"mb","via"}`.
    #[wasm_bindgen]
    pub fn estimate_detect(
        read: js_sys::Function,
        size: f64,
        recipe: &str,
        budget_mb: f64,
    ) -> std::result::Result<String, JsValue> {
        let mut ix = crate::open_one(&read, 0, size).map_err(crate::js_err)?;
        let r = crate::run::recipe_for(recipe, &mut ix).map_err(crate::js_err)?;
        mzlab_ops::run::feat::estimate::detect_estimate_json(&mut ix, &r, budget_mb)
            .map_err(crate::js_err)
    }
}
