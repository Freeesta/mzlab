//! What a table file is written with, one rule for `flow run` and the browser: the polarity and the recipe of the table when it has one, else the sign of the
//! polarities of the files of its lineage, positive when neither says.
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::samples as core_samples;
use mzlab_feat::facts::file_facts;
use mzlab_feat::store::read_table_from;

/// `-1` or `1`; `indexes` are the `.mzidx` of the files the table was made from (none when they are not at hand).
pub fn of_table<R: ReadAt, I: ReadAt>(r: &mut R, size: u64, indexes: &[Index<I>]) -> Result<i32> {
    let (_, meta) = read_table_from(r, size)?;
    let recipe = meta.get("recipe").and_then(|v| v.str()).unwrap_or("");
    if recipe.contains("polarity: Some(") && !recipe.contains("polarity: Some(0)") {
        return Ok(if recipe.contains("polarity: Some(-1)") {
            -1
        } else {
            1
        });
    }
    let sum: i64 = indexes
        .iter()
        .map(|ix| i64::from(file_facts(ix).polarity))
        .sum();
    Ok(if sum < 0 { -1 } else { 1 })
}

/// The samples text with its files in the order of the files of the table (`core_samples::order_like` on the names its meta records): the readers pair them by position.
pub fn samples_like<R: ReadAt>(r: &mut R, size: u64, bytes: &[u8]) -> Result<String> {
    let names = mzlab_feat::store::table_names(r, size)?;
    core_samples::order_like(&String::from_utf8_lossy(bytes), &names)
}
