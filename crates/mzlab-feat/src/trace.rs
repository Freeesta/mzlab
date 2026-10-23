//! Ion traces of the MS1 peaks of one file and polarity: the `detect` of mzFinder.
//!
//! Log-ppm channels on two shifted grids; (channel, scan) cells; a segment is a run of scans in one channel with gaps <= `max_gap`.
//! Segments of neighbouring channels that cover the same scans are stitched, the two grids are merged and the Fourier-transform side
//! lobes of very intense ions are marked. The peaks come from the index in slabs of m/z (a slab ends only at a gap of 50 ppm or more
//! between two peaks above the floor, where no trace can pass), so the memory is bounded and the result does not depend on the slabs.
//! Sums run in a fixed order and `ln` comes from `libm`: native and WASM give the same bits.

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};

use crate::axis::MassAxis;
use crate::centwave::{self, Peak};
use crate::channels::{by_scan, segments, stitch};
use crate::post::{finish, median_interval, Row};
use crate::table::FileFeatures;

pub const CHANNEL_PPM: f64 = 4e-6;
pub const FLOOR: f64 = 2e4;
pub const MIN_HEIGHT: f64 = 2e5;
pub const MIN_POINTS: u32 = 5;
pub const LONG_TRACE: u32 = 400;
pub const MAX_GAP: u32 = 2;
pub const SLAB_PEAKS: usize = 1_500_000;
pub const TOUCH_HEIGHT: f64 = 2e6;
/// A slab never ends at a smaller gap (ppm) between neighbouring peaks.
pub const SLAB_GAP_PPM: f64 = 50.0;
/// Bytes of working memory per peak of a slab (m/z, intensity, scan, log m/z, channel, ...).
const BYTES_PER_PEAK: usize = 64;

#[derive(Clone)]
pub struct Params {
    /// Polarity of the MS1 scans (1 or -1); `None` = all the MS1 scans.
    pub polarity: Option<i8>,
    pub floor: f64,
    pub min_points: u32,
    pub min_height: f64,
    pub max_gap: u32,
    pub long_trace: u32,
    /// Two rows of different grids are one ion when their scans overlap by this share of the shorter one (0 = only the apex and edges test of `merge_grids`).
    pub merge_overlap: f64,
    /// Axis of the channels of m/z (`Da` = unit resolution).
    pub axis: MassAxis,
    /// Working memory of the detection, bytes: it sets the size of the slabs (at most `SLAB_PEAKS` peaks).
    pub budget: usize,
    /// How the traces become peaks; only the wavelet model shows in the text of the recipe.
    pub peak: Peak,
}

// The text of the recipe is built from this: the default peak model adds nothing, so the fingerprints of the files made before it exist keep their name.
impl std::fmt::Debug for Params {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("Params");
        d.field("polarity", &self.polarity)
            .field("floor", &self.floor)
            .field("min_points", &self.min_points)
            .field("min_height", &self.min_height)
            .field("max_gap", &self.max_gap)
            .field("long_trace", &self.long_trace)
            .field("axis", &self.axis)
            .field("budget", &self.budget);
        if self.merge_overlap != 0.0 {
            d.field("merge_overlap", &self.merge_overlap);
        }
        if self.peak != Peak::Minima {
            d.field("peak", &self.peak);
        }
        d.finish()
    }
}

impl Default for Params {
    fn default() -> Self {
        Params {
            polarity: Some(1),
            floor: FLOOR,
            min_points: MIN_POINTS,
            min_height: MIN_HEIGHT,
            max_gap: MAX_GAP,
            long_trace: LONG_TRACE,
            merge_overlap: 0.0,
            axis: MassAxis::Ppm,
            budget: SLAB_PEAKS * BYTES_PER_PEAK,
            peak: Peak::Minima,
        }
    }
}

impl Params {
    fn slab_peaks(&self) -> usize {
        (self.budget / BYTES_PER_PEAK).clamp(1024, SLAB_PEAKS)
    }
}
pub(crate) fn reserve<T>(v: &mut Vec<T>, extra: usize) -> Result<()> {
    v.try_reserve(extra).map_err(|_| Error::memory_budget())
}

/// `vec![value; n]` that refuses with `error.memoryBudget` instead of aborting when the allocation fails.
pub(crate) fn filled<T: Clone>(n: usize, value: T) -> Result<Vec<T>> {
    let mut v = Vec::new();
    reserve(&mut v, n)?;
    v.resize(n, value);
    Ok(v)
}

/// One slab of peaks (m/z ascending, above the floor) through both grids.
struct Slab {
    mz: Vec<f64>,
    it: Vec<f64>,
    pos: Vec<u32>,
}

