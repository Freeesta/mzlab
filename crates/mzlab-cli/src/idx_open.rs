//! `open` of a `.mzidx` that keeps the last few parsed: a series reads each of its indexes in several passes (recipe, strong and weak detection,
//! alignment, gap filling), and each open parsed the whole JSON scan table again. A kept index serves only when the header, meta and scan table
//! read back are the same bytes (`idxread::Head`), so a file written again is parsed again.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use mzlab_core::idxread::{Head, Index};
use mzlab_core::Result;

/// Files kept, the most recently opened last, and the bytes of their heads held at most: the scan tables stay in memory, so a few (a series of
/// more files parses each again, as without the store).
const KEPT: usize = 8;
const KEPT_BYTES: usize = 256 << 20;

/// One file's last parsed index and the head it was parsed from. The opens of a file take its slot in turn, so the strong and the weak detection
/// that start together parse it once; other files parse meanwhile.
type Slot = Arc<Mutex<Option<(Head, Index<File>)>>>;

/// (file, its slot, the bytes of the head it holds)
static KEPT_INDEXES: Mutex<Vec<(PathBuf, Slot, usize)>> = Mutex::new(Vec::new());

/// The files whose scan table was parsed, for the tests.
#[cfg(test)]
pub(crate) static PARSED: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

fn slot_of(idx: &Path) -> Slot {
    let mut kept = KEPT_INDEXES.lock().unwrap_or_else(|e| e.into_inner());
    let e = match kept.iter().position(|e| e.0 == idx) {
        Some(i) => kept.remove(i),
        None => (idx.to_path_buf(), Slot::default(), 0),
    };
    let slot = e.1.clone();
    kept.push(e);
    while kept.len() > 1
        && (kept.len() > KEPT || kept.iter().map(|e| e.2).sum::<usize>() > KEPT_BYTES)
    {
        kept.remove(0);
    }
    slot
}

pub(crate) fn open(idx: &Path) -> Result<Index<File>> {
    let mut f = File::open(idx).map_err(crate::io)?;
    let size = f.metadata().map_err(crate::io)?.len();
    let head = Head::read(&mut f, size)?;
    let slot = slot_of(idx);
    let mut held = slot.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((h, ix)) = held.as_ref()
        && *h == head
    {
        return Ok(ix.over(f));
    }
    // a second handle of the path, not `try_clone`: WASI preview 1 cannot duplicate a descriptor (the wasm flow)
    let ix = Index::open_head(File::open(idx).map_err(crate::io)?, &head)?;
    #[cfg(test)]
    PARSED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(idx.to_path_buf());
    let (out, bytes) = (ix.over(f), head.len());
    *held = Some((head, ix));
    drop(held);
    let mut kept = KEPT_INDEXES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(e) = kept.iter_mut().find(|e| Arc::ptr_eq(&e.1, &slot)) {
        e.2 = bytes;
    }
    Ok(out)
}

#[cfg(test)]
#[path = "idx_open_tests.rs"]
mod tests;
