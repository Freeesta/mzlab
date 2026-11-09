//! The limits of one 1-Da bucket of the `.mzidx` builder: the round that holds it when the peaks are in a scratch file, and the refusal of a bucket
//! too dense for any round, given by pass 1 as soon as the peaks it counted prove it (`tests/mzidx_denso.rs`).

use super::{First, Peak, DEFAULT_BUDGET, NB};
use crate::error::{Error, Result};

/// The largest round when the peaks are in a scratch file (the round of the default budget, `DEFAULT_BUDGET / 2`).
const ROUND_MAX: usize = DEFAULT_BUDGET / 2;

/// How often (in scans) pass 1 checks that no 1-Da bucket has outgrown what a round may hold.
const DENSE_EVERY: usize = 256;

/// The bytes of peaks one round may hold: half of the budget (a round holds its peaks and, while it is written, the same peaks as byte arrays) minus
/// what the build holds whatever the round (`fixed`). Read from a scratch file (`spill`) a round costs no decoding, so a larger one only holds more
/// memory, and measured slower (`misure-rust.md`): at most `ROUND_MAX`.
pub(super) fn round_cap(budget: usize, fixed: usize, spill: bool) -> usize {
    let cap = (budget / 2).saturating_sub(fixed).max(budget / 8);
    if spill {
        cap.min(ROUND_MAX)
    } else {
        cap
    }
}

impl First<'_> {
    /// Every `DENSE_EVERY` scans: the refusal `build_dopo` would give at the end, as soon as the peaks counted so far prove it: a lone bucket is admitted while its sections
    /// cost at most `budget * 2`, and every scan is in its level's "all" section (12 B a peak if that section holds `f32`, else 24). So the rest of
    /// the file is neither decoded, kept nor spilled for an index that cannot be built.
    #[allow(clippy::indexing_slicing)] // `counts` and `counts32` have NB items
    pub(super) fn too_dense(&self) -> Result<()> {
        if !self.visti.is_multiple_of(DENSE_EVERY) {
            return Ok(());
        }
        let at_least = |b: usize| -> usize {
            self.groups
                .iter()
                .map(|g| match g.counts32.get(b) {
                    Some(&c32) => c32.min(g.counts[b]) as usize * std::mem::size_of::<Peak<f32>>(),
                    None => g.counts[b] as usize * std::mem::size_of::<Peak<f64>>(),
                })
                .sum()
        };
        match (0..NB).map(at_least).max() {
            Some(c) if c > self.budget * 2 => Err(slice_refusal(c, self.budget)),
            _ => Ok(()),
        }
    }
}

/// The refusal of a 1-Da bucket whose peaks (`cost` bytes in the sections) are more than a lone round may hold (`budget * 2`): phase `indexPeaks`,
/// whose `stima_mb` and `budget_mb` are the bucket and that limit, so the page names the real limit of one file's builder.
pub(super) fn slice_refusal(cost: usize, budget: usize) -> Error {
    crate::budget::refusal("indexPeaks", cost, budget * 2)
}
