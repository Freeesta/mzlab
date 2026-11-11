//! What a read of a `.wiff` pair left out of a sample (`Reader::losses`): the blocks cut by the end of the `.wiff.scan` (`Record::cut`) and the scan
//! blocks whose head disagrees with their index row (`Record::damaged`), which `build` leaves out of the index with no point count.

use super::reader::WiffReader;
use crate::error::Result;
use crate::readers::{Losses, Reader, DAMAGED, TRUNCATED};

impl WiffReader {
    pub(super) fn losses_of(&mut self, c: usize) -> Result<Losses> {
        let (cut, n, len) = (self.missing(c), self.index(c)?.len(), self.scan_len);
        let p = self.ready(c)?;
        let rows = p.w.records.iter().zip(&p.counts);
        let bad = rows
            .filter(|(r, k)| k.is_none() && !r.cut(len) && r.points > 0.0)
            .count();
        Ok(Losses::of(
            n + cut + bad,
            n,
            if cut > 0 { TRUNCATED } else { DAMAGED },
        ))
    }
}
