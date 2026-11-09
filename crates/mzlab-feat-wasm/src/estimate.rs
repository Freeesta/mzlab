//! The memory aligning a series and detecting one file ask for, as the page shows it before the step runs (Flow web runner, `docs/agenti/flow-web.md`): the rule is
//! `mzlab_feat::slabs::stima` and `mzlab_feat::trace_estimate`; this module only reads the stored files or the index and writes the JSON.

use mzlab_core::budget::MemBudget;
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
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

/// `Ok` when the detection of `index` under `r` fits in `cap` bytes (`trace_stima`), else `error.memoryBudget` with the phase `detect`: asked before
/// `detect` by the page, the app and `flow run`, so a file too large is refused with its numbers instead of growing past the memory.
pub fn detect_fits<R: ReadAt>(index: &mut Index<R>, r: &Recipe, cap: usize) -> Result<()> {
    let x = mzlab_feat::trace_estimate::inputs(index, r)?;
    mzlab_feat::trace_estimate::check(&x, r, cap)
}

/// What detecting `index` under `r` asks for against `budget_mb` (0 or NaN: the default of the `.wasm`): `{"mb","via"}`, `via` = `memory` or `refused`.
pub fn detect_estimate_json<R: ReadAt>(
    index: &mut Index<R>,
    r: &Recipe,
    budget_mb: f64,
) -> Result<String> {
    let x = mzlab_feat::trace_estimate::inputs(index, r)?;
    let need = mzlab_feat::trace_estimate::bytes(&x, r);
    let via = if need > MemBudget::from_request(budget_mb).cap() {
        "refused"
    } else {
        "memory"
    };
    Ok(format!(
        "{{\"mb\":{},\"via\":\"{via}\"}}",
        need.div_ceil(1 << 20)
    ))
}

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
        super::stima_json(&mut crate::JsStored { files }, &r, budget_mb).map_err(crate::js_err)
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
        super::detect_estimate_json(&mut ix, &r, budget_mb).map_err(crate::js_err)
    }
}
