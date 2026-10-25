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
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn file(n: usize) -> Vec<u8> {
        (0..n)
            .map(|i| (i.wrapping_mul(2654435761) >> 7) as u8)
            .collect()
    }

    /// The fetches `(offset, length)` a cache made.
    type FetchLog = Rc<RefCell<Vec<(u64, usize)>>>;

    /// A cache over `data` with the list of the fetches it made.
    fn cache(data: Vec<u8>, size: usize, slots: usize) -> (impl ReadAt, FetchLog) {
        let log: FetchLog = Rc::default();
        let l = log.clone();
        let dim = data.len() as u64;
        let fetch = move |off: u64, buf: &mut [u8]| -> Result<()> {
            l.borrow_mut().push((off, buf.len()));
            let end = off as usize + buf.len();
            if end > data.len() {
                return Err(crate::Error::new("error.badIndex"));
            }
            buf.copy_from_slice(&data[off as usize..end]);
            Ok(())
        };
        (BlockCache::new(fetch, dim, size, slots), log)
    }

    #[test]
    fn every_read_gives_the_bytes_of_the_file() {
        let data = file(100_000);
        for slots in [1, 2, 3] {
            let (mut c, _) = cache(data.clone(), 4096, slots);
            let mut state = 0x2545_f491_4f6c_dd1du64;
            for _ in 0..3000 {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let len = (state % 9000) as usize + 1;
                let off = (state >> 20) % (data.len() as u64 - len as u64);
                let mut b = vec![0u8; len];
                c.read_at(off, &mut b).unwrap();
                assert_eq!(b, data[off as usize..off as usize + len]);
            }
        }
    }

    #[test]
    fn a_read_past_the_end_is_refused_and_leaves_the_cache_sound() {
        let data = file(10_000);
        let (mut c, _) = cache(data.clone(), 4096, 2);
        let mut b = [0u8; 16];
        assert!(c.read_at(9_990, &mut b).is_err());
        c.read_at(100, &mut b).unwrap();
        assert_eq!(b, data[100..116]);
    }

    #[test]
    fn two_streams_in_turn_cost_one_block_each_with_two_slots_and_a_block_per_read_with_one() {
        let data = file(1_000_000);
        let run = |slots: usize| {
            let (mut c, log) = cache(data.clone(), 65_536, slots);
            let mut b = [0u8; 8];
            for k in 0..200u64 {
                c.read_at(k * 100, &mut b).unwrap(); // the packets, forward
                c.read_at(900_000 + k * 50, &mut b).unwrap(); // the trailer, far away, forward
            }
            let n = log.borrow().len();
            n
        };
        assert_eq!(run(1), 400);
        assert_eq!(run(2), 2);
    }
}
