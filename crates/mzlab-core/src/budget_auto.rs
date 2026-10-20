//! The memory budget chosen by the program (mode "Automatic"): from what is known of the machine it gives two budgets in MB, `safe` (the engine works
//! with it without asking) and `high` (usable only after the page warned), the reason (a key the page translates), the confidence and the numbers it
//! used, so that the choice can be shown and written in the audit log of the page. A browser instance may grow to 4 GiB (the build limit, which reserves
//! nothing) but the program never goes over 3072 MB there; a native engine (desktop app) takes a share of the RAM. The budget never replaces the
//! estimates of `stima`: those refuse a file that does not fit, whatever the budget is.

use crate::budget::{NATIVE_BUDGET_MB, WASM_BUDGET_MB};
use crate::json::Out;

/// Hard limit of the memory of one WASM instance, MB (`.cargo/config.toml`: 4 GiB, the maximum of wasm32).
pub const WASM_HARD_MB: usize = 4096;
/// The levels a browser budget takes, MB; the supervisor goes down one at a time. Never above 3072 (code, stack and fragmentation need room under the
/// 4 GiB of wasm32).
pub const LEVELS: [usize; 7] = [96, 128, 192, 512, 1024, 2048, 3072];
/// Smallest budget the program chooses or accepts (a smaller one cannot open a file).
pub const MIN_MB: usize = 64;
/// Largest native budget, MB.
pub const NATIVE_MAX_MB: usize = 16384;

/// What the program knows of the machine; a number it could not read is `0`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Machine {
    /// The engine runs natively (desktop app); otherwise it is the WASM of a browser.
    pub native: bool,
    /// Memory of the device in GB as the browser says it (`navigator.deviceMemory`, rounded down to a power of two and at most 8): browser only.
    pub device_gb: f64,
    /// RAM of the computer in MB: native only.
    pub ram_mb: f64,
    /// Logical cores.
    pub cores: u32,
    /// Phone or tablet (the page decides, `telefono.js`).
    pub small: bool,
    /// Safari on iOS or iPadOS (the page decides): the system closes tabs that use more than about 1 GB.
    pub ios: bool,
    /// The most memory the growth probe of the page obtained, MB; `0` = no probe (it did not run, it failed or it took too long).
    pub probe_mb: f64,
}

/// A budget with its reason, confidence and the limit it sits under.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    /// The safe budget: the one the engine works with (what older callers call "the budget").
    pub mb: usize,
    /// The budget usable after a warning (`>= mb`).
    pub high_mb: usize,
    /// `automatic` or `manual`.
    pub mode: &'static str,
    /// The rule that gave the number (`budget.auto.<reason>` in the catalogs of the page).
    pub reason: &'static str,
    /// `high` (the rule used a number the machine gave), `medium` (it used a default for a number it could not read) or `low`.
    pub confidence: &'static str,
    /// The limit above which no budget can go: 4 GiB of the build for the WASM, the RAM for the native engine.
    pub hard_mb: usize,
}

/// The choice as the JSON the page shows and logs: the numbers the rule used are in `inputs`.
pub fn json(c: &Choice, m: &Machine, max_scans: usize) -> String {
    let mut o = Out::default();
    o.raw("{\"mode\":")
        .str(c.mode)
        .raw(",\"mb\":")
        .int(c.mb as i64)
        .raw(",\"safe_mb\":")
        .int(c.mb as i64)
        .raw(",\"high_mb\":")
        .int(c.high_mb as i64);
    o.raw(",\"levels\":[");
    if !m.native {
        for (i, l) in LEVELS.iter().enumerate() {
            if i > 0 {
                o.raw(",");
            }
            o.int(*l as i64);
        }
    }
    o.raw("],\"reason\":")
        .str(c.reason)
        .raw(",\"confidence\":")
        .str(c.confidence);
    o.raw(",\"hard_mb\":")
        .int(c.hard_mb as i64)
        .raw(",\"max_scans\":")
        .int(max_scans as i64);
    o.raw(&format!(
        ",\"inputs\":{{\"native\":{},\"device_gb\":{},\"ram_mb\":{},\"cores\":{},\"small\":{},\"ios\":{},\"probe_mb\":{}}}}}",
        m.native, m.device_gb as i64, m.ram_mb as i64, m.cores, m.small, m.ios, m.probe_mb as i64
    ));
    o.0
}

