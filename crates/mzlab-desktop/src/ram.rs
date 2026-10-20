//! The memory of the computer and the budget the program chooses from it for the native engine (`mzlab-core::budget_auto`, mode Automatic).

use mzlab_core::budget::MemBudget;
use mzlab_core::mzidx::{ScanTable, DEFAULT_BUDGET};
use mzlab_core::{budget_auto, stima, Error};

/// The budget in MB the program chooses for this computer (`budget_auto`, mode Automatic).
pub(crate) fn native_budget_mb() -> f64 {
    budget_auto::native_auto_mb()
}

/// Refuses in order, before the builder takes memory, a file whose index would not fit the budget chosen for this machine (`error.memoryBudget`, phase `index`).
pub(crate) fn check_index(table: &ScanTable) -> Result<(), Error> {
    let budget = MemBudget::mb(native_budget_mb());
    stima::check_index(
        &budget,
        table.level.len(),
        table.scans_json.len(),
        DEFAULT_BUDGET,
    )
}
