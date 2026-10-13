//! The ion map (RT × m/z) of one unit-resolution file from its index: `ionmap`, `ionmap_region`, `map_stretch`, `map_background`,
//! `map_points`, `map_snap` and `map_measure` of `mzlab/explore.py`. The peaks are read in table order (sorted by m/z), so every sum is
//! added in the order numpy's `bincount` adds it. The m/z axis of a region is linear or logarithmic (cells of equal ppm, high resolution).

use std::sync::Arc;

use crate::error::Result;
use crate::idxread::ReadAt;
use crate::mapgrid::{median_cols, Grid};
use crate::routes::Routes;

impl<R: ReadAt> Routes<R> {
    /// The section of a level with its scan RTs; None when the level has no peaks to map.
    fn map_level(&mut self, level: u8) -> Result<Option<(usize, Vec<f64>)>> {
        let Some(si) = self.idx.section_of(level, self.pol) else { return Ok(None) };
        if self.idx.sections[si].n_peaks == 0 || self.idx.sections[si].n_scans == 0 {
            return Ok(None);
        }
        Ok(Some((si, self.idx.arrays(si)?.rt.clone())))
    }

    /// Per cell the maximum (as float32) or the sum of the intensities of the peaks `a..b` that `cell(scan position, m/z)` puts in a cell.
    fn accumulate(&mut self, si: usize, (a, b): (u64, u64), size: usize, mean: bool, mut cell: impl FnMut(usize, f64) -> Option<usize>) -> Result<Vec<f64>> {
        let mut acc = vec![0.0f64; size];
        self.idx.each_peak(si, a, b, |p, mz, y| {
            if let Some(c) = cell(p, mz) {
                if mean {
                    acc[c] += y;
                } else if (y as f32) as f64 > acc[c] {
                    acc[c] = (y as f32) as f64;
                }
            }
        })?;
        Ok(acc)
    }

    /// `ionmap`: the map of the level on the common grid of the session, `nrt × nmz` (the last six of a file are kept, and the engine trims all the files to its byte budget). A section in single precision
    /// puts its peaks in the cells with float32 arithmetic, as numpy does on a float32 array.
    pub(crate) fn ion_map(&mut self, level: u8, g: &Grid, mean: bool) -> Result<Arc<Vec<f32>>> {
        let key = format!("{level}|{mean}|{:?}", g);
        if let Some(i) = self.maps.iter().position(|(k, _, _)| *k == key) {
            self.maps[i].2 = super::routes::next_tick();
            return Ok(self.maps[i].1.clone());
        }
        let m = Arc::new(self.build_ion_map(level, g, mean)?);
        if self.maps.len() >= 6 {
            self.maps.remove(0);
        }
        self.maps.push((key, m.clone(), super::routes::next_tick()));
        Ok(m)
    }

    fn build_ion_map(&mut self, level: u8, g: &Grid, mean: bool) -> Result<Vec<f32>> {
        let Some((si, rt)) = self.map_level(level)? else { return Ok(vec![0.0; g.nrt * g.nmz]) };
        let bins = scan_bins(&rt, g.rt0, (g.rt1 - g.rt0).max(1e-9), g.nrt);
        let n_scans = counts(bins.iter().cloned(), g.nrt);
        let n = self.idx.sections[si].n_peaks;
        let f4 = self.idx.sections[si].f4;
        let at = |mz: f64| if f4 { ((mz as f32 - g.mz0 as f32) / g.dmz as f32) as f64 } else { (mz - g.mz0) / g.dmz };
        let acc = self.accumulate(si, (0, n), g.nrt * g.nmz, mean, |p, mz| cell_of(bins[p], at(mz), g.nmz))?;
        Ok(finish(acc, mean, &n_scans, g.nmz))
    }

