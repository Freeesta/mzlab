//! `feat/table` `row` and `slots`: the `row/2` and the `feature-slots/1` of one row of the held table, the bytes of the site (`table_row`, `table_slots`) and of the CLI.

use mzlab_core::error::Result;
use mzlab_core::idxread::ReadAt;
use mzlab_ops::run::table::core as t;
use serde_json::Value;

/// `kind` is `row` or `slots`; `p` carries `samples`, `id` and, for `slots`, the `output` name of the table.
pub(crate) fn text<R: ReadAt>(kind: &str, at: &mut R, size: u64, p: &Value) -> Result<String> {
    let samples = p["samples"].as_str().unwrap_or("");
    let id = p["id"].as_u64().unwrap_or(0) as u32;
    if kind == "slots" {
        t::slots_json(at, size, samples, id, p["output"].as_str().unwrap_or(""))
    } else {
        t::row_json(at, size, samples, id)
    }
}
