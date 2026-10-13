//! Ion traces of the rows that take part in a candidate pair, and their shape correlation scan by scan.
//!
//! The trace of a row is the sum of the peaks within `ppm` of its m/z in every scan of a window around its apex (`half` scans on each side,
//! at least the FWHM and three more). Two traces are compared on the scans from `fw` before the earlier apex to `fw` after the later one
//! (`fw` = the larger FWHM, at least 2 scans), inside both windows: Pearson correlation, no smoothing, no baseline.

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::sweep::{ppm_ranges, windows};
use crate::table::FileFeatures;
use crate::trace::{filled, Params};

const NONE: u32 = u32::MAX;
/// Fewest scans a correlation is computed on.
pub const MIN_POINTS: usize = 5;

#[derive(Debug, Clone, Default)]
pub struct Traces {
    slot: Vec<u32>,
    first: Vec<u32>,
    start: Vec<u32>,
    y: Vec<f64>,
}

/// Scans of the window of a row: `(first, last)` positions in the MS1 section.
pub fn window_of(f: &FileFeatures, row: usize, nscan: usize) -> (usize, usize) {
    let half = ((f.fwhm[row].ceil() as usize) + 3).clamp(5, 40);
    let apex = (f.apex[row] as usize).min(nscan.saturating_sub(1));
    (
        apex.saturating_sub(half),
        (apex + half).min(nscan.saturating_sub(1)),
    )
}

impl Traces {
    /// Traces given by hand (tests): `(row, first scan, values)`.
    pub fn from_parts(n_rows: usize, parts: &[(usize, u32, Vec<f64>)]) -> Self {
        let mut t = Traces {
            slot: vec![NONE; n_rows],
            // fixed size: the first offset of the trace index
            start: vec![0],
            ..Default::default()
        };
        for (row, first, y) in parts {
            t.slot[*row] = t.first.len() as u32;
            t.first.push(*first);
            t.y.extend_from_slice(y);
            t.start.push(t.y.len() as u32);
        }
        t
    }

    fn get(&self, row: usize) -> Option<(usize, &[f64])> {
        let s = *self.slot.get(row)?;
        (s != NONE).then(|| {
            let s = s as usize;
            (
                self.first[s] as usize,
                &self.y[self.start[s] as usize..self.start[s + 1] as usize],
            )
        })
    }

    /// Pearson correlation of the traces of two rows on the scans described in the module header; None when fewer than `MIN_POINTS` scans
    /// are shared or one trace is flat there.
    pub fn correlation(&self, f: &FileFeatures, a: usize, b: usize) -> Option<f64> {
        let ((fa, ya), (fb, yb)) = (self.get(a)?, self.get(b)?);
        let fw = (f.fwhm[a].max(f.fwhm[b]).ceil() as i64).max(2);
        let (pa, pb) = (f.apex[a] as i64, f.apex[b] as i64);
        let lo = (pa.min(pb) - fw).max(fa as i64).max(fb as i64);
        let hi = (pa.max(pb) + fw)
            .min((fa + ya.len()) as i64 - 1)
            .min((fb + yb.len()) as i64 - 1);
        if hi - lo + 1 < MIN_POINTS as i64 {
            return None;
        }
        let xs = &ya[lo as usize - fa..=hi as usize - fa];
        let ys = &yb[lo as usize - fb..=hi as usize - fb];
        pearson(xs, ys)
    }
}

pub(crate) fn pearson(x: &[f64], y: &[f64]) -> Option<f64> {
    let n = x.len() as f64;
    let (mx, my) = (x.iter().sum::<f64>() / n, y.iter().sum::<f64>() / n);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (a, b) in x.iter().zip(y) {
        let (da, db) = (a - mx, b - my);
        sxy += da * db;
        sxx += da * da;
        syy += db * db;
    }
    (sxx > 0.0 && syy > 0.0).then(|| sxy / (sxx * syy).sqrt())
}

/// Traces of `rows` (ascending, so their ranges of m/z ascend too) from the peak table of the MS1 section.
pub fn gather<R: ReadAt>(
    index: &mut Index<R>,
    f: &FileFeatures,
    rows: &[u32],
    p: &Params,
    ppm: f64,
) -> Result<Traces> {
    let mut t = Traces {
        slot: filled(f.len(), NONE)?,
        // fixed size: the first offset of the trace index
        start: vec![0],
        ..Default::default()
    };
    let Some(si) = index.section_of(1, p.polarity) else {
        return Ok(t);
    };
    let nscan = index.arrays(si)?.rt.len();
    let centres: Vec<f64> = rows.iter().map(|&r| f.mz[r as usize]).collect();
    let wins: Vec<(usize, usize)> = rows
        .iter()
        .map(|&r| window_of(f, r as usize, nscan))
        .collect();
    for (k, &r) in rows.iter().enumerate() {
        t.slot[r as usize] = k as u32;
        t.first.push(wins[k].0 as u32);
        t.start
            .push(t.start[k] + (wins[k].1 - wins[k].0 + 1) as u32);
    }
    t.y = filled(*t.start.last().unwrap_or(&0) as usize, 0.0)?;
    let max_peaks = (p.budget / 24).clamp(1024, crate::trace::SLAB_PEAKS);
    let ranges = ppm_ranges(&centres, ppm);
    windows(index, si, &ranges, max_peaks, |k, a, b, slab| {
        let (w0, w1) = wins[k];
        let base = t.start[k] as usize;
        for j in a..b {
            let s = slab.pos[j] as usize;
            if s >= w0 && s <= w1 {
                t.y[base + s - w0] += slab.it[j];
            }
        }
        Ok(())
    })?;
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_shapes_correlate_and_flat_does_not() {
        let g: Vec<f64> = (0..21)
            .map(|i| (-((i as f64 - 10.0).powi(2)) / 8.0).exp())
            .collect();
        let scaled: Vec<f64> = g.iter().map(|v| v * 0.1).collect();
        assert!((pearson(&g, &scaled).unwrap() - 1.0).abs() < 1e-12);
        assert!(pearson(&g, &vec![1.0; 21]).is_none());
    }
}