    /// `ionmap_region`: a region on bins of its own (RT bins of `(rt1 - rt0) / nrt`, m/z bins of `(mz1 - mz0) / nmz`).
    pub(crate) fn region_map(&mut self, level: u8, (rt0, rt1): (f64, f64), nrt: usize, (mz0, mz1): (f64, f64), (nmz, log): (usize, bool), mean: bool) -> Result<Vec<f32>> {
        let zeros = vec![0.0f32; nrt * nmz];
        let Some((si, rt)) = self.map_level(level)? else { return Ok(zeros) };
        if rt1 <= rt0 || mz1 <= mz0 {
            return Ok(zeros);
        }
        let (a, b) = (self.idx.search(si, mz0, false)?, self.idx.search(si, mz1, false)?);
        let bins = scan_bins(&rt, rt0, rt1 - rt0, nrt);
        let inside: Vec<bool> = rt.iter().map(|&r| r >= rt0 && r < rt1).collect();
        let n_scans = counts((0..rt.len()).filter(|&p| inside[p]).map(|p| bins[p]), nrt);
        let acc = self.accumulate(si, (a, b.max(a)), nrt * nmz, mean, |p, mz| inside[p].then(|| clipped_cell(bins[p], frac(mz, mz0, mz1, log) * nmz as f64, nmz)))?;
        Ok(finish(acc, mean, &n_scans, nmz))
    }

    /// `map_stretch`: for each m/z cell of a region the mean over the scans of the stretch `[rt0, rt1]`.
    pub(crate) fn stretch_background(&mut self, level: u8, (rt0, rt1): (f64, f64), (mz0, mz1): (f64, f64), (nmz, log): (usize, bool)) -> Result<Vec<f32>> {
        if !(rt1 > rt0 && mz1 > mz0) || (log && mz0 <= 0.0) {
            return Ok(vec![0.0; nmz]);
        }
        self.region_map(level, (rt0, rt1), 1, (mz0, mz1), (nmz, log), true)
    }

    /// `map_background`: for each cell the median over the whole run of the groups of consecutive scans (at most 1500 groups).
    pub(crate) fn run_background(&mut self, level: u8, (mz0, mz1): (f64, f64), (nmz, log): (usize, bool), mean: bool) -> Result<Vec<f32>> {
        let zeros = vec![0.0f32; nmz];
        let Some((si, rt)) = self.map_level(level)? else { return Ok(zeros) };
        if !(mz1 > mz0) || (log && mz0 <= 0.0) {
            return Ok(zeros);
        }
        let ns = rt.len();
        let grp = ns.div_ceil(1500).max(1);
        let ng = ns.div_ceil(grp);
        let (a, b) = (self.idx.search(si, mz0, false)?, self.idx.search(si, mz1, false)?);
        let n_scans = counts((0..ns).map(|p| p / grp), ng);
        let acc = self.accumulate(si, (a, b.max(a)), ng * nmz, mean, |p, mz| Some(clipped_cell(p / grp, frac(mz, mz0, mz1, log) * nmz as f64, nmz)))?;
        Ok(median_cols(&finish(acc, mean, &n_scans, nmz), ng, nmz))
    }

    /// `data_extent`: RT of the first and the last scan that holds peaks, and the lowest and highest m/z; None when the level is empty.
    pub(crate) fn data_extent(&mut self, level: u8) -> Result<Option<((f64, f64), (f64, f64))>> {
        let Some((si, rt)) = self.map_level(level)? else { return Ok(None) };
        let sum = self.idx.arrays(si)?.sumint.clone();
        let on: Vec<usize> = (0..rt.len()).filter(|&p| sum[p] > 0.0).collect();
        let (Some(&first), Some(&last)) = (on.first(), on.last()) else { return Ok(None) };
        Ok(self.idx.mz_range(si)?.map(|mz| ((rt[first], rt[last]), mz)))
    }

    /// `map_snap`: the most intense peak within ±`drt` minutes and ±1 bin of `dmz` around the click (RT of its scan, its m/z).
    pub(crate) fn snap(&mut self, level: u8, rt: f64, mz: f64, dmz: f64) -> Result<Option<(f64, f64)>> {
        let Some((si, rts)) = self.map_level(level)? else { return Ok(None) };
        let f = (mz / dmz).floor();
        let (lo, hi) = ((f - 1.0) * dmz, (f + 2.0) * dmz);
        let (a, b) = (self.idx.search(si, lo, false)?, self.idx.search(si, hi, false)?);
        let mut best: Option<(f64, f64, f64)> = None;
        self.idx.each_peak(si, a, b.max(a), |p, m, y| {
            if (rts[p] - rt).abs() <= 0.2 && best.map_or(true, |s| y > s.0) {
                best = Some((y, rts[p], m));
            }
        })?;
        Ok(best.map(|s| (s.1, s.2)))
    }

