//! Sensitivity of the low-resolution Dig: products of the synthetic parent at low intensity (down to the level of the noise ions) and near the dead
//! volume, added to the series of `comune`. A true product never disappears: at worst it is `debole` (the «Weak evidence» band of the table).
//! `cargo test --release -p mzlab-finder-lr-wasm --test lr_recall -- --ignored --nocapture` prints the table threshold -> recall.

use mzlab_feat::legacy::lr;
use std::sync::OnceLock;

mod comune;
use comune::*;
use mzlab_feat_wasm::run;

/// (m/z, RT) of products of one transformation of the parent (`regole_tp_2.json`): two near the dead volume (rt_min 0.5 min).
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
    // four times the noise is shown with the thresholds of the profile
    let weak = run(1.2e4, DEFAULTS);
    assert_eq!(weak.shown, TPS.len(), "{weak:?}");
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

/// Bench of the thresholds: products at 2-10 times the noise (an ion of noise is about 3e3) against series of pure noise (decoys, the pool of `comune`
/// and no product). `cargo test --release -p mzlab-finder-lr-wasm --test lr_recall -- --ignored --nocapture bench` prints the curve recall / false
/// positives per threshold pair `(floor, min_height)`.
const BENCH_LEVELS: [f64; 6] = [6e3, 9e3, 1.2e4, 1.5e4, 2.1e4, 3e4];
const BENCH_PAIRS: [(f64, f64); 12] = [
    (2e3, 2e4),
    (2e3, 1e4),
    (3e3, 1e4),
    (4e3, 1e4),
    (3e3, 7e3),
    (3e3, 5e3),
    (3.5e3, 7e3),
    (3.5e3, 5e3),
    (4e3, 7e3),
    (4e3, 5e3),
    (4e3, 4e3),
    (5e3, 5e3),
];
const DECOY_SEEDS: [u64; 3] = [200, 400, 600];

fn bench_series(i: usize) -> &'static (String, Vec<Vec<u8>>) {
    static S: OnceLock<Vec<(String, Vec<Vec<u8>>)>> = OnceLock::new();
    &S.get_or_init(|| {
        BENCH_LEVELS
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
    })[i]
}

fn decoy_series(i: usize) -> &'static (String, Vec<Vec<u8>>) {
    static S: OnceLock<Vec<(String, Vec<Vec<u8>>)>> = OnceLock::new();
    &S.get_or_init(|| {
        DECOY_SEEDS
            .iter()
            .map(|&o| series_seeded(o, &|_| Vec::new()))
            .collect()
    })[i]
}

/// Features of the sample files at the end of the series that match no component of the series (the pool and the products): the detections of noise.
fn spurious(idxs: &[Vec<u8>], tune: (f64, f64), with_tps: bool) -> (usize, usize) {
    let (mut n, mut files) = (0, 0);
    for (i, idx) in idxs.iter().enumerate() {
        let name = format!("f{i}");
        let bytes = run::detect(&mut open(idx), &name, &recipe_at(idx, Some(tune))).unwrap();
        let (feats, _) = run::read_files([bytes]).unwrap();
        let f = &feats[0];
        let known: Vec<(f64, f64)> = TIMES
            .iter()
            .flat_map(|&t| pool(Some(t)))
            .map(|c| (c.0 + SHIFT, c.1))
            .chain(pool(None).iter().map(|c| (c.0 + SHIFT, c.1)))
            .chain(
                TPS.iter()
                    .filter(|_| with_tps)
                    .map(|&(m, rt)| (m + SHIFT, rt)),
            )
            .collect();
        for k in 0..f.mz.len() {
            let hit = known.iter().any(|&(m, rt)| {
                (f.mz[k] - m).abs() <= 0.5 && (f64::from(f.rt[k]) - rt).abs() <= 0.15
            });
            n += usize::from(!hit);
        }
        files += 1;
    }
    (n, files)
}

/// Products found in the last sample file by the detection alone.
fn detected_tps(idxs: &[Vec<u8>], tune: (f64, f64)) -> usize {
    let idx = &idxs[TIMES.len() - 1];
    let bytes = run::detect(&mut open(idx), "last", &recipe_at(idx, Some(tune))).unwrap();
    let (feats, _) = run::read_files([bytes]).unwrap();
    let f = &feats[0];
    TPS.iter()
        .filter(|&&(m, rt)| {
            (0..f.mz.len()).any(|k| {
                (f.mz[k] - m - SHIFT).abs() <= 0.5 && (f64::from(f.rt[k]) - rt).abs() <= 0.2
            })
        })
        .count()
}

#[test]
#[ignore]
fn bench_recall_false_positives() {
    println!("recall: products of {} found in the last sample (detection) / shown+weak in the Dig; x = amplitude / 3e3", TPS.len());
    println!("false positives: features of the decoys (pure noise) that match nothing, per trace; entries of the Dig that are unexpected ions");
    for tune in BENCH_PAIRS {
        let recall: Vec<String> = BENCH_LEVELS
            .iter()
            .enumerate()
            .map(|(i, &a)| {
                let (files, idxs) = bench_series(i);
                let t = tally(&dig_at(files, PARENT, DEFAULTS, idxs, Some(tune)));
                format!(
                    "{:.0}x {}|{}+{}w",
                    a / 3e3,
                    detected_tps(idxs, tune),
                    t.shown,
                    t.weak
                )
            })
            .collect();
        let (mut fp, mut traces, mut shown_unexpected) = (0, 0, 0);
        for i in 0..DECOY_SEEDS.len() {
            let (files, idxs) = decoy_series(i);
            let (n, f) = spurious(idxs, tune, false);
            fp += n;
            traces += f;
            let d = dig_at(files, PARENT, DEFAULTS, idxs, Some(tune));
            shown_unexpected += d
                .entries
                .iter()
                .filter(|e| e.unexpected && matches!(e.label, "forte" | "possibile"))
                .count();
        }
        println!(
            "floor {:.1e} min_height {:.1e} | recall {} | FP/trace {:.2} | pool ions shown per decoy (constant: not noise) {:.1}",
            tune.0,
            tune.1,
            recall.join(" ; "),
            fp as f64 / traces as f64,
            shown_unexpected as f64 / DECOY_SEEDS.len() as f64
        );
    }
}
