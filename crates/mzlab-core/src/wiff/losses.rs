//! What a read of a `.wiff` pair left out of a sample (`Reader::losses`): the blocks cut by the end of the `.wiff.scan` (`Record::cut`), the blocks with
//! no scan head where the index row says points (`Record::damaged`) and the scan blocks whose bytes contradict their head (`scan::Block::Damaged`).

use super::reader::WiffReader;
use crate::error::Result;
use crate::readers::{Losses, Reader, DAMAGED, TRUNCATED};

impl WiffReader {
    pub(super) fn losses_of(&mut self, c: usize) -> Result<Losses> {
        let (cut, n, len) = (self.missing(c), self.index(c)?.len(), self.scan_len);
        let p = self.ready(c)?;
        let rows = p.w.records.iter().zip(&p.counts);
        let bad = p.damaged
            + rows
                .filter(|(r, k)| !r.cut(len) && r.damaged(k.is_some()))
                .count();
        Ok(Losses::of(
            n + cut + bad,
            n,
            if cut > 0 { TRUNCATED } else { DAMAGED },
        ))
    }
}
