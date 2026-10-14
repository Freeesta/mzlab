//! Walk through the peak table of a section once, handing every feature the peaks in its window of m/z.
//!
//! The ranges are sorted by their lower bound, like the peaks of the table, so a slab of at most `max_peaks` peaks serves every range
//! that lies inside it; the result for a feature does not depend on the size of the slabs.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::trace::reserve;

/// Peaks of a slab: m/z, intensity and position of the scan (in the section), in table order.
#[derive(Default)]
pub(crate) struct Slab {
    pub(crate) mz: Vec<f64>,
    pub(crate) it: Vec<f64>,
    pub(crate) pos: Vec<u32>,
}

impl Slab {
    fn load<R: ReadAt>(&mut self, index: &mut Index<R>, si: usize, a: u64, b: u64) -> Result<()> {
        self.mz.clear();
        self.it.clear();
        self.pos.clear();
        let n = (b - a) as usize;
        reserve(&mut self.mz, n)?;
        reserve(&mut self.it, n)?;
        reserve(&mut self.pos, n)?;
        let (mz, it, pos) = (&mut self.mz, &mut self.it, &mut self.pos);
        index.each_peak(si, a, b, |p, m, y| {
            mz.push(m);
            it.push(y);
            pos.push(p as u32);
        })
    }
}

/// Windows of `tol` (ppm, or minus Da: `axis::half`) around the centres, as ranges for `windows`.
pub(crate) fn ppm_ranges(centres: &[f64], tol: f64) -> Vec<(f64, f64)> {
    let w = tol * 1e-6;
    centres
        .iter()
        .map(|&c| {
            if tol < 0.0 {
                (c + tol, c - tol)
            } else {
                (c * (1.0 - w), c * (1.0 + w))
            }
        })
        .collect()
}

/// `f(i, lo..hi, slab)` for every range `i` (ascending by lower bound): the peaks `lo..hi` of the slab lie inside `ranges[i]`.
pub(crate) fn windows<R: ReadAt>(
    index: &mut Index<R>,
    si: usize,
    ranges: &[(f64, f64)],
    max_peaks: usize,
    mut f: impl FnMut(usize, usize, usize, &Slab) -> Result<()>,
) -> Result<()> {
    let n_peaks = index.sections[si].n_peaks;
    // a high-resolution survey stores f32: the bounds are compared in f32, as the index search does
    let f4 = index.sections[si].f4;
    let cast = |v: f64| if f4 { v as f32 as f64 } else { v };
    let mut slab = Slab::default();
    let mut i = 0;
    while i < ranges.len() {
        let a = index.search(si, ranges[i].0, false)?;
        // a slab always holds the whole window of its first range (a trace has about one peak per scan)
        let end = index.search(si, ranges[i].1, true)?;
        let b = (a + max_peaks as u64).min(n_peaks).max(end);
        slab.load(index, si, a, b)?;
        let last = slab.mz.last().copied().unwrap_or(f64::MAX);
        let more = b < n_peaks;
        let mut j = i;
        while j < ranges.len() && (j == i || !more || ranges[j].1 < last) {
            let (lo, hi) = (cast(ranges[j].0), cast(ranges[j].1));
            let x = slab.mz.partition_point(|&m| m < lo);
            let y = slab.mz.partition_point(|&m| m <= hi);
            f(j, x, y, &slab)?;
            j += 1;
        }
        i = j;
    }
    Ok(())
}
