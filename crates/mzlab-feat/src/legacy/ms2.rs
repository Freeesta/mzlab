//! The MS2 of a row as mzFinder pools it: the DDA scans whose isolation centre is within 10 ppm of the m/z and
//! whose time lies inside the peak in that file (`select_ms2`), their peaks (read from the MS2 section of the `.mzidx`) and the consensus spectrum
//! (`consensus_arrays`, `ms2_consensus`).

use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

/// One MS2 scan of the series: file position, scan index (into `Index::scans`), time in minutes.
pub type Pick = (usize, usize, f64);

/// The level-2 scans of one file that have an isolation window (`index_ms2`): scan index, time, centre of the window.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ms2Index {
    pub scan: Vec<usize>,
    pub rt: Vec<f64>,
    pub center: Vec<f64>,
}

impl Ms2Index {
    pub fn of<R: ReadAt>(ix: &Index<R>) -> Ms2Index {
        let mut out = Ms2Index::default();
        for (i, s) in ix.scans.iter().enumerate() {
            if let (2, Some((a, b))) = (s.level, s.iso) {
                out.scan.push(i);
                out.rt.push(s.rt);
                out.center.push((a + b) / 2.0);
            }
        }
        out
    }
}

/// `Ms2Index` with the scans also ordered by centre of the window: `select_file` finds the scans of a row without going through all of them.
#[derive(Debug, Clone, Default)]
pub struct Ms2Sorted {
    index: Ms2Index,
    /// Positions in `index`, by centre of the window.
    by_center: Vec<u32>,
}

impl Ms2Sorted {
    pub fn of<R: ReadAt>(ix: &Index<R>) -> Ms2Sorted {
        let index = Ms2Index::of(ix);
        let mut by_center: Vec<u32> = (0..index.scan.len() as u32).collect();
        mzlab_core::sortc::by(&mut by_center, |&a, &b| {
            index.center[a as usize].total_cmp(&index.center[b as usize])
        });
        Ms2Sorted { index, by_center }
    }

    /// What `select` gives for this one file `k` (same scans, same order, same bits): the window of centres is found by bisection and the exact test of `select` is applied to what lies in it.
    pub fn select_file(&self, k: usize, mz: f64, limits: (f64, f64), ppm: f64) -> Vec<Pick> {
        let (lo, hi) = limits;
        if !(lo.is_finite() && hi.is_finite()) {
            return vec![];
        }
        let ix = &self.index;
        let tol = mz * ppm * 1e-6;
        // a margin far above the rounding of the test, so the bisection never drops a scan the test would keep
        let wide = tol * (1.0 + 1e-9) + mz.abs() * 1e-12;
        let a = self
            .by_center
            .partition_point(|&i| ix.center[i as usize] < mz - wide);
        let b = self
            .by_center
            .partition_point(|&i| ix.center[i as usize] <= mz + wide);
        let mut hit: Vec<u32> = self.by_center[a..b]
            .iter()
            .copied()
            .filter(|&i| {
                let i = i as usize;
                (ix.center[i] - mz).abs() <= tol && ix.rt[i] >= lo && ix.rt[i] <= hi
            })
            .collect();
        mzlab_core::sortc::ord(&mut hit);
        hit.into_iter()
            .map(|i| (k, ix.scan[i as usize], ix.rt[i as usize]))
            .collect()
    }
}

/// `select_ms2(indexes, mz, limits, ppm)`: limits = (left, right) of the peak in every file, NaN where the row has no peak there.
pub fn select(indexes: &[Ms2Index], mz: f64, limits: &[(f64, f64)], ppm: f64) -> Vec<Pick> {
    let mut out = vec![];
    for (k, ix) in indexes.iter().enumerate() {
        let (lo, hi) = limits[k];
        if !(lo.is_finite() && hi.is_finite()) {
            continue;
        }
        for i in 0..ix.scan.len() {
            if (ix.center[i] - mz).abs() <= mz * ppm * 1e-6 && ix.rt[i] >= lo && ix.rt[i] <= hi {
                out.push((k, ix.scan[i], ix.rt[i]));
            }
        }
    }
    out
}

