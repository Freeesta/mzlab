//! The whole peak table of one section in memory (m/z sorted, scan position, intensity), for the tools that look at thousands of m/z windows
//! of the same file: the block-wise reads of the index are made once instead of once per window. Sections that are too big stay on the index.

use crate::error::Result;
use crate::idxread::{Index, ReadAt};

/// Most peaks a section may have to be loaded whole (about 20 bytes each).
pub const MAX_PEAKS: u64 = 20_000_000;

pub struct PeakTable {
    pub mz: Vec<f64>,
    pub pos: Vec<u32>,
    pub inten: Vec<f64>,
    f4: bool,
}

impl PeakTable {
    /// `np.searchsorted(mz, v, side)`; a single-precision section compares in `f32` as the index does.
    fn search(&self, v: f64, right: bool) -> usize {
        let v = if self.f4 { v as f32 as f64 } else { v };
        if right {
            self.mz.partition_point(|&x| x <= v)
        } else {
            self.mz.partition_point(|&x| x < v)
        }
    }

    /// The peaks with `lo <= m/z <= hi` as the range `a..b` of the table.
    pub fn window(&self, lo: f64, hi: f64) -> (usize, usize) {
        (self.search(lo, false), self.search(hi, true))
    }
}

impl<R: ReadAt> Index<R> {
    /// The section's peak table, or None when it has more than `MAX_PEAKS` peaks.
    pub fn load_table(&mut self, si: usize) -> Result<Option<PeakTable>> {
        let n = self.sections[si].n_peaks;
        if n > MAX_PEAKS {
            return Ok(None);
        }
        let (mut mz, mut pos, mut inten) = (
            Vec::with_capacity(n as usize),
            Vec::with_capacity(n as usize),
            Vec::with_capacity(n as usize),
        );
        self.each_peak(si, 0, n, |p, m, y| {
            pos.push(p as u32);
            mz.push(m);
            inten.push(y);
        })?;
        Ok(Some(PeakTable {
            mz,
            pos,
            inten,
            f4: self.sections[si].f4,
        }))
    }
}
