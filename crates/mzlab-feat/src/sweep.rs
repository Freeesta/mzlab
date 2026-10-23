//! Walk through the peak table of a section once, handing every feature the peaks in its window of m/z.
//!
//! The ranges are sorted by their lower bound, like the peaks of the table, so a slab of at most `max_peaks` peaks serves every range
//! that lies inside it; the result for a feature does not depend on the size of the slabs.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::trace::{filled, reserve};
use std::sync::{Mutex, OnceLock};

/// Peaks of a slab: m/z, intensity and position of the scan (in the section), in table order.
#[derive(Default)]
pub(crate) struct Slab {
    pub(crate) mz: Vec<f64>,
    pub(crate) it: Vec<f64>,
    pub(crate) pos: Vec<u32>,
    by_scan: OnceLock<ByScan>,
    /// Cursors of `add_trace` (the single-thread path); parallel callers keep their own (`add_trace_cur`).
    cur: Mutex<Vec<u32>>,
}

/// The peaks of a slab grouped by scan, each scan in table order: `orig` is the table position, so the peaks of a window `a..b` of a scan are
/// found by bisection instead of by walking the whole window of the table.
struct ByScan {
    start: Vec<u32>,
    orig: Vec<u32>,
    it: Vec<f64>,
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
        Ok(ByScan { start, orig, it: y })
    }
}

