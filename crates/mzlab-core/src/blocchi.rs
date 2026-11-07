//! A read cache of a few blocks over a member read from far apart: a reader like `opentfraw` asks a few bytes at a time and alternates between regions (the
//! packets of the scans, the trailer records, the index), so one block thrown away at every far read is read again at the next. Used by the `thermo` module's
//! `JsMember` (each fetch is a JS call) and by the `raw_tempi` simulator; it holds `slots` blocks and replaces the least recently used. With `read_ahead`, a miss that
//! continues a block or lands a little after it (the reader going on in order, as along the scans) refills that same slot with a block twice as long, up to the cap: fewer and larger fetches.
use crate::idxread::ReadAt;
use crate::Result;

struct Slot {
    start: u64,
    data: Vec<u8>,
    used: u64,
}

/// `fetch(offset, buf)` fills `buf` with the bytes of the member at `offset` (and refuses a range past the end); the cache reads blocks of `size` bytes through it.
pub struct BlockCache<F> {
    fetch: F,
    dim: u64,
    size: usize,
    /// The longest block a run of reads in order grows to (`size` without read-ahead).
    max: usize,
    slots: Vec<Slot>,
    tick: u64,
}

impl<F: FnMut(u64, &mut [u8]) -> Result<()>> BlockCache<F> {
    pub fn new(fetch: F, dim: u64, size: usize, slots: usize) -> Self {
        BlockCache {
            fetch,
            dim,
            size,
            max: size,
            slots: (0..slots.max(1))
                .map(|_| Slot {
                    start: 0,
                    data: Vec::new(),
                    used: 0,
                })
                .collect(),
            tick: 0,
        }
    }

    /// Blocks that continue one another double up to `max` bytes (an OPFS or `File` read costs a fixed part per call: 1-4 MiB reads are 2-3x faster per byte than 256 KiB ones in Chromium).
    pub fn read_ahead(mut self, max: usize) -> Self {
        self.max = max.max(self.size);
        self
    }
}

impl<F: FnMut(u64, &mut [u8]) -> Result<()>> ReadAt for BlockCache<F> {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.tick += 1;
        // a range that wraps past u64::MAX is no cache hit: the fetch refuses it
        let Some(end) = off.checked_add(buf.len() as u64) else {
            return (self.fetch)(off, buf);
        };
        let (mut victim, mut oldest) = (0, u64::MAX);
        let mut size = self.size;
        let mut seguito = None;
        for (i, s) in self.slots.iter_mut().enumerate() {
            let fine = s.start + s.data.len() as u64;
            if off >= s.start && end <= fine {
                s.used = self.tick;
                let a = (off - s.start) as usize;
                buf.copy_from_slice(&s.data[a..a + buf.len()]);
                return Ok(());
            }
            if s.used < oldest {
                (victim, oldest) = (i, s.used);
            }
            // a read that runs past the end of a block, or starts after it by less than its length (the reader skips a profile to the centroids): the reader goes on in order
            if seguito.is_none()
                && !s.data.is_empty()
                && off >= s.start
                && off <= fine.saturating_add(s.data.len() as u64)
            {
                seguito = Some((i, s.data.len()));
            }
        }
        if self.max > self.size {
            if let Some((i, len)) = seguito {
                // the run refills its own slot (the block behind it is read) with a longer block, and leaves the other runs' blocks alone
                victim = i;
                size = len.saturating_mul(2).clamp(self.size, self.max);
            }
        }
        let n = self.dim.saturating_sub(off).min(size as u64) as usize;
        if buf.len() >= size || n < buf.len() {
            // big (or past the end, for the fetch to refuse): as asked
            return (self.fetch)(off, buf);
        }
        let s = &mut self.slots[victim];
        // the fetch overwrites the whole block: only the bytes a longer block adds are zeroed, not 256 KB at every miss
        s.data.resize(n, 0);
        let r = (self.fetch)(off, &mut s.data);
        if r.is_err() {
            // a failed fetch must not leave a block that claims bytes it does not hold
            s.data.clear();
            return r;
        }
        (s.start, s.used) = (off, self.tick);
        buf.copy_from_slice(&s.data[..buf.len()]);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
