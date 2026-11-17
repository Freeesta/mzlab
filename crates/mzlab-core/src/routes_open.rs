//! Opening one more index beside the files the engine already holds (site engine `open_index`/`share_index`, desktop `idxopen`/`open_lr`: the same functions).
//! Every open file keeps its scan table (about 1 KB per DDA scan) outside the budget of the cache: when the memory runs out, the cache of the
//! other files goes first (read again from their indexes, same answers), then the refusal names the files held, so the page says «close some files»
//! and never «the file is too big» or «damaged index».

use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::idxread::{Index, ReadAt};
use crate::routes::{trim_cache, Routes};

/// A reservation that failed (`error.memoryBudget` without a phase): the browser or the computer gave no more memory.
fn failed_reservation(e: &Error) -> bool {
    e.key == "error.memoryBudget" && !e.params.iter().any(|(k, _)| *k == "fase")
}

/// `open()` once; after a failed reservation the cache of `files` is emptied and `open()` runs once more. A second failure is `error.memoryBudget`
/// with `fase` `openFiles`, `aperti` (files held) and `scansioni` (their scans); any other error passes as it is.
pub fn open_beside<R: ReadAt, T>(
    files: &mut HashMap<u32, Routes<R>>,
    mut open: impl FnMut() -> Result<T>,
) -> Result<T> {
    match open() {
        Err(e) if failed_reservation(&e) => {}
        done => return done,
    }
    trim_cache(files, 0);
    open().map_err(|e| {
        if !failed_reservation(&e) {
            return e;
        }
        // the slots of one file share its scan table (`Index::over`): counted once, so `aperti` is files, not slots
        let mut tables: Vec<&std::sync::Arc<Vec<crate::idxread::Scan>>> = Vec::new();
        for r in files.values() {
            if !tables
                .iter()
                .any(|t| std::sync::Arc::ptr_eq(t, &r.idx.scans))
            {
                tables.push(&r.idx.scans);
            }
        }
        let scans: usize = tables.iter().map(|t| t.len()).sum();
        let held = tables.len();
        Error::memory_budget()
            .with("fase", "openFiles")
            .with("aperti", held)
            .with("scansioni", scans)
    })
}

/// Puts the index of one file in `slot`: over the scan table `from` already holds for the same file (the slots of one file share it, `Index::over`),
/// else opened with `open_beside`. `reader()` gives the `.mzidx` and its size. Answers `{"scans":n,"hr1":b}`.
pub fn open_slot<R: ReadAt>(
    files: &mut HashMap<u32, Routes<R>>,
    slot: u32,
    from: Option<u32>,
    mut reader: impl FnMut() -> Result<(R, u64)>,
) -> Result<String> {
    files.remove(&slot);
    let shared = match from.and_then(|f| files.get(&f)) {
        Some(r) => Some(r.idx.over(reader()?.0)),
        None => None,
    };
    let idx = match shared {
        Some(idx) => idx,
        None => open_beside(files, || {
            let (r, size) = reader()?;
            Index::open(r, size)
        })?,
    };
    let out = format!("{{\"scans\":{},\"hr1\":{}}}", idx.scans.len(), idx.hr1());
    files.insert(slot, Routes::new(idx));
    Ok(out)
}

#[cfg(test)]
#[path = "routes_open_tests.rs"]
mod tests;
