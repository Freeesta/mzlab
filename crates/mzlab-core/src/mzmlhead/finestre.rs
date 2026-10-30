//! A search over the file in windows, for `Mzml::chromatograms`: the chromatograms of an MRM file are the whole file, so the list is read
//! window by window instead of in one buffer as large as the file (which also capped such files at `MAX_REGION`).

use super::tags::find_tag;
use super::{find, Mzml};
use crate::error::Result;
use crate::idxread::ReadAt;

/// Bytes read at a time.
pub(super) const WINDOW: u64 = 4 << 20;

/// The last window read and where it starts.
pub(super) struct Finestre {
    at: u64,
    buf: Vec<u8>,
}

impl Finestre {
    pub(super) fn new() -> Self {
        Finestre {
            at: 0,
            buf: Vec::new(),
        }
    }

    /// The window holds `from..` up to `from + want` (or the end of the file); otherwise a new window starts at `from`.
    fn load<R: ReadAt>(&mut self, m: &mut Mzml<R>, from: u64, want: u64) -> Result<()> {
        let end = self.at + self.buf.len() as u64;
        if from < self.at || from.saturating_add(want).min(m.size) > end {
            self.buf = m.read(from, from.saturating_add(WINDOW.max(want)))?;
            self.at = from;
        }
        Ok(())
    }

    /// The first `pat` that starts at or after `from` and ends by `to`; with `tag`, followed by white space or `>` (`find_tag`).
    pub(super) fn find<R: ReadAt>(
        &mut self,
        m: &mut Mzml<R>,
        pat: &[u8],
        mut from: u64,
        to: u64,
        tag: bool,
    ) -> Result<Option<u64>> {
        let to = to.min(m.size);
        // a match cut by the end of a window is found again in the next one, which starts this far back
        let back = pat.len() as u64 + 1;
        while from < to {
            self.load(m, from, back)?;
            let hi = (self.at + self.buf.len() as u64).min(to);
            let slice = self.buf.get(..self.off(hi)).unwrap_or_default();
            let lo = self.off(from);
            let got = if tag {
                find_tag(slice, pat, lo)
            } else {
                find(slice, pat, lo)
            };
            if let Some(p) = got {
                return Ok(Some(self.at + p as u64));
            }
            if hi >= to {
                break;
            }
            from = from.max(hi.saturating_sub(back) + 1);
        }
        Ok(None)
    }

    /// The place of file offset `x` (not before the window) in the window.
    fn off(&self, x: u64) -> usize {
        usize::try_from(x.saturating_sub(self.at)).unwrap_or(usize::MAX)
    }

    /// The bytes `a..b`, from the window when it holds them.
    pub(super) fn bytes<R: ReadAt>(&mut self, m: &mut Mzml<R>, a: u64, b: u64) -> Result<Vec<u8>> {
        let end = self.at + self.buf.len() as u64;
        if a >= self.at && b <= end {
            return Ok(self
                .buf
                .get(self.off(a)..self.off(b))
                .unwrap_or_default()
                .to_vec());
        }
        m.read(a, b)
    }
}

#[cfg(test)]
#[path = "finestre_tests.rs"]
mod tests;