/// The peaks of the scans `wanted` (indices into `Index::scans`) of one file, in the order asked: one pass over the MS2 sections (both polarities).
pub fn read_scans<R: ReadAt>(
    ix: &mut Index<R>,
    wanted: &[usize],
) -> Result<Vec<(Vec<f64>, Vec<f64>)>> {
    let mut out: Vec<(Vec<f64>, Vec<f64>)> = wanted.iter().map(|_| (vec![], vec![])).collect();
    let sections: Vec<usize> = (0..ix.sections.len())
        .filter(|&s| ix.sections[s].level == 2)
        .collect();
    let at: std::collections::HashMap<usize, usize> =
        wanted.iter().enumerate().map(|(k, &w)| (w, k)).collect();
    for si in sections {
        let ids = ix.arrays(si)?.scan_ids.clone();
        let slot: Vec<Option<usize>> = ids
            .iter()
            .map(|&g| usize::try_from(g).ok().and_then(|g| at.get(&g).copied()))
            .collect();
        if slot.iter().all(Option::is_none) {
            continue;
        }
        let n = ix.sections[si].n_peaks;
        ix.each_peak(si, 0, n, |p, mz, it| {
            if let Some(Some(k)) = slot.get(p) {
                out[*k].0.push(mz);
                out[*k].1.push(it);
            }
        })?;
    }
    Ok(out)
}

/// `consensus_arrays(mzs, ints, ppm, min_frac, min_rel)`: moved to `mzlab_core::msnalbero::consenso` (the MSn tree uses it too).
pub use mzlab_core::msnalbero::consenso as consensus_arrays;

/// `ms2_consensus`: peaks in at least 40 % of the scans and at least 0.5 % of the base peak, relative to the strongest (= 100).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Consensus {
    pub mz: Vec<f64>,
    pub rel: Vec<f64>,
    pub n_found: Vec<usize>,
    pub n_scans: usize,
}

pub fn consensus(spectra: &[(Vec<f64>, Vec<f64>)]) -> Consensus {
    let (mz, mut rel, n_found) = consensus_arrays(spectra, 8.0, 0.4, 0.005);
    let mx = rel.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !rel.is_empty() {
        rel.iter_mut().for_each(|r| *r = 100.0 * *r / mx);
    }
    Consensus {
        mz,
        rel,
        n_found,
        n_scans: spectra.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `select_file` gives what `select` gives for the same file, on windows at the edge of the tolerance, ties and unsorted centres.
    #[test]
    fn select_file_is_select_on_one_file() {
        let mut m = Ms2Index::default();
        let mut x = 12345u64;
        for i in 0..400 {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let c = 200.0 + ((x >> 33) % 3000) as f64 * 0.01 + if i % 7 == 0 { 0.0 } else { 0.001 };
            m.scan.push(i * 3);
            m.rt.push(i as f64 * 0.05);
            m.center.push(c);
        }
        let mut by_center: Vec<u32> = (0..m.scan.len() as u32).collect();
        mzlab_core::sortc::by(&mut by_center, |&a, &b| {
            m.center[a as usize].total_cmp(&m.center[b as usize])
        });
        let sorted = Ms2Sorted {
            index: m.clone(),
            by_center,
        };
        for &c in m.center.iter().take(60) {
            for ppm in [0.0, 5.0, 10.0, 500.0] {
                for lim in [(0.0, 30.0), (4.0, 9.0), (f64::NAN, 1.0)] {
                    let want = select(std::slice::from_ref(&m), c, &[lim], ppm);
                    assert_eq!(sorted.select_file(0, c, lim, ppm), want);
                }
            }
        }
    }
}
