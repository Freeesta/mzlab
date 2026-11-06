//! The indexes of a series opened once for one step of the alignment (`features.align`, `features.gapfill`): the check of the kind, the recipe, the
//! facts of the files and the gap filling each take a copy over a new handle of the same file (`Index::over`) instead of opening and parsing it again.
//! A file whose size or time of change differs from the first open is opened again in full, so a copy never meets other bytes.

use std::fs::File;
use std::path::PathBuf;
use std::time::SystemTime;

use mzlab_core::error::Result;
use mzlab_core::idxread::Index;

use crate::flow::io;
use crate::flow_base::open;

/// A file of an opened index: its path, and the size and time of change it had at the first open.
struct Seen {
    path: PathBuf,
    size: u64,
    changed: Option<SystemTime>,
}

/// The indexes as first opened, beside their files.
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

    /// The indexes as they were opened (read only: the check of the kind of the series).
    pub(crate) fn all(&self) -> &[Index<File>] {
        &self.indexes
    }

    /// Where `path` is among the opened indexes.
    pub(crate) fn position(&self, path: &PathBuf) -> Option<usize> {
        self.files.iter().position(|f| &f.path == path)
    }

    /// Index `k` as `open` would give it now: a copy of its tables over a new handle, or a full open when the file changed since.
    pub(crate) fn get(&self, k: usize) -> Result<Index<File>> {
        let seen = &self.files[k];
        let f = File::open(&seen.path).map_err(io)?;
        let meta = f.metadata().map_err(io)?;
        if meta.len() != seen.size || meta.modified().ok() != seen.changed {
            return open(&seen.path);
        }
        Ok(self.indexes[k].over(f))
    }
}
