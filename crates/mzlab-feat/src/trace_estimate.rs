//! The memory detecting one file asks for, from the header of its index and a sampled histogram of its intensities, before anything is detected:
//! what `features.detect` refuses with `error.memoryBudget` (phase `detect`) when it does not fit, on the page, in the app and in `flow run`.
//! The coefficients come from a synthetic sweep (`docs/agenti/memoria.md`), the larger of the `.wasm` and the native peak, with a margin.

use mzlab_core::budget::refusal;
use mzlab_core::error::Result;
use mzlab_core::idxread::{Index, ReadAt};

use crate::params::{intensity_hist, Hist};
use crate::recipe::Recipe;
use crate::trace_base::{BYTES_PER_PEAK, SLAB_PEAKS};

/// Bytes of a detection whatever the file, with the slab of the default `budget`: the module, the recipe, the histograms and the sinks.
pub const FLOOR_BYTES: f64 = 68.0 * 1048576.0;
/// Bytes per MS1 scan (the retention times, the per-scan arrays and the grids of the traces).
pub const PER_SCAN: f64 = 3_250.0;
/// Bytes per MS1 peak at or above the floor (the rows, their quality, isotopes and MS2 links).
pub const PER_KEPT: f64 = 13.0;

/// What the estimate reads: the MS1 scans and peaks of the polarity, and how many of the peaks reach the floor of the recipe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Inputs {
    pub scans: usize,
    pub peaks: u64,
    pub kept: u64,
}

/// The inputs of `index` under `r`: none (all zero) when the file has no MS1 scans of the polarity, which detects nothing.
pub fn inputs<R: ReadAt>(index: &mut Index<R>, r: &Recipe) -> Result<Inputs> {
    let Some(si) = index.section_of(1, r.detect.polarity) else {
        return Ok(Inputs {
            scans: 0,
            peaks: 0,
            kept: 0,
        });
    };
    let (scans, peaks) = (index.sections[si].n_scans, index.sections[si].n_peaks);
    let h = intensity_hist(index, si)?;
    Ok(Inputs {
        scans,
        peaks,
        kept: kept_of(&h, peaks, r.detect.floor),
    })
}

/// `peaks` times the share of the sampled intensities whose bin starts at or above `floor` (the bin holding the floor counts: a margin).
fn kept_of(h: &Hist, peaks: u64, floor: f64) -> u64 {
    let total: u64 = h.counts.iter().sum();
    if total == 0 {
        return 0;
    }
    let above: u64 = (0..h.counts.len())
        .filter(|&i| Hist::edge(i + 1) > floor)
        .map(|i| h.counts[i])
        .sum();
    (peaks as u128 * above as u128).div_ceil(total as u128) as u64
}

/// Bytes the detection of a file with `x` asks for under `r`: the floor, the per-scan and per-peak parts, and what a slab larger than the default one adds.
pub fn bytes(x: &Inputs, r: &Recipe) -> usize {
    let slab = |budget: usize| (budget / BYTES_PER_PEAK).clamp(1024, SLAB_PEAKS) * BYTES_PER_PEAK;
    let more = slab(r.detect.budget).saturating_sub(slab(crate::trace::Params::default().budget));
    (FLOOR_BYTES + PER_SCAN * x.scans as f64 + PER_KEPT * x.kept as f64) as usize + more
}

/// `Ok` when the detection of `x` fits in `cap` bytes, else `error.memoryBudget` with the phase `detect` and its numbers (`budget::refusal`).
pub fn check(x: &Inputs, r: &Recipe, cap: usize) -> Result<()> {
    let need = bytes(x, r);
    if need <= cap {
        return Ok(());
    }
    Err(refusal("detect", need, cap))
}

#[cfg(test)]
#[path = "trace_estimate_tests.rs"]
mod tests;
