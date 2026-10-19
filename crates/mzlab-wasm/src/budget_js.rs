//! The memory budget of the engine as the page asks for it (`budget_choice`); the rule itself is `mzlab-core::budget_auto`.

use mzlab_core::budget::MemBudget;
use wasm_bindgen::prelude::*;

/// The memory budget of the engine (`mzlab-core::budget_auto`): chosen by the program from what the browser says of the device, or `manual_mb` when it is a
/// positive number (kept inside the limits). JSON: `{mode, mb, reason, confidence, hard_mb, max_scans, inputs}`.
#[wasm_bindgen]
pub fn budget_choice(device_gb: f64, cores: u32, small: bool, manual_mb: f64) -> String {
    use mzlab_core::budget_auto as ba;
    let m = ba::Machine {
        device_gb: if device_gb.is_finite() {
            device_gb.max(0.0)
        } else {
            0.0
        },
        cores,
        small,
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
