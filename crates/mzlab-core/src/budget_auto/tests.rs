use super::*;

fn browser(device_gb: f64, small: bool) -> Machine {
    Machine {
        device_gb,
        small,
        cores: 4,
        ..Machine::default()
    }
}

fn pair(m: &Machine) -> (usize, usize) {
    let c = auto(m);
    (c.mb, c.high_mb)
}

#[test]
fn the_table_of_the_devices() {
    assert_eq!(pair(&browser(0.5, false)), (96, 96));
    assert_eq!(pair(&browser(1.0, true)), (96, 96));
    assert_eq!(pair(&browser(2.0, false)), (128, 192));
    assert_eq!(pair(&browser(4.0, true)), (192, 192));
    assert_eq!(pair(&browser(4.0, false)), (512, 1024));
    assert_eq!(pair(&browser(8.0, false)), (1024, 3072));
    let many = Machine {
        cores: 12,
        ..browser(8.0, false)
    };
    assert_eq!(pair(&many), (2048, 3072));
    assert_eq!(pair(&browser(0.0, false)), (512, 2048));
    let ios = Machine {
        ios: true,
        ..browser(0.0, false)
    };
    assert_eq!(pair(&ios), (192, 192));
    let unknown = auto(&browser(0.0, true));
    assert_eq!((unknown.mb, unknown.confidence), (192, "medium"));
    assert_eq!(auto(&browser(8.0, false)).confidence, "high");
}

#[test]
fn a_browser_never_goes_over_the_levels_or_the_build() {
    for gb in [0.0, 0.5, 1.0, 2.0, 4.0, 8.0] {
        for small in [false, true] {
            for cores in [2, 8, 16] {
                let c = auto(&Machine {
                    cores,
                    ..browser(gb, small)
                });
                assert!(
                    c.mb >= 96 && c.mb <= c.high_mb && c.high_mb <= 3072,
                    "{gb} {small}"
                );
                assert!(LEVELS.contains(&c.mb) || c.mb == c.high_mb);
                assert_eq!(c.hard_mb, WASM_HARD_MB);
            }
        }
    }
}

#[test]
fn the_probe_only_lowers_the_budgets() {
    let probed = |gb: f64, probe: f64| {
        pair(&Machine {
            probe_mb: probe,
            ..browser(gb, false)
        })
    };
    assert_eq!(probed(8.0, 3072.0), (1024, 3072));
    assert_eq!(probed(8.0, 1536.0), (1024, 1536));
    assert_eq!(probed(8.0, 768.0), (768, 768));
    // a probe under 192 never takes the engine under what it has always had
    assert_eq!(probed(8.0, 100.0), (192, 192));
    assert_eq!(probed(8.0, 0.0), (1024, 3072));
    assert_eq!(probed(8.0, f64::NAN), (1024, 3072));
    // the probe cannot raise a phone
    assert_eq!(
        pair(&Machine {
            probe_mb: 3072.0,
            ..browser(4.0, true)
        }),
        (192, 192)
    );
}

#[test]
fn the_supervisor_goes_down_one_level_at_a_time() {
    assert_eq!(budget_down(3072, 1), Some(2048));
    assert_eq!(budget_down(1024, 1), Some(512));
    assert_eq!(budget_down(1024, 2), Some(192));
    assert_eq!(budget_down(192, 1), Some(128));
    assert_eq!(budget_down(192, 2), Some(128));
    assert_eq!(budget_down(96, 1), None);
    assert_eq!(budget_down(1024, 3), None);
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
    assert_eq!(auto(&m(8192.0)).high_mb, 2048);
    let none = auto(&m(0.0));
    assert_eq!(
        (none.mb, none.reason, none.confidence),
        (1024, "ramUnknown", "low")
    );
}

#[test]
fn a_manual_budget_is_kept_inside_the_limits_and_says_so() {
    let b = browser(8.0, false);
    assert_eq!(manual(&b, 100_000.0).mb, 3072);
    assert_eq!(manual(&b, 1.0).mb, MIN_MB);
    assert_eq!(manual(&b, f64::NAN).mb, 3072);
    let c = manual(&b, 128.0);
    assert_eq!((c.mb, c.mode, c.reason), (128, "manual", "manual"));
    assert_eq!(manual(&browser(4.0, true), 1000.0).mb, 192);
    let j = json(&c, &b, 100);
    assert!(j.starts_with("{\"mode\":\"manual\",\"mb\":128"));
    assert!(j.contains("\"levels\":[96,128,192,512,1024,2048,3072]"));
}
