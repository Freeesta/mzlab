//! Reads of the scan table: blocks of the source (a block given back is filled again, not zeroed) and the search of a tag in them.

use crate::error::{Error, Result};
use crate::idxread::ReadAt;

/// Each spectrum's `[start, end)` and header text. Offsets come from the `indexList` when the file has a valid one, else the file is walked.
pub(super) struct Source<'a, R: ReadAt> {
    pub(super) r: &'a mut R,
    pub(super) size: u64,
    /// A block given back (`give`): the next `read` fills it instead of zeroing a new one.
    pub(super) spare: Vec<u8>,
}

impl<R: ReadAt> Source<'_, R> {
    pub(super) fn read(&mut self, a: u64, b: u64) -> Result<Vec<u8>> {
        let n = b.min(self.size).saturating_sub(a) as usize;
        let mut v = std::mem::take(&mut self.spare);
        if v.len() >= n {
            v.truncate(n);
        } else {
            v.try_reserve_exact(n - v.len())
                .map_err(|_| Error::memory_budget())?;
            v.resize(n, 0);
        }
        self.r.read_at(a, &mut v)?;
        Ok(v)
    }

    /// Keeps a block that is no longer needed for the next `read`.
    pub(super) fn give(&mut self, v: Vec<u8>) {
        if v.len() > self.spare.len() {
            self.spare = v;
        }
    }
}

/// Where `pat` first is in `h`.
pub(super) fn find(h: &[u8], pat: &[u8]) -> Option<usize> {
    let (&first, rest) = pat.split_first()?;
    let last = h.len().checked_sub(pat.len())?;
    let mut i = 0;
    while i <= last {
        i += first_byte(h.get(i..=last)?, first)?;
        if h.get(i + 1..i + pat.len()) == Some(rest) {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Where `b` first is in `h`. The walk of a file crosses whole arrays of base64 (where `<` never is) looking for a tag: blocks of 32 bytes
/// compared with no early exit, so the compiler can use vector compares (SSE/AVX, wasm simd128).
fn first_byte(h: &[u8], b: u8) -> Option<usize> {
    let (blocks, tail) = h.as_chunks::<32>();
    for (k, blk) in blocks.iter().enumerate() {
        if blk.iter().fold(0u8, |acc, &c| acc | u8::from(c == b)) != 0 {
            return blk.iter().position(|&c| c == b).map(|p| k * 32 + p);
        }
    }
    tail.iter()
        .position(|&c| c == b)
        .map(|p| blocks.len() * 32 + p)
}
