//! What a read of a `.wiff` pair left out of a sample (`Reader::losses`): the blocks cut by the end of the `.wiff.scan` (`Record::cut`), the blocks with
//! no scan head where the index row says points (`Record::damaged`), the damaged scan blocks (`scan::Block::Damaged`: bytes that contradict their head, an unknown version, a TOF block),
//! the index rows that place no block (`idx::place`) and the MRM cycles with a NaN or infinite cps (`chromatograms::Cycles`).

use super::reader::WiffReader;
use crate::error::Result;
use crate::readers::{Losses, Reader, DAMAGED, TRUNCATED};

impl WiffReader {
    /// Spectra for a sample with scans; MRM cycles (`Losses::in_cycles`) for an MRM sample that lost some, which has no spectra to count.
    pub(super) fn losses_of(&mut self, c: usize) -> Result<Losses> {
        let (cut, n, len) = (self.missing(c), self.index(c)?.len(), self.scan_len);
        self.chromatograms(c)?;
        let p = self.ready(c)?;
        let rows = p.w.records.iter().zip(&p.counts);
        let bad = p.damaged
            + p.w.unplaced
            + p.cycles.damaged
            + rows
                .filter(|(r, k)| !r.cut(len) && r.damaged(k.is_some()))
                .count();
        let cause = if cut > 0 { TRUNCATED } else { DAMAGED };
        let scans = (0..p.w.experiments.len()).any(|e| p.w.scan_experiment(e));
        // a whole MRM sample stays `complete(0)`, as every record of it was before cycles were counted
        if n == 0 && p.damaged == 0 && !scans && cut + bad > 0 {
            return Ok(Losses::of(p.cycles.read + cut + bad, p.cycles.read, cause).in_cycles());
        }
        Ok(Losses::of(n + cut + bad, n, cause))
    }
}
