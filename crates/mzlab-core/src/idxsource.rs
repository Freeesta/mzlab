//! Whether an index still belongs to the file it was built from: the `source` of its metadata against the file's size and `probe`
//! (SHA-1 of the size and of the first and last 64 KB, `upload::probe`). A file replaced by another with the same name, size and date
//! is caught here, so the reuse paths (desktop at start-up, the warmed files of the site, a live CLI run) build its index again.

use crate::error::Result;
use crate::idxread::{Index, ReadAt};
use crate::json::Value;
use crate::upload;

/// Whether `source` describes a file of `size` bytes with this `probe`; a source without both (a damaged or foreign index) never does.
pub fn same_print(source: &Value, size: u64, probe: &str) -> bool {
    let size_ok = source.get("size").and_then(Value::f64) == Some(size as f64);
    size_ok && source.get("probe").and_then(Value::str) == Some(probe)
}

/// Whether `source` describes the file of `size` bytes read by `r` (only its first and last 64 KB are read).
pub fn same_file<R: ReadAt>(source: &Value, r: &mut R, size: u64) -> Result<bool> {
    if source.get("size").and_then(Value::f64) != Some(size as f64) {
        return Ok(false);
    }
    Ok(same_print(source, size, &upload::probe(r, size)?))
}

impl<R: ReadAt> Index<R> {
    /// Whether this index was built from the file of `size` bytes read by `f` (`same_file` on its `source`).
    pub fn built_from<F: ReadAt>(&self, f: &mut F, size: u64) -> Result<bool> {
        same_file(&self.source, f, size)
    }
}
