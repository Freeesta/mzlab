//! The scan table of an index as the compact JSON rows its file holds, read again from the file: for a copy of the index whose rows change in a few
//! columns (`centroids::index`).

use crate::error::Result;
use crate::idxread::{u64_at, Head, Index, ReadAt, HEADER};
use crate::readat::index_bytes;

impl<R: ReadAt> Index<R> {
    /// The rows of the scan table (`SCAN_COLUMNS`), as the file has them.
    pub fn rows_bytes(&mut self) -> Result<Vec<u8>> {
        let mut h = [0u8; HEADER];
        index_bytes(&mut self.r, 0, &mut h)?;
        let size = u64_at(&h, 48);
        let head = Head::read(&mut self.r, size)?;
        Ok(head.rows.unwrap_or_default())
    }
}
