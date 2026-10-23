//! Walk through the peak table of a section once, handing every feature the peaks in its window of m/z.
//!
//! The ranges are sorted by their lower bound, like the peaks of the table, so a slab of at most `max_peaks` peaks serves every range
//! that lies inside it; the result for a feature does not depend on the size of the slabs.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::trace::{filled, reserve};
use std::cell::{OnceCell, RefCell};

/// Peaks of a slab: m/z, intensity and position of the scan (in the section), in table order.
#[derive(Default)]
pub(crate) struct Slab {
    pub(crate) mz: Vec<f64>,
    pub(crate) it: Vec<f64>,
    pub(crate) pos: Vec<u32>,
    by_scan: OnceCell<ByScan>,
}

/// The peaks of a slab grouped by scan, each scan in table order: `orig` is the table position, so the peaks of a window `a..b` of a scan are
/// found by bisection instead of by walking the whole window of the table.
struct ByScan {
    start: Vec<u32>,
    orig: Vec<u32>,
    it: Vec<f64>,
    /// Per scan, where the last window began: windows come in ascending order of m/z, so the next one starts at or after it.
    cur: RefCell<Vec<u32>>,
}

impl ByScan {
    fn new(pos: &[u32], it: &[f64]) -> Result<ByScan> {
        let nscan = pos.iter().max().map_or(0, |&m| m as usize + 1);
        let mut start = filled(nscan + 1, 0u32)?;
        for &s in pos {
            start[s as usize + 1] += 1;
        }
        for k in 0..nscan {
            start[k + 1] += start[k];
        }
        let mut next = start.clone();
        let mut orig = filled(pos.len(), 0u32)?;
        let mut y = filled(pos.len(), 0.0f64)?;
        for (i, &s) in pos.iter().enumerate() {
            let at = next[s as usize] as usize;
            orig[at] = i as u32;
            y[at] = it[i];
            next[s as usize] += 1;
        }
        let cur = RefCell::new(start[..nscan].to_vec());
        Ok(ByScan {
            start,
            orig,
            it: y,
            cur,
        })
    }
}

/// The bisection path of `Slab::add_trace`: per scan, the peaks of the table positions `a..b`, found from the cursor of the last window.
fn bisect_into(g: &ByScan, a: usize, b: usize, w0: usize, y: &mut [f64]) {
    let w1 = w0 + y.len();
    let mut cur = g.cur.borrow_mut();
    for s in w0..w1.min(cur.len()) {
        let (from, to) = (g.start[s] as usize, g.start[s + 1] as usize);
        let below = |o: &u32| (*o as usize) < a;
        let mut k = cur[s] as usize;
        if k > from && g.orig[k - 1] as usize >= a {
            // a window that starts before the last one: look again from the start of the scan
            k = from + g.orig[from..to].partition_point(below);
        } else {
            let mut steps = 0;
            while k < to && below(&g.orig[k]) {
                k += 1;
                steps += 1;
                if steps == 16 {
                    k += g.orig[k..to].partition_point(below);
                    break;
                }
            }
        }
        cur[s] = k as u32;
        while k < to && (g.orig[k] as usize) < b {
            y[s - w0] += g.it[k];
            k += 1;
        }
    }
}

/// The walking path of `Slab::add_trace`.
fn walk_into(pos: &[u32], it: &[f64], w0: usize, y: &mut [f64]) {
    let len = y.len();
    for (&s, &v) in pos.iter().zip(it) {
        let d = (s as usize).wrapping_sub(w0);
        if d < len {
            y[d] += v;
        }
    }
}

/// Looking up the window in one scan costs about this many peaks of the plain walk.
const BISECT_COST: usize = 4;

impl Slab {
    fn load<R: ReadAt>(&mut self, index: &mut Index<R>, si: usize, a: u64, b: u64) -> Result<()> {
        self.mz.clear();
        self.it.clear();
        self.pos.clear();
        self.by_scan = OnceCell::new();
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

impl Slab {
    /// Adds to `y[s - w0]` the intensity of the peaks of the table positions `a..b` that lie in scan `s`, for the scans `w0..w0 + y.len()`, in table
    /// order: the sums are the ones of walking `a..b`, found by the shorter of the two ways.
    pub(crate) fn add_trace(&self, a: usize, b: usize, w0: usize, y: &mut [f64]) -> Result<()> {
        if y.len().saturating_mul(BISECT_COST) >= b - a {
            walk_into(&self.pos[a..b], &self.it[a..b], w0, y);
            return Ok(());
        }
        if self.by_scan.get().is_none() {
            let built = ByScan::new(&self.pos, &self.it)?;
            let _ = self.by_scan.set(built);
        }
        let Some(g) = self.by_scan.get() else {
            return Ok(());
        };
        bisect_into(g, a, b, w0, y);
        Ok(())
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