fn ceiling(m: &Machine) -> usize {
    if m.native {
        NATIVE_MAX_MB
    } else {
        auto(m).high_mb
    }
}

/// The browser table: (safe, high, reason) from the facts of the device, before the probe.
fn browser_levels(m: &Machine) -> (usize, usize, &'static str) {
    let known = m.device_gb > 0.0;
    if known && m.device_gb <= 1.0 {
        (96, 96, "lowMemory")
    } else if known && m.device_gb <= 2.0 {
        (128, 192, "lowMemory")
    } else if m.small || m.ios {
        (WASM_BUDGET_MB, WASM_BUDGET_MB, "smallDevice")
    } else if !known {
        (512, 2048, "unknownDevice")
    } else if m.device_gb <= 4.0 {
        (512, 1024, "computerMemory")
    } else if m.cores >= 8 {
        (2048, 3072, "computerMemory")
    } else {
        (1024, 3072, "computerMemory")
    }
}

/// The budgets the program chooses for `m`.
pub fn auto(m: &Machine) -> Choice {
    if m.native {
        let (mb, reason, confidence) = if m.ram_mb >= 1024.0 {
            let mb = ((m.ram_mb / 4.0) as usize).clamp(NATIVE_BUDGET_MB, NATIVE_MAX_MB);
            (mb, "ramShare", "high")
        } else {
            (NATIVE_BUDGET_MB, "ramUnknown", "low")
        };
        return Choice {
            mb,
            high_mb: mb,
            mode: "automatic",
            reason,
            confidence,
            hard_mb: m.ram_mb as usize,
        };
    }
    let (mut safe, mut high, reason) = browser_levels(m);
    // the probe only lowers: a browser that grows less than the table says wins; never under the 192 MB the engine has always worked with
    if m.probe_mb.is_finite() && m.probe_mb > 0.0 {
        high = high.min((m.probe_mb as usize).max(WASM_BUDGET_MB.min(high)));
        safe = safe.min(high);
    }
    Choice {
        mb: safe,
        high_mb: high,
        mode: "automatic",
        reason,
        confidence: if m.device_gb > 0.0 || m.probe_mb > 0.0 {
            "high"
        } else {
            "medium"
        },
        hard_mb: WASM_HARD_MB,
    }
}

/// The budget after the engine died for lack of memory while it worked with `current_mb`: `strikes` is the number of such deaths within a minute (1 =
/// the first). The first goes one level down, the second to 192 at most (or one level down if that is lower), the third stops (`None`: the page shows
/// `error.memoryBudget`).
pub fn budget_down(current_mb: usize, strikes: u32) -> Option<usize> {
    if strikes >= 3 {
        return None;
    }
    let below = LEVELS.iter().rev().copied().find(|&l| l < current_mb)?;
    Some(if strikes >= 2 {
        below.min(WASM_BUDGET_MB)
    } else {
        below
    })
}

/// A budget chosen by hand, kept inside what the machine allows (`MIN_MB` to the ceiling of its kind).
pub fn manual(m: &Machine, mb: f64) -> Choice {
    let top = ceiling(m);
    let mb = if mb.is_finite() { mb as usize } else { top };
    let a = auto(m);
    let mb = mb.clamp(MIN_MB, top);
    Choice {
        mb,
        high_mb: mb.max(a.high_mb.min(top)),
        mode: "manual",
        reason: "manual",
        confidence: "user",
        hard_mb: a.hard_mb,
    }
}

#[cfg(test)]
mod tests;
