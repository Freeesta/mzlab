//! The memory budget of the engine as the page asks for it (`budget_choice`); the rule itself is `mzlab-core::budget_auto`.

use mzlab_core::budget::MemBudget;
use wasm_bindgen::prelude::*;

/// The memory budget of the engine (`mzlab-core::budget_auto`): chosen by the program from what the browser says of the device, or `manual_mb` when it is a
/// positive number (kept inside the limits). `ios` is Safari on iOS/iPadOS, `probe_mb` what the growth probe obtained (0 = none). JSON: `{mode, mb, safe_mb,
/// high_mb, levels, reason, confidence, hard_mb, max_scans, inputs}`.
#[wasm_bindgen]
pub fn budget_choice(
    device_gb: f64,
    cores: u32,
    small: bool,
    manual_mb: f64,
    ios: bool,
    probe_mb: f64,
) -> String {
    use mzlab_core::budget_auto as ba;
    let m = ba::Machine {
        device_gb: if device_gb.is_finite() {
            device_gb.max(0.0)
        } else {
            0.0
        },
        cores,
        small,
        ios,
        probe_mb: if probe_mb.is_finite() {
            probe_mb.max(0.0)
        } else {
            0.0
        },
        ..ba::Machine::default()
    };
    let c = if manual_mb.is_finite() && manual_mb > 0.0 {
        ba::manual(&m, manual_mb)
    } else {
        ba::auto(&m)
    };
    ba::json(
        &c,
        &m,
        mzlab_core::stima::max_scans(&MemBudget::mb(c.mb as f64)),
    )
}

/// The budget in MB after a death for lack of memory with `current_mb` (`strikes` = deaths within a minute): 0 = stop and show `error.memoryBudget`.
#[wasm_bindgen]
pub fn budget_down(current_mb: f64, strikes: u32) -> f64 {
    if !current_mb.is_finite() || current_mb < 0.0 {
        return 0.0;
    }
    mzlab_core::budget_auto::budget_down(current_mb as usize, strikes).map_or(0.0, |mb| mb as f64)
}
