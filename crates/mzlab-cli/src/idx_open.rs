//! `open` of a `.mzidx` that keeps the last few parsed: a series reads each of its indexes in several passes (recipe, strong and weak detection,
//! alignment, gap filling), and each open parsed the whole JSON scan table again. A kept index serves only when the header, meta and scan table
//! read back are the same bytes (`idxread::Head`), so a file written again is parsed again.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use mzlab_core::idxread::{Head, Index};
use mzlab_core::Result;

/// Indexes kept, the most recently opened last, and the bytes of their heads held at most: the scan tables stay in memory, so a few (a series of
/// more files parses each again, as without the store).
const KEPT: usize = 8;
const KEPT_BYTES: usize = 256 << 20;

type Kept = (PathBuf, Head, Index<File>);
static KEPT_INDEXES: Mutex<Vec<Kept>> = Mutex::new(Vec::new());

/// The files whose scan table was parsed, for the tests.
#[cfg(test)]
pub(crate) static PARSED: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

pub(crate) fn open(idx: &Path) -> Result<Index<File>> {
    let mut f = File::open(idx).map_err(crate::io)?;
    let size = f.metadata().map_err(crate::io)?.len();
    let head = Head::read(&mut f, size)?;
    let lock = || KEPT_INDEXES.lock().unwrap_or_else(|e| e.into_inner());
    {
        let mut kept = lock();
        if let Some(i) = kept.iter().position(|(p, h, _)| p == idx && *h == head) {
            let e = kept.remove(i);
            let ix = e.2.reopen(f);
            kept.push(e);
            return Ok(ix);
        }
    }
    let ix = Index::open_head(f.try_clone().map_err(crate::io)?, &head)?;
    #[cfg(test)]
    PARSED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(idx.to_path_buf());
    let out = ix.reopen(f);
    let mut kept = lock();
    kept.retain(|(p, _, _)| p != idx);
    kept.push((idx.to_path_buf(), head, ix));
    while kept.len() > KEPT || kept.iter().map(|k| k.1.len()).sum::<usize>() > KEPT_BYTES {
        kept.remove(0);
    }
    Ok(out)
}

#[cfg(test)]
#[path = "idx_open_tests.rs"]
mod tests;
