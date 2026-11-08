//! The index of a file of the series read through a cache of a few blocks (`mzlab_core::blocchi::BlockCache`): every read of `JsReader` is a call to
//! JavaScript and an OPFS sync-handle read, whose cost is mostly fixed per call (about 0.2 ms in Chromium), while the engine asks a few KB at a time
//! (the slabs of the detection and of the weak candidates, the gap filling). The bytes given back are the same, so every output is.

use mzlab_core::blocchi::BlockCache;
use mzlab_core::idxread::ReadAt;
use mzlab_core::Result;

use crate::JsReader;

/// First block of a run of reads, and the longest a run in order grows to (`BlockCache::read_ahead`).
const BLOCK: usize = 256 << 10;
const BLOCK_MAX: usize = 1 << 20;
/// Blocks kept per file: the detection reads a few regions of the index in turn (scan table, peaks).
const SLOTS: usize = 3;

type Fetch = Box<dyn FnMut(u64, &mut [u8]) -> Result<()>>;

/// `JsReader` behind the cache (at most `SLOTS` × `BLOCK_MAX` bytes per open file).
pub(crate) struct Blocks(BlockCache<Fetch>);

impl Blocks {
    pub(crate) fn over(mut r: JsReader, size: u64) -> Self {
        let fetch: Fetch = Box::new(move |off, buf| r.read_at(off, buf));
        Blocks(BlockCache::new(fetch, size, BLOCK, SLOTS).read_ahead(BLOCK_MAX))
    }
}

impl ReadAt for Blocks {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.0.read_at(off, buf)
    }
}
