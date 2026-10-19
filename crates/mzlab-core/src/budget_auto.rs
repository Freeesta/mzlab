//! The memory budget chosen by the program (mode "Automatic"): from what is known of the machine it gives the budget in MB, the reason (a key the page
//! translates), the confidence and the numbers it used, so that the choice can be shown, changed by hand and written in the audit log of the page.
//! A browser engine has a hard limit of 256 MB fixed by the build (the budget is 75 % of it at most); a native engine (desktop app) has no such limit
//! and takes a share of the RAM. The budget never replaces the estimates of `stima`: those refuse a file that does not fit, whatever the budget is.

use crate::budget::{NATIVE_BUDGET_MB, WASM_BUDGET_MB};
use crate::json::Out;

/// Hard limit of the memory of one WASM instance, MB (`.cargo/config.toml`).
pub const WASM_HARD_MB: usize = 256;
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
}

/// A budget with its reason, confidence and the limit it sits under.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    pub mb: usize,
    /// `automatic` or `manual`.
    pub mode: &'static str,
    /// The rule that gave the number (`budget.auto.<reason>` in the catalogs of the page).
    pub reason: &'static str,
    /// `high` (the rule used a number the machine gave), `medium` (it used a default for a number it could not read) or `low`.
    pub confidence: &'static str,
    /// The limit above which no budget can go: 256 MB of the build for the WASM, the RAM for the native engine.
    pub hard_mb: usize,
}

/// The choice as the JSON the page shows and logs: the numbers the rule used are in `inputs`.
pub fn json(c: &Choice, m: &Machine, max_scans: usize) -> String {
    let mut o = Out::default();
    o.raw("{\"mode\":")
        .str(c.mode)
        .raw(",\"mb\":")
        .int(c.mb as i64);
    o.raw(",\"reason\":")
        .str(c.reason)
        .raw(",\"confidence\":")
        .str(c.confidence);
    o.raw(",\"hard_mb\":")
        .int(c.hard_mb as i64)
        .raw(",\"max_scans\":")
        .int(max_scans as i64);
    o.raw(&format!(
        ",\"inputs\":{{\"native\":{},\"device_gb\":{},\"ram_mb\":{},\"cores\":{},\"small\":{}}}}}",
        m.native, m.device_gb as i64, m.ram_mb as i64, m.cores, m.small
    ));
    o.0
}

fn ceiling(m: &Machine) -> usize {
    if m.native {
        NATIVE_MAX_MB
    } else {
        WASM_BUDGET_MB
    }
}

/// The budget the program chooses for `m`.
pub fn auto(m: &Machine) -> Choice {
    let (mb, reason, confidence) = if m.native {
        if m.ram_mb >= 1024.0 {
            let mb = ((m.ram_mb / 4.0) as usize).clamp(NATIVE_BUDGET_MB, NATIVE_MAX_MB);
            (mb, "ramShare", "high")
        } else {
            (NATIVE_BUDGET_MB, "ramUnknown", "low")
        }
    } else if m.device_gb > 0.0 && m.device_gb <= 1.0 {
        (96, "lowMemory", "high")
    } else if m.device_gb > 0.0 && m.device_gb <= 2.0 {
        (128, "lowMemory", "high")
    } else if m.small {
        (
            WASM_BUDGET_MB,
            "smallDevice",
            if m.device_gb > 0.0 { "high" } else { "medium" },
        )
    } else {
        (
            WASM_BUDGET_MB,
            "buildLimit",
            if m.device_gb > 0.0 { "high" } else { "medium" },
        )
    };
    Choice {
        mb,
        mode: "automatic",
        reason,
        confidence,
        hard_mb: if m.native {
            m.ram_mb as usize
        } else {
            WASM_HARD_MB
        },
    }
}

/// A budget chosen by hand, kept inside what the machine allows (`MIN_MB` to the ceiling of its kind).
pub fn manual(m: &Machine, mb: f64) -> Choice {
    let top = ceiling(m);
    let mb = if mb.is_finite() { mb as usize } else { top };
    Choice {
        mb: mb.clamp(MIN_MB, top),
        mode: "manual",
        reason: "manual",
        confidence: "user",
        hard_mb: auto(m).hard_mb,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn browser(device_gb: f64, small: bool) -> Machine {
        Machine {
            device_gb,
            small,
            cores: 4,
            ..Machine::default()
        }
    }

    #[test]
    fn a_browser_never_goes_over_what_the_build_allows() {
        for gb in [0.0, 0.5, 1.0, 2.0, 4.0, 8.0] {
            for small in [false, true] {
                let c = auto(&browser(gb, small));
                assert!(c.mb <= WASM_BUDGET_MB && c.mb >= MIN_MB, "{gb} {small}");
                assert_eq!(c.hard_mb, WASM_HARD_MB);
            }
        }
    }

    #[test]
    fn a_phone_keeps_192_of_256_and_a_poor_device_less() {
        assert_eq!(auto(&browser(4.0, true)).mb, 192);
        assert_eq!(auto(&browser(1.0, true)).mb, 96);
        assert_eq!(auto(&browser(2.0, false)).mb, 128);
        let unknown = auto(&browser(0.0, true));
        assert_eq!((unknown.mb, unknown.confidence), (192, "medium"));
    }

    #[test]
    fn a_native_engine_takes_a_quarter_of_the_ram_between_1_and_16_gb() {
        let m = |ram_mb| Machine {
            native: true,
            ram_mb,
            cores: 8,
            ..Machine::default()
        };
        assert_eq!(auto(&m(8192.0)).mb, 2048);
        assert_eq!(auto(&m(2048.0)).mb, NATIVE_BUDGET_MB);
        assert_eq!(auto(&m(1_000_000.0)).mb, NATIVE_MAX_MB);
        let none = auto(&m(0.0));
        assert_eq!(
            (none.mb, none.reason, none.confidence),
            (1024, "ramUnknown", "low")
        );
    }

    #[test]
    fn a_manual_budget_is_kept_inside_the_limits_and_says_so() {
        let b = browser(8.0, false);
        assert_eq!(manual(&b, 1000.0).mb, WASM_BUDGET_MB);
        assert_eq!(manual(&b, 1.0).mb, MIN_MB);
        assert_eq!(manual(&b, f64::NAN).mb, WASM_BUDGET_MB);
        let c = manual(&b, 128.0);
        assert_eq!((c.mb, c.mode, c.reason), (128, "manual", "manual"));
        let j = json(&c, &b, 100);
        assert!(j.starts_with("{\"mode\":\"manual\",\"mb\":128"));
    }
}