fn run_slab(slab: &Slab, rt: &[f64], dt: f64, p: &Params, rows: &mut [Vec<Row>; 2]) -> Result<()> {
    if slab.mz.is_empty() {
        return Ok(());
    }
    let mut lmz: Vec<f64> = Vec::new();
    reserve(&mut lmz, slab.mz.len())?;
    let den = libm::log1p(CHANNEL_PPM);
    lmz.extend(slab.mz.iter().map(|&m| p.axis.coord(m, den)));
    let scan_order = by_scan(&slab.pos)?;
    for (g, shift) in [0.0f64, 0.5].into_iter().enumerate() {
        let mut segs = Vec::new();
        segments(
            &lmz,
            &slab.mz,
            &slab.it,
            &slab.pos,
            &scan_order,
            shift,
            p,
            &mut segs,
        )?;
        let mut segs = stitch(segs, p)?;
        if let Peak::CentWave(cw) = &p.peak {
            segs = centwave::refine(segs, &slab.mz, &slab.it, &slab.pos, dt, p, cw)?;
        }
        for s in segs {
            if s.n >= p.min_points && s.mx >= p.min_height {
                reserve(&mut rows[g], 1)?;
                rows[g].push(Row {
                    mz: s.mzc,
                    rt: rt[s.apex as usize],
                    n: s.n,
                    height: s.mx,
                    area: s.tot * dt,
                    apex: s.apex,
                    lo: s.a,
                    hi: s.z,
                    left: rt[s.a as usize],
                    right: rt[s.z as usize],
                    long: s.z - s.a + 1 > p.long_trace,
                    grid: g as u8,
                });
            }
        }
    }
    Ok(())
}

/// Ion traces of the MS1 scans of an index (one polarity, or all).
pub fn detect<R: ReadAt>(index: &mut Index<R>, p: &Params) -> Result<FileFeatures> {
    detect_with(index, p, &mut |_, _| {})
}

/// `detect` telling `progress(done, total)` how many peaks it has gone through; the traces are the same with any `progress`.
pub fn detect_with<R: ReadAt>(
    index: &mut Index<R>,
    p: &Params,
    progress: &mut dyn FnMut(usize, usize),
) -> Result<FileFeatures> {
    let Some(si) = index.section_of(1, p.polarity) else {
        return Ok(FileFeatures::default());
    };
    let rt: Vec<f64> = index.arrays(si)?.rt.clone();
    let dt = median_interval(&rt);
    let nscan = rt.len();
    let n_peaks = index.sections[si].n_peaks;
    let max_slab = p.slab_peaks();
    let mut rows: [Vec<Row>; 2] = [Vec::new(), Vec::new()];
    let mut slab = Slab {
        mz: Vec::new(),
        it: Vec::new(),
        pos: Vec::new(),
    };
    const STEP: u64 = 1 << 16;
    let mut a = 0u64;
    let mut chunk: Vec<(u32, f64, f64)> = Vec::new();
    while a < n_peaks {
        let b = (a + STEP).min(n_peaks);
        chunk.clear();
        index.each_peak(si, a, b, |pos, mz, it| {
            if it >= p.floor {
                chunk.push((pos as u32, mz, it));
            }
        })?;
        for &(pos, mz, it) in &chunk {
            if (pos as usize) >= nscan {
                continue;
            }
            if slab.mz.len() >= max_slab {
                let last = slab.mz[slab.mz.len() - 1];
                if (mz - last) / mz * 1e6 >= SLAB_GAP_PPM {
                    run_slab(&slab, &rt, dt, p, &mut rows)?;
                    slab.mz.clear();
                    slab.it.clear();
                    slab.pos.clear();
                }
            }
            reserve(&mut slab.mz, 1)?;
            reserve(&mut slab.it, 1)?;
            reserve(&mut slab.pos, 1)?;
            slab.mz.push(mz);
            slab.it.push(it);
            slab.pos.push(pos);
        }
        a = b;
        progress(a as usize, n_peaks as usize);
    }
    run_slab(&slab, &rt, dt, p, &mut rows)?;
    drop(slab);

    finish(rows, dt, p.axis, p.merge_overlap)
}

#[cfg(test)]
mod tests {
    use super::filled;

    #[test]
    fn filled_gives_the_vec_or_refuses_with_the_key() {
        assert_eq!(filled(3, 7u8).unwrap(), vec![7, 7, 7]);
        assert!(filled::<f64>(0, 0.0).unwrap().is_empty());
        let e = filled::<u64>(usize::MAX / 16, 0).unwrap_err();
        assert_eq!(e.key, "error.memoryBudget");
    }
}