    /// `Item.xic` for one level: the RT of every scan and the summed intensity within `mz ± tol` (empty when the level has no section).
    pub(crate) fn xic_arrays(&mut self, mz: f64, tol: f64, level: u8) -> Result<(Vec<f64>, Vec<f64>)> {
        let Some(si) = self.idx.section_of(level, self.pol) else { return Ok((vec![], vec![])) };
        let (a, b) = (self.idx.search(si, mz - tol, false)?, self.idx.search(si, mz + tol, true)?);
        let y = if b <= a { vec![0.0; self.idx.sections[si].n_scans] } else { self.idx.sum_window(si, a, b)? };
        Ok((self.idx.arrays(si)?.rt.clone(), y))
    }

    /// `map_points`: the points of a region, the `cap` most intense first (ties in table order).
    pub(crate) fn sticks(&mut self, level: u8, (rt0, rt1): (f64, f64), (mz0, mz1): (f64, f64), cap: usize) -> Result<Sticks> {
        let none = Sticks { rt: vec![], mz: vec![], inten: vec![], total: 0 };
        let Some((si, rts)) = self.map_level(level)? else { return Ok(none) };
        let (a, b) = (self.idx.search(si, mz0, false)?, self.idx.search(si, mz1, true)?);
        let mut pts: Vec<(f64, f64, f64)> = Vec::new();
        self.idx.each_peak(si, a, b.max(a), |p, m, y| {
            if rts[p] >= rt0 && rts[p] <= rt1 {
                pts.push((rts[p], m, y));
            }
        })?;
        let total = pts.len();
        let mut order: Vec<usize> = (0..total).collect();
        order.sort_by(|&i, &j| pts[j].2.total_cmp(&pts[i].2));
        order.truncate(cap);
        Ok(Sticks {
            rt: order.iter().map(|&i| pts[i].0 as f32).collect(),
            mz: order.iter().map(|&i| pts[i].1).collect(),
            inten: order.iter().map(|&i| pts[i].2 as f32).collect(),
            total,
        })
    }
}

/// The true points of a region for the sticks of the 3D viewer (most intense first).
pub struct Sticks {
    pub rt: Vec<f32>,
    pub mz: Vec<f64>,
    pub inten: Vec<f32>,
    pub total: usize,
}

/// Fraction of the m/z axis `[mz0, mz1]` at `mz`: linear, or by ln(m/z) for cells of equal ppm.
fn frac(mz: f64, mz0: f64, mz1: f64, log: bool) -> f64 {
    if log {
        (mz / mz0).ln() / (mz1 / mz0).ln()
    } else {
        (mz - mz0) / (mz1 - mz0)
    }
}

/// Map cell of the scan position `p` (`scan_bin[p] * nmz + j`) from the fraction `f` of the m/z axis, or None outside the axis.
fn cell_of(bin: usize, f: f64, nmz: usize) -> Option<usize> {
    let j = f.floor();
    (j >= 0.0 && j < nmz as f64).then(|| bin * nmz + j as usize)
}

/// The same with the cell clipped to the axis (a region holds only peaks inside it; a rounding at the edge must not drop one).
fn clipped_cell(bin: usize, f: f64, nmz: usize) -> usize {
    bin * nmz + f.floor().clamp(0.0, (nmz - 1) as f64) as usize
}

/// RT bin of every scan: `clip(floor((rt - rt0) / span * n), 0, n - 1)`.
fn scan_bins(rt: &[f64], rt0: f64, span: f64, n: usize) -> Vec<usize> {
    rt.iter().map(|&r| ((r - rt0) / span * n as f64).floor().clamp(0.0, (n - 1) as f64) as usize).collect()
}

fn counts(bins: impl Iterator<Item = usize>, n: usize) -> Vec<usize> {
    let mut c = vec![0; n];
    bins.for_each(|b| c[b] += 1);
    c
}

/// Cells to float32: the maximum as it is, the mean per scan of the RT bin (`n_scans[row]`, at least 1) for the sums.
fn finish(acc: Vec<f64>, mean: bool, n_scans: &[usize], nmz: usize) -> Vec<f32> {
    if mean {
        acc.iter().enumerate().map(|(i, v)| (v / n_scans[i / nmz].max(1) as f64) as f32).collect()
    } else {
        acc.iter().map(|&v| v as f32).collect()
    }
}
