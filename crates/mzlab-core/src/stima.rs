//! Memory estimates of the load of a big mzML, made BEFORE the memory is taken: the scan table (`scantable::read`, phase `table`) and the `.mzidx`
//! builder (phase `index`). A load that does not fit in the budget of the instance ends with the orderly `error.memoryBudget`
//! (`budget::refusal`: phase, estimate and budget in MB), never with a worker that dies. The per-scan figures are measured with
//! `examples/mzml_memoria.rs` on files of 0.5 to 4 GB (heap peak divided by the scans, rounded up); `docs/agenti/memoria.md` has the table.

use crate::budget::{refusal, MemBudget};
use crate::error::Result;

const MB: usize = 1 << 20;

/// Peak heap of the scan table per scan: the row (176 bytes), what a scan with a precursor adds (about 330), the native id, and the offsets of the
/// spectra; the JSON that comes out of it is counted in as it grows. Measured 556 to 563.
pub const TABLE_PER_SCAN: usize = 640;
/// What the index builder holds per scan beyond the JSON of the rows: the four columns, the per-scan tables and the groups (measured 160 to 180).
pub const INDEX_PER_SCAN: usize = 220;
/// What a load holds apart from the scans: the buffers of the reader and of the hash, the head of the file.
pub const FIXED: usize = 8 * MB;
/// Scans between two looks at the progress when the file has no index of offsets (the count is not known before the walk).
pub const EVERY: usize = 4096;

/// Peak heap of `scantable::read` for `scans` spectra.
pub fn table_bytes(scans: usize) -> usize {
    FIXED.saturating_add(scans.saturating_mul(TABLE_PER_SCAN))
}

/// Peak heap of the `.mzidx` builder: the table it is given (`scans_json` and the columns), the per-scan tables, and the part of the budget for the
/// peaks that it keeps in memory (half of it; the rest waits in the scratch file).
pub fn index_bytes(scans: usize, scans_json: usize, peaks_budget: usize) -> usize {
    FIXED
        .saturating_add(scans_json)
        .saturating_add(scans.saturating_mul(INDEX_PER_SCAN + 18))
        .saturating_add(peaks_budget / 2)
}

/// Refuses in order (`error.memoryBudget`, phase `table`) if the table of `scans` spectra does not fit in the budget.
pub fn check_table(budget: &MemBudget, scans: usize) -> Result<()> {
    let est = table_bytes(scans);
    if est > budget.cap() {
        return Err(refusal("table", est, budget.cap()));
    }
    Ok(())
}

/// The same while the scans are counted by walking the file (no index of offsets): `seen` spectra found in the first `pos` bytes of `size`; the
/// estimate is the extrapolation to the whole file. Looks only every `EVERY` scans.
pub fn check_walk(budget: &MemBudget, seen: usize, pos: u64, size: u64) -> Result<()> {
    if seen == 0 || !seen.is_multiple_of(EVERY) || pos == 0 {
        return Ok(());
    }
    let total = (seen as u128 * size as u128 / pos as u128).min(usize::MAX as u128) as usize;
    check_table(budget, total.max(seen))
}

/// Refuses in order (phase `index`) if the builder, given a table of `scans` spectra whose rows are `scans_json` bytes and `peaks_budget` bytes for
/// the peaks, would not fit in the budget.
pub fn check_index(
    budget: &MemBudget,
    scans: usize,
    scans_json: usize,
    peaks_budget: usize,
) -> Result<()> {
    let est = index_bytes(scans, scans_json, peaks_budget);
    if est > budget.cap() {
        return Err(refusal("index", est, budget.cap()));
    }
    Ok(())
}

/// The largest number of scans a table can have in `budget` (what the settings show next to the budget).
pub fn max_scans(budget: &MemBudget) -> usize {
    budget.cap().saturating_sub(FIXED) / TABLE_PER_SCAN
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_tables_fit_and_big_ones_are_refused_with_the_numbers() {
        let b = MemBudget::mb(192.0);
        assert!(check_table(&b, 100_000).is_ok());
        let e = check_table(&b, 1_200_000).unwrap_err();
        assert_eq!(e.key, "error.memoryBudget");
        let p = |k: &str| {
            e.params
                .iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| v.clone())
        };
        assert_eq!(p("fase").as_deref(), Some("table"));
        assert_eq!(p("budget_mb").as_deref(), Some("192"));
        assert!(p("stima_mb").unwrap().parse::<usize>().unwrap() > 700);
    }

    #[test]
    fn walk_extrapolates_from_the_progress() {
        let b = MemBudget::mb(64.0);
        // a quarter of the file holds 40 000 scans: 160 000 in all, 100 MB of table
        assert!(check_walk(&b, 40_960, 100, 400).is_err());
        // not a multiple of EVERY: no look
        assert!(check_walk(&b, 40_001, 100, 400).is_ok());
        assert!(check_walk(&MemBudget::mb(192.0), 8192, 1000, 2000).is_ok());
    }

    #[test]
    fn the_index_counts_the_json_and_half_of_the_peaks_budget() {
        let b = MemBudget::mb(192.0);
        assert!(check_index(&b, 100_000, 28 * MB, 64 * MB).is_ok());
        let e = check_index(&b, 100_000, 28 * MB, 400 * MB).unwrap_err();
        assert_eq!(e.key, "error.memoryBudget");
        assert!(max_scans(&b) > 250_000 && max_scans(&b) < 310_000);
    }
}
