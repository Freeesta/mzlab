//! The `famiglia/1` record of one file on demand (`family_record`): L1-L3 done again keeping the families with their arcs and proofs, which neither the
//! `.mzfeatf` nor the `.mzfeat` store. Built into the `.wasm` of the feature `detect`; the page asks for it per file (`docs/agenti/mzfeat-wasm-famiglia.md`).

use js_sys::Function;
use mzlab_core::budget::MemBudget;
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_feat::family::analyse_families;
use mzlab_feat::famrecord;
use mzlab_feat::pipeline::analyse;
use mzlab_feat::recipe::Recipe;
use wasm_bindgen::prelude::*;

use crate::{js_err, open_one, run};

/// Bytes the `famiglia/1` text may take per row and per arc: the estimate reserved from the budget before the record is built.
const ROW_BYTES: usize = 600;
const ARC_BYTES: usize = 900;

/// Same steps as `series::analyse_file`, so the families are the ones of the stored table; refused with `error.memoryBudget` if the text would not fit.
pub fn record<R: ReadAt>(
    index: &mut Index<R>,
    name: &str,
    r: &Recipe,
    budget: &MemBudget,
) -> Result<String> {
    let (mut f, links) = analyse(index, &r.detect, &r.steps)?;
    let fam = analyse_families(index, &mut f, &links, &r.detect, &r.fam)?;
    let _guard = budget.reserve(
        "family_record",
        f.len() * ROW_BYTES + fam.arcs.len() * ARC_BYTES,
    )?;
    Ok(famrecord::record(name, &f, &fam))
}

/// The `famiglia/1` record of one file (`.mzidx` of `size` bytes read with `read`). `recipe` as in `detect_file`; the budget is the default of the WASM
/// (192 MB). JSON text, or the error `error.badRecipe` / `error.memoryBudget`.
#[wasm_bindgen]
pub fn family_record(
    read: Function,
    size: f64,
    name: &str,
    recipe: &str,
) -> std::result::Result<String, JsValue> {
    let mut ix = open_one(&read, 0, size).map_err(js_err)?;
    let r = run::recipe_for(recipe, &mut ix).map_err(js_err)?;
    let budget = MemBudget::from_request(f64::NAN);
    record(&mut ix, name, &r, &budget).map_err(js_err)
}
