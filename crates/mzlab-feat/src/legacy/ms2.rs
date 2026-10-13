//! The MS2 of a row as mzFinder pools it (`TP_Mine/py/tpmine/hr/spectra.py`): the DDA scans whose isolation centre is within 10 ppm of the m/z and
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
