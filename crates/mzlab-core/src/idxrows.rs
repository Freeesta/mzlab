//! The scan table of an index as the compact JSON rows its file holds, read again from the file: for a copy of the index whose rows change in a few
//! columns (`centroids::index`), and for a lean index (`Index::open_lean`) that needs its scan table after all (`load_scans`).

use crate::error::{Error, Result};
use crate::idxread::{nativo, u64_at, Head, Index, ReadAt, HEADER};
use crate::json;
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

    /// The scan table of an index opened lean (`Index::open_lean`), read and parsed now: the index is then what `open` would have given (a no-op when
    /// it holds its scan table). For the one file of the gap filling that is centroided, whose centroids are built from the scan rows.
    pub fn load_scans(&mut self) -> Result<()> {
        if !self.scans.is_empty() {
            return Ok(());
        }
        let bad = || Error::new("error.badIndex");
        let mut h = [0u8; HEADER];
        index_bytes(&mut self.r, 0, &mut h)?;
        let nat = nativo(&h).ok_or_else(bad)?;
        let head = Head::read(&mut self.r, u64_at(&h, 48))?;
        let rows = head.rows.unwrap_or_default();
        let mut scans = crate::budget::scan_table(json::count_elements(&rows)?)?;
        json::each_element(&rows, |r| {
            scans.push(crate::scanrow::scan_of_row(r, nat).ok_or_else(bad)?);
            Ok(())
        })?;
        self.layout = crate::layout::Layout::of_index(&self.source, &scans);
        self.layout.seed(&self.run, nat).ok_or_else(bad)?;
        self.scans = std::sync::Arc::new(scans);
        Ok(())
    }
}
