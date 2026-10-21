//! The memory aligning a series asks for, as the page shows it before the step runs (Flow web runner, `docs/agenti/flow-web.md`): the rule is
//! `mzlab_feat::slabs::stima`; this module only reads the stored files and writes the JSON.

use mzlab_core::budget::MemBudget;
use mzlab_core::error::Result;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::slabs::{rows_of, stima, Stored, Via};

/// What aligning the stored series `src` asks for under `budget_mb` (0/NaN = the default): `{"mb", "via", "files", "rows"}` with `via` = `memory`, `slabs` or
/// `refused` (the slabs do not fit either: what `align_and_fill_stored` refuses with `error.memoryBudget`).
pub fn stima_json(src: &mut dyn Stored, r: &Recipe, budget_mb: f64) -> Result<String> {
    let rows = rows_of(src)?;
    let cap = MemBudget::from_request(budget_mb).cap();
    let s = stima(&rows, r, cap);
    let via = if s.bytes > cap { Via::Refused } else { s.via };
    Ok(format!(
        "{{\"mb\":{},\"via\":\"{}\",\"files\":{},\"rows\":{}}}",
        s.bytes.div_ceil(1 << 20),
        via.name(),
        rows.len(),
        rows.iter().sum::<usize>()
    ))
}

#[cfg(feature = "detect")]
mod export {
    use js_sys::Array;
    use wasm_bindgen::prelude::*;

    /// The memory aligning the `.mzfeatf` of a series (an `Array` of `Uint8Array`) asks for under `budget_mb`: JSON `{"mb","via","files","rows"}`.
    #[wasm_bindgen]
    pub fn stima_series(
        files: Array,
        recipe: &str,
        budget_mb: f64,
    ) -> std::result::Result<String, JsValue> {
        let r = crate::run::recipe_from_json(recipe).map_err(crate::js_err)?;
        super::stima_json(&mut crate::JsStored { files }, &r, budget_mb).map_err(crate::js_err)
    }
}
