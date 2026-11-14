//! The `famiglia/1` record of one file on demand (`family_record`): L1-L3 done again keeping the families with their arcs and proofs, which neither the
//! `.mzfeatf` nor the `.mzfeat` store. Built into the `.wasm` of the feature `detect`; the page asks for it per file (`docs/agenti/mzfeat-wasm-pannello.md`).

use mzlab_core::budget::MemBudget;
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_feat::family::analyse_families;
use mzlab_feat::famrecord;
use mzlab_feat::pipeline::analyse;
use mzlab_feat::recipe::Recipe;

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