/// The bisection path of `Slab::add_trace`: per scan, the peaks of the table positions `a..b`, found from the cursor `cur` (per scan, where
/// the last window began: windows come in ascending order of m/z, so the next one starts at or after it).
fn bisect_into(g: &ByScan, cur: &mut [u32], a: usize, b: usize, w0: usize, y: &mut [f64]) {
    let w1 = w0 + y.len();
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

/// Looking up the window in one scan costs about this many peaks of the plain walk.
const BISECT_COST: usize = 4;

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

impl Slab {
    fn load<R: ReadAt>(&mut self, index: &mut Index<R>, si: usize, a: u64, b: u64) -> Result<()> {
        self.mz.clear();
        self.it.clear();
        self.pos.clear();
        self.by_scan = OnceLock::new();
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

impl Slab {
    /// The peaks of the slab by scan, built on first use (callers that share the slab between threads call `prepare` first).
    fn scans(&self) -> Result<&ByScan> {
        if let Some(g) = self.by_scan.get() {
            return Ok(g);
        }
        let built = ByScan::new(&self.pos, &self.it)?;
        let mut cur = self.cur.lock().unwrap_or_else(|e| e.into_inner());
        cur.clear();
        cur.extend_from_slice(&built.start[..built.start.len() - 1]);
        Ok(self.by_scan.get_or_init(|| built))
    }

    /// Adds to `y[s - w0]` the intensity of the peaks of the table positions `a..b` that lie in scan `s`, for the scans `w0..w0 + y.len()`, in table
    /// order: the sums are the ones of walking `a..b`, found by the shorter of the two ways.
    pub(crate) fn add_trace(&self, a: usize, b: usize, w0: usize, y: &mut [f64]) -> Result<()> {
        if y.len().saturating_mul(BISECT_COST) >= b - a {
            walk_into(&self.pos[a..b], &self.it[a..b], w0, y);
            return Ok(());
        }
        let g = self.scans()?;
        let mut cur = self.cur.lock().unwrap_or_else(|e| e.into_inner());
        bisect_into(g, &mut cur, a, b, w0, y);
        Ok(())
    }

    /// Builds the by-scan view now, so that `add_trace_cur` can run on several threads.
    pub(crate) fn prepare(&self) -> Result<()> {
        self.scans().map(|_| ())
    }

    /// A fresh cursor for `add_trace_cur` (empty until the slab is prepared).
    pub(crate) fn cursor(&self) -> Vec<u32> {
        self.by_scan
            .get()
            .map_or_else(Vec::new, |g| g.start[..g.start.len() - 1].to_vec())
    }

    /// `add_trace` for a thread with its own `cur` from `cursor`: the same sums; the bisection path needs a prepared slab.
    pub(crate) fn add_trace_cur(
        &self,
        cur: &mut [u32],
        a: usize,
        b: usize,
        w0: usize,
        y: &mut [f64],
    ) {
        match self.by_scan.get() {
            Some(g) if y.len().saturating_mul(BISECT_COST) < b - a => {
                bisect_into(g, cur, a, b, w0, y)
            }
            _ => walk_into(&self.pos[a..b], &self.it[a..b], w0, y),
        }
    }

    /// Whether the trace of the scans `w0..w1` over the table positions `a..b` is served by bisection.
    pub(crate) fn bisects(len: usize, a: usize, b: usize) -> bool {
        len.saturating_mul(BISECT_COST) < b - a
    }
}

/// `f(i, lo..hi, slab)` for every range `i` (ascending by lower bound): the peaks `lo..hi` of the slab lie inside `ranges[i]`.
pub(crate) fn windows<R: ReadAt>(
    index: &mut Index<R>,
    si: usize,
    ranges: &[(f64, f64)],
    max_peaks: usize,
    mut f: impl FnMut(usize, usize, usize, &Slab) -> Result<()>,
) -> Result<()> {
    windows_by_slab(index, si, ranges, max_peaks, |slab, hits| {
        hits.iter().try_for_each(|&(j, x, y)| f(j, x, y, slab))
    })
}

/// `f(slab, hits)` for every slab, with `hits` = `(i, lo, hi)` of the ranges the slab serves, in order: the same windows as `windows`,
/// handed over together so that the caller can work on them in parallel.
pub(crate) fn windows_by_slab<R: ReadAt>(
    index: &mut Index<R>,
    si: usize,
    ranges: &[(f64, f64)],
    max_peaks: usize,
    mut f: impl FnMut(&Slab, &[(usize, usize, usize)]) -> Result<()>,
) -> Result<()> {
    let n_peaks = index.sections[si].n_peaks;
    // a high-resolution survey stores f32: the bounds are compared in f32, as the index search does
    let f4 = index.sections[si].f4;
    let cast = |v: f64| if f4 { v as f32 as f64 } else { v };
    let mut slab = Slab::default();
    let mut hits: Vec<(usize, usize, usize)> = Vec::new();
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
        hits.clear();
        while j < ranges.len() && (j == i || !more || ranges[j].1 < last) {
            let (lo, hi) = (cast(ranges[j].0), cast(ranges[j].1));
            let x = slab.mz.partition_point(|&m| m < lo);
            let y = slab.mz.partition_point(|&m| m <= hi);
            hits.push((j, x, y));
            j += 1;
        }
        f(&slab, &hits)?;
        i = j;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_cursor_gives_the_sums_of_the_shared_one() {
        let mut slab = Slab::default();
        for s in 0..40u32 {
            for k in 0..30u32 {
                slab.mz.push(100.0 + k as f64);
                slab.it.push((s * 31 + k) as f64);
                slab.pos.push(s);
            }
        }
        // table order is by m/z: sort the 1200 peaks the way the index stores them
        let mut ord: Vec<usize> = (0..slab.mz.len()).collect();
        ord.sort_by(|&a, &b| slab.mz[a].total_cmp(&slab.mz[b]).then(a.cmp(&b)));
        slab.it = ord.iter().map(|&i| slab.it[i]).collect();
        slab.pos = ord.iter().map(|&i| slab.pos[i]).collect();
        slab.mz = ord.iter().map(|&i| slab.mz[i]).collect();
        slab.prepare().unwrap();
        let mut cur = slab.cursor();
        for (a, b) in [(0, 400), (390, 800), (700, 1200), (10, 80)] {
            let (mut y1, mut y2) = (vec![0.0; 12], vec![0.0; 12]);
            slab.add_trace(a, b, 5, &mut y1).unwrap();
            slab.add_trace_cur(&mut cur, a, b, 5, &mut y2);
            assert_eq!(y1, y2);
        }
    }
}
