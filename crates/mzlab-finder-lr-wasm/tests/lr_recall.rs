//! Sensitivity of the low-resolution Dig: products of the synthetic parent at low intensity (down to the level of the noise ions) and near the dead
//! volume, added to the series of `comune`. A true product never disappears: at worst it is `debole` (the «Weak evidence» band of the table).
//! `cargo test --release -p mzlab-finder-lr-wasm --test lr_recall -- --ignored --nocapture` prints the table threshold -> recall.

use mzlab_feat::legacy::lr;
use std::sync::OnceLock;

mod comune;
use comune::*;

/// (m/z, RT) of products of one transformation of the parent (`regole_tp.json`): two near the dead volume (rt_min 0.5 min).
const TPS: [(f64, f64); 6] = [
    (273.0540, 6.0),
    (239.0485, 10.5),
    (259.0747, 5.6),
    (223.0536, 11.5),
    (185.0015, 0.85),
    (285.0540, 1.2),
];

/// Amplitudes at the plateau; a noise ion is about 3e3.
const LEVELS: [f64; 8] = [3e3, 6e3, 1.2e4, 1.6e4, 2e4, 2.5e4, 5e4, 1e5];

const DEFAULTS: &str = "{}";
const NO_FILTER: &str = r#"{"min_snr": 0, "max_fwhm_s": 1e9, "min_points": 0, "dead_factor": 0, "parent_decay": false}"#;

fn series_at(a: f64) -> &'static (String, Vec<Vec<u8>>) {
    static S: OnceLock<Vec<(String, Vec<Vec<u8>>)>> = OnceLock::new();
    let all = S.get_or_init(|| {
        LEVELS
            .iter()
            .map(|&a| {
                series_with(&move |t| {
                    t.map_or(vec![], |k| {
                        TPS.iter()
                            .map(|&(m, rt)| (m, rt, 0.07, a * (1.0 - (-0.05 * k).exp())))
                            .collect()
                    })
                })
            })
            .collect()
    });
    &all[LEVELS.iter().position(|&x| x == a).unwrap()]
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct Tally {
    shown: usize,
    weak: usize,
    missing: usize,
}

fn tally(d: &lr::LrDig) -> Tally {
    let mut t = Tally::default();
    for &(m, rt) in &TPS {
        let e = d
            .entries
            .iter()
            .find(|e| (e.mz - m).abs() <= 0.35 && e.ref_rt.is_some_and(|x| (x - rt).abs() <= 0.25));
        match e.map(|e| e.label) {
            Some("forte" | "possibile") => t.shown += 1,
            Some(_) => t.weak += 1,
            None => t.missing += 1,
        }
    }
    t
}

fn run(a: f64, settings: &str) -> Tally {
    let (files, idxs) = series_at(a);
    tally(&dig_with(files, PARENT, settings, idxs))
}

#[test]
fn no_true_product_goes_from_shown_to_hidden() {
    // at the limit of detection of the pipeline (1.6e4-2.5e4) the filters with their defaults show what the Dig without them shows
    for a in [1.6e4, 2e4, 2.5e4] {
        let (before, now) = (run(a, NO_FILTER), run(a, DEFAULTS));
        assert!(
            now.shown >= before.shown && now.missing <= before.missing,
            "amplitude {a}: without filters {before:?}, defaults {now:?}"
        );
    }
    let strong = run(1e5, DEFAULTS);
    assert_eq!(strong.shown, TPS.len(), "{strong:?}");
}

#[test]
#[ignore]
fn table_threshold_recall() {
    let sets = [
        ("no filter", NO_FILTER.to_string()),
        ("S/N 3", r#"{"min_snr": 3}"#.to_string()),
        ("S/N 5", r#"{"min_snr": 5}"#.to_string()),
        ("S/N 10", r#"{"min_snr": 10}"#.to_string()),
        ("defaults", DEFAULTS.to_string()),
    ];
    println!(
        "amplitude | {}",
        sets.iter().map(|s| s.0).collect::<Vec<_>>().join(" | ")
    );
    for a in LEVELS {
        let row: Vec<String> = sets
            .iter()
            .map(|(_, s)| {
                let t = run(a, s);
                format!("{}+{}w/{}", t.shown, t.weak, TPS.len())
            })
            .collect();
        println!("{a:e} | {}", row.join(" | "));
    }
}
