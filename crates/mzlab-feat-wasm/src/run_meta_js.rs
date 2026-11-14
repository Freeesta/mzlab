//! JS exports of the Flow meta of a job (`flow_job_recipe`, `flow_file_sha`, `flow_idx_source`, `flow_job_meta`); the plain functions are `mzlab_ops::run::feat::run::meta`.
#![cfg(feature = "detect")]
use mzlab_ops::run::feat::run::meta;

/// The recipe text of `instance` for the pool (`job_recipe_text`): `plan` is the `mzflow/2` text, `registry` the `step/1` array.
#[cfg(feature = "detect")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn flow_job_recipe(
    plan: &str,
    registry: &str,
    instance: &str,
    filled: bool,
) -> std::result::Result<String, wasm_bindgen::JsValue> {
    let reg = mzlab_core::json::parse(registry.as_bytes())
        .and_then(|v| mzlab_core::step::registry_from_value(&v))
        .map_err(crate::js_err)?;
    meta::job_recipe_text(plan.as_bytes(), &reg, instance, filled).map_err(crate::js_err)
}

#[cfg(feature = "detect")]
mod export {
    use super::meta;
    use js_sys::{Array, Function};
    use wasm_bindgen::prelude::*;

    /// `{"bytes","sha256"}` of file 0 of `read(0, offset, length)` (`file_sha`).
    #[wasm_bindgen]
    pub fn flow_file_sha(read: Function, size: f64) -> std::result::Result<String, JsValue> {
        let mut r = crate::JsReader { read, k: 0 };
        let sha = meta::file_sha(&mut r, size as u64).map_err(crate::js_err)?;
        Ok(format!("{{\"bytes\":{size},\"sha256\":\"{sha}\"}}"))
    }

    /// The `source` of the index that is file 0 of `read`, as it is: `size` and `sha256` of the file it was made from (what the plan names as input in
    /// the record of a run) and `lost` when its read was partial, which the page hands to `record_prov` as `flow run` records `index.mzidx`.
    #[wasm_bindgen]
    pub fn flow_idx_source(read: Function, size: f64) -> std::result::Result<String, JsValue> {
        let ix = crate::open_one(&read, 0, size).map_err(crate::js_err)?;
        Ok(mzlab_core::flow::value_text(&ix.source))
    }

    /// The meta of `instance` (`job_meta`, JSON): `plan` is the `mzflow/2` text, `registry` the `step/1` array. `read(k, offset, length)` reads index 0 (the
    /// first of the series) and 1 (the file of a detection), `sizes` their sizes; `files` the `.mzfeatf` of an alignment (empty for a detection).
    #[wasm_bindgen]
    pub fn flow_job_meta(
        plan: &str,
        registry: &str,
        instance: &str,
        read: Function,
        sizes: Vec<f64>,
        files: Array,
        budget_mb: f64,
    ) -> std::result::Result<String, JsValue> {
        let reg = mzlab_core::json::parse(registry.as_bytes())
            .and_then(|v| mzlab_core::step::registry_from_value(&v))
            .map_err(crate::js_err)?;
        let mut first = crate::open_one(&read, 0, sizes[0]).map_err(crate::js_err)?;
        let mut own = crate::open_one(&read, 1, sizes[1]).map_err(crate::js_err)?;
        let mut src = crate::JsStored { files };
        let m = meta::job_meta(
            plan.as_bytes(),
            &reg,
            instance,
            (&mut first, &mut own),
            Some(&mut src),
            budget_mb,
        )
        .map_err(crate::js_err)?;
        Ok(meta::meta_json(&m))
    }
}
