//! The base of the containers `.mzfeatf` and `.mzfeat` (magic, alignment, version, error, origin, size of the pieces, the section written in
//! pieces) that `store` writes and `store_read` reads, apart from both so that the reader and `evid` do not depend on the writer. `store` re-exports it.

use mzlab_core::error::{Error, Result};

use crate::facts::FileFacts;

pub(crate) const MAGIC: &[u8; 8] = b"MZFEAT01";
pub(crate) const ALIGN: usize = 64;
/// Version of the container of the aligned table (`"version"` of its meta): 2 adds the `id`, the summaries of the rows and, optional, the identity columns (`identity` = 1 in the meta); the files of version 1 are still read.
pub const TABLE_FORMAT: i64 = 2;

pub(crate) fn bad() -> Error {
    Error::new("error.badMzfeat")
}

/// What the meta says about the origin of a table.
#[derive(Debug, Clone, Default)]
pub struct Origin {
    /// `(name, SHA-256)` of every input file, in the order of the files of the table.
    pub inputs: Vec<(String, String)>,
    /// How each input was acquired (meta `columns`, same order as `inputs`); empty when not read.
    pub facts: Vec<FileFacts>,
    /// SHA-256 of the `campioni/1` document of the series (meta `samples`), when there is one.
    pub samples: Option<String>,
    /// `(step, seconds)`; leave empty to get the same bytes in two runs.
    pub times: Vec<(String, f64)>,
}

/// Elements converted and written at a time (the buffer of a write is at most `PIECE * 8` bytes).
pub(crate) const PIECE: usize = 8192;

/// A section that is written in pieces and whose length is known beforehand (the `evid` graph: tens of MB that never exist as one buffer).
pub trait Section {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Gives the bytes, in order, to `put`; they must add up to `len()`.
    fn emit(&mut self, put: &mut dyn FnMut(&[u8]) -> Result<()>) -> Result<()>;
}

impl Section for &[u8] {
    fn len(&self) -> usize {
        <[u8]>::len(self)
    }
    fn emit(&mut self, put: &mut dyn FnMut(&[u8]) -> Result<()>) -> Result<()> {
        put(self)
    }
}
