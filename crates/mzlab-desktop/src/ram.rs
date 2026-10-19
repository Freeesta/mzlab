//! The memory of the computer and the budget the program chooses from it for the native engine (`mzlab-core::budget_auto`, mode Automatic).

use mzlab_core::budget::MemBudget;
use mzlab_core::mzidx::{ScanTable, DEFAULT_BUDGET};
use mzlab_core::{budget_auto, stima, Error};

/// RAM of the computer in MB, 0 when it cannot be read (Windows has no simple source without a system call: the budget then takes its default).
fn ram_mb() -> f64 {
    if let Ok(t) = std::fs::read_to_string("/proc/meminfo") {
        let kb = t.lines().find_map(|l| l.strip_prefix("MemTotal:"));
        return kb
            .and_then(|v| v.trim().trim_end_matches("kB").trim().parse::<f64>().ok())
            .map_or(0.0, |kb| kb / 1024.0);
    }
    let out = std::process::Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output();
    out.ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|t| t.trim().parse::<f64>().ok())
        .map_or(0.0, |b| b / (1024.0 * 1024.0))
}

/// The budget in MB the program chooses for this computer (`budget_auto`, mode Automatic).
pub(crate) fn native_budget_mb() -> f64 {
    let cores = std::thread::available_parallelism().map_or(0, |n| n.get() as u32);
    let m = budget_auto::Machine {
        native: true,
        ram_mb: ram_mb(),
        cores,
        ..Default::default()
    };
    budget_auto::auto(&m).mb as f64
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
