//! The pass over a stored table from outside the crate: reads the `.mzfeat` by offsets, the parent and the registry from their JSON, runs `build` and
//! writes the `.mzfeat` with the block `interp`. Used by the `.wasm` `interp`, the desktop engine and the tests, so all three write the same bytes.

use mzlab_core::budget::WriteAt;
use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};

use super::{build, json, store, Inputs};
use crate::legacy::series;
use crate::store_read::read_table_from;

fn user(e: String) -> Error {
    match e.as_str() {
        "error.interp.noTp" => Error::new("error.interp.noTp"),
        _ => Error::new("error.interp.failed").with("detail", e),
    }
}

/// Interprets the rows of the `.mzfeat` (`size` bytes) and writes it again, with the block, to `sink`. `open(k)` gives the index of file `k` of the
/// series, `infusion` the one of the infusion file when there is one; `registry` is the `ipotesi/1` JSON (may be empty).
#[allow(clippy::too_many_arguments)]
pub fn run_to<R: ReadAt, I: ReadAt, W: WriteAt>(
    table: &mut R,
    size: u64,
    parent_json: &str,
    registry: &str,
    polarity: i8,
    open: &mut dyn FnMut(usize) -> Result<Index<I>>,
    infusion: Option<&mut Index<I>>,
    sink: &mut W,
) -> Result<()> {
    let (t, _) = read_table_from(table, size)?;
    let parent = series::parent(parent_json).map_err(user)?;
    let (hyps, _, _) = series::registry(registry).map_err(user)?;
    let mut hyps: Vec<_> = hyps.into_iter().collect();
    hyps.sort_by_key(|h| h.0);
    let hyps: Vec<_> = hyps.into_iter().map(|h| h.1).collect();
    let inp = Inputs {
        table: &t,
        parent: &parent,
        hyps: &hyps,
        polarity,
    };
    let mut fail = None;
    let out = build(
        &inp,
        &mut |k| {
            open(k).map_err(|e| {
                fail = Some(e.clone());
                e.key.to_string()
            })
        },
        infusion,
    );
    let out = match out {
        Ok(o) => o,
        Err(e) => return Err(fail.unwrap_or_else(|| user(e))),
    };
    let rows: Vec<(u32, String)> = out
        .rows
        .iter()
        .map(|r| (r.id, json::row_json(r, &parent)))
        .collect();
    store::append(table, size, sink, &json::meta_json(&out, &parent), &rows)
}
