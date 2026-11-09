//! The indexes of a series opened once for one step of the alignment (`features.align`, `features.gapfill`): the check of the kind, the recipe, the
//! facts of the files and the gap filling each take a copy over a new handle of the same file (`Index::over`) instead of opening and parsing it again.
//! A file whose size or time of change differs from the first open is opened again in full, so a copy never meets other bytes. Past the check of the kind
//! only the first indexes whose tables fit in `KEEP_BYTES` stay in memory; the others are opened in full each time, as before.

use std::fs::File;
use std::path::PathBuf;
use std::time::SystemTime;

use mzlab_core::error::Result;
use mzlab_core::idxread::Index;

use crate::flow::io;
use crate::flow_base::open;

/// The memory the tables of the kept indexes may take during the step (a long series keeps its first indexes, never all).
pub(crate) const KEEP_BYTES: usize = 256 << 20;

/// About how many bytes the tables of `ix` take in memory (the scan table and its filters, the keys).
pub(crate) fn weight(ix: &Index<File>) -> usize {
    let scans: usize = ix.scans.iter().map(|s| s.filter.capacity()).sum();
    let keys: usize = ix.keys.iter().map(|k| k.0.capacity() + 24).sum();
    ix.scans.len() * std::mem::size_of::<mzlab_core::idxread::Scan>() + scans + keys
}

/// A file of an opened index: its path, and the size and time of change it had at the first open.
struct Seen {
    path: PathBuf,
    size: u64,
    changed: Option<SystemTime>,
}

/// The indexes as first opened (all until `keep`, then the first ones), beside their files.
pub(crate) struct Indexes {
    indexes: Vec<Index<File>>,
    files: Vec<Seen>,
}

impl Indexes {
    /// Opens every index of `paths` once, in order.
    pub(crate) fn open(paths: &[PathBuf]) -> Result<Self> {
        let (mut indexes, mut files) = (Vec::new(), Vec::new());
        for p in paths {
            // the size and time are read first: a change after them makes `get` open the file again rather than copy older tables
            let meta = std::fs::metadata(p).map_err(io)?;
            indexes.push(open(p)?);
            files.push(Seen {
                path: p.clone(),
                size: meta.len(),
                changed: meta.modified().ok(),
            });
        }
        Ok(Indexes { indexes, files })
    }

    /// The indexes as they were opened (read only: the check of the kind of the series, before `keep`).
    pub(crate) fn all(&self) -> &[Index<File>] {
        &self.indexes
    }

    /// Keeps the first indexes whose tables take at most `bytes` together and drops the others (`get` opens those in full).
    pub(crate) fn keep(&mut self, bytes: usize) {
        let mut total = 0;
        let n = self.indexes.iter().take_while(|ix| {
            total += weight(ix);
            total <= bytes
        });
        let n = n.count();
        self.indexes.truncate(n);
    }

    /// Where `path` is among the opened indexes.
    pub(crate) fn position(&self, path: &PathBuf) -> Option<usize> {
        self.files.iter().position(|f| &f.path == path)
    }

    /// Index `k` as `open` would give it now: a copy of its tables over a new handle, or a full open when it was not kept or the file changed since.
    pub(crate) fn get(&self, k: usize) -> Result<Index<File>> {
        let seen = &self.files[k];
        if k >= self.indexes.len() {
            return open(&seen.path);
        }
        let f = File::open(&seen.path).map_err(io)?;
        let meta = f.metadata().map_err(io)?;
        if meta.len() != seen.size || meta.modified().ok() != seen.changed {
            return open(&seen.path);
        }
        Ok(self.indexes[k].over(f))
    }
}
