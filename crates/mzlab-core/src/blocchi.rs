//! A read cache of a few blocks over a member read from far apart: a reader like `opentfraw` asks a few bytes at a time and alternates between regions (the
//! packets of the scans, the trailer records, the index), so one block thrown away at every far read is read again at the next. Used by the `thermo` module's
//! `JsMember` (each fetch is a JS call) and by the `raw_tempi` simulator; it holds `slots` blocks and replaces the least recently used.
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
    slots: Vec<Slot>,
    max_slots: usize,
    tick: u64,
}

impl<F: FnMut(u64, &mut [u8]) -> Result<()>> BlockCache<F> {
    pub fn new(fetch: F, dim: u64, size: usize, slots: usize) -> Self {
        BlockCache {
            fetch,
            dim,
            size,
            slots: Vec::new(),
            max_slots: slots.max(1),
            tick: 0,
        }
    }
}

impl<F: FnMut(u64, &mut [u8]) -> Result<()>> ReadAt for BlockCache<F> {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.tick += 1;
        let end = off + buf.len() as u64;
        if let Some(s) = self
            .slots
            .iter_mut()
            .find(|s| off >= s.start && end <= s.start + s.data.len() as u64)
        {
            s.used = self.tick;
            let a = (off - s.start) as usize;
            buf.copy_from_slice(&s.data[a..a + buf.len()]);
            return Ok(());
        }
        let n = self.size.min(self.dim.saturating_sub(off) as usize);
        if buf.len() >= self.size || n < buf.len() {
            // big (or past the end, for the fetch to refuse): as asked
            return (self.fetch)(off, buf);
        }
        let idx = if self.slots.len() < self.max_slots {
            self.slots.push(Slot {
                start: 0,
                data: Vec::new(),
                used: 0,
            });
            self.slots.len() - 1
        } else {
            (0..self.slots.len())
                .min_by_key(|&i| self.slots[i].used)
                .unwrap_or(0)
        };
        let s = &mut self.slots[idx];
        s.data.resize(n, 0);
        // a failed fetch must not leave a block that claims bytes it does not hold
        s.start = u64::MAX;
        s.used = 0;
        s.data.fill(0);
        let r = (self.fetch)(off, &mut s.data);
        if r.is_ok() {
            s.start = off;
            s.used = self.tick;
            buf.copy_from_slice(&s.data[..buf.len()]);
        }
        r
    }
}

#[cfg(test)]
mod tests;
