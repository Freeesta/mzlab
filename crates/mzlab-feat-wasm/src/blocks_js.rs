//! The index of a file of the series read through a cache of a few blocks (`mzlab_core::blocchi::BlockCache`): every read of `JsReader` is a call to
//! JavaScript and an OPFS sync-handle read, whose cost is mostly fixed per call (about 0.2 ms in Chromium), while the engine asks a few KB at a time
//! (the slabs of the detection and of the weak candidates, the gap filling). The bytes given back are the same, so every output is. Blocks are
//! allocated on the first miss and freed with the index; when many indexes are open together their blocks share one budget (`shape`).

use mzlab_core::blocchi::BlockCache;
use mzlab_core::idxread::ReadAt;
use mzlab_core::Result;

use crate::JsReader;

/// First block of a run of reads, and the longest a run in order grows to (`BlockCache::read_ahead`).
const BLOCK: usize = 256 << 10;
const BLOCK_MAX: usize = 1 << 20;
/// Blocks kept per file: the detection reads a few regions of the index in turn (scan table, peaks).
const SLOTS: usize = 3;
/// The most the blocks of the indexes open together may hold: wasm memory never shrinks, an iPad tab has little of it, and the proofs come after the
/// hypotheses in the same memory (`tests/test_guardie_memoria.py`).
const SHARED: usize = 4 << 20;
/// The smallest block of a crowded set: below it the fixed cost of each JS call comes back.
const BLOCK_MIN: usize = 64 << 10;

/// First block, longest block and slots of each of `n` indexes open together: one index alone keeps `SLOTS` blocks of `BLOCK` growing to `BLOCK_MAX`;
/// of a set, each keeps one block of `SHARED / n` (4 KiB multiple, between `BLOCK_MIN` and `BLOCK`), growing up to its share (at most `BLOCK_MAX`).
pub(crate) fn shape(n: usize) -> (usize, usize, usize) {
    if n <= 1 {
        return (BLOCK, BLOCK_MAX, SLOTS);
    }
    let share = (SHARED / n / 4096 * 4096).max(BLOCK_MIN);
    (share.min(BLOCK), share.min(BLOCK_MAX), 1)
}

type Fetch = Box<dyn FnMut(u64, &mut [u8]) -> Result<()>>;

/// `JsReader` behind the cache (at most `slots` × `max` bytes per open file, see `shape`).
pub(crate) struct Blocks(BlockCache<Fetch>);

impl Blocks {
    /// The reader of one of `together` indexes open at the same time.
    pub(crate) fn over(mut r: JsReader, size: u64, together: usize) -> Self {
        let (block, max, slots) = shape(together);
        let fetch: Fetch = Box::new(move |off, buf| r.read_at(off, buf));
        Blocks(BlockCache::new(fetch, size, block, slots).read_ahead(max))
    }
}

impl ReadAt for Blocks {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.0.read_at(off, buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_blocks_of_the_indexes_open_together_stay_within_the_budget() {
        for n in [2, 4, 8, 9, 12, 50, 64] {
            let (block, max, slots) = shape(n);
            assert!(block <= max && max <= BLOCK_MAX && slots == 1, "{n}");
            assert!(
                n * slots * max <= SHARED,
                "{n} indexes hold {} B",
                n * slots * max
            );
        }
        assert_eq!(shape(1), (BLOCK, BLOCK_MAX, SLOTS));
        assert_eq!(shape(4), (BLOCK, BLOCK_MAX, 1));
        assert_eq!(shape(12), (BLOCK, 348160, 1));
        // past SHARED / BLOCK_MIN indexes the floor wins: 64 KiB each
        assert_eq!(shape(100), (BLOCK_MIN, BLOCK_MIN, 1));
    }
}
