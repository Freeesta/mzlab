//! ADAP chromatogram builder (`untargeted::adap`) against mzmine 3.9.0: parity per chromatogram on the four synthetic files, hand-made cases, and the
//! independence from the slabs. The mzML files are written again by `tools/bench/oro_mzmine.py --genera` (skipped without python3 and numpy).
//! The reference CSV keeps 5 decimals of m/z and 4 significant digits of height, so those are compared at the precision they have.
#![cfg(feature = "untargeted")]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_feat::untargeted::adap::{build, build_slabs, slab_count, AdapParams, Ms1Scan};

mod untargeted_common;

use untargeted_common::{rows, same, scans_of, shape, Row};

const GOLD: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/data/untargeted/mzmine39/adap.csv"
);

/// The mzML of the gold run, or `None` when python3 or numpy are missing.
fn generate() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("untargeted_oro");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ok = Command::new("python3")
        .arg(root.join("tools/bench/oro_mzmine.py"))
        .arg("--genera")
        .arg(&dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    ok.then_some(dir)
}

#[test]
fn parity_with_mzmine39() {
    let Some(dir) = generate() else {
        eprintln!("SKIP untargeted_adap: python3 with numpy is missing");
        return;
    };
    let p = AdapParams::default();
    let rows = rows(Path::new(GOLD));
    let (mut total, mut equal, mut bad) = (0usize, 0usize, Vec::new());
    for file in ["oro_1", "oro_2", "oro_3", "oro_4"] {
        let scans = scans_of(&dir.join(format!("{file}.mzML")), &p);
        let ours = build(&scans, &p);
        let refs: Vec<&Row> = rows.iter().filter(|r| r.file == file).collect();
        let mut used = vec![false; ours.len()];
        for r in &refs {
            total += 1;
            let cand = ours.iter().enumerate().find(|(k, c)| {
                !used[*k] && (c.mz - r.mz).abs() < 5e-3 && (shape(c, &scans).0 - r.rt).abs() < 0.02
            });
            match cand {
                Some((k, c)) => match same(r, c, &scans, 1e-5, 6e-4) {
                    Ok(()) => {
                        used[k] = true;
                        equal += 1
                    }
                    Err(w) => bad.push(format!("{file} m/z {}: {w}", r.mz)),
                },
                None => bad.push(format!(
                    "{file} m/z {} RT {}: no chromatogram of ours",
                    r.mz, r.rt
                )),
            }
        }
        let extra = used.iter().filter(|u| !**u).count();
        if extra > 0 {
            bad.push(format!(
                "{file}: {extra} chromatograms of ours without a reference row"
            ));
        }
    }
    eprintln!("ADAP parity: {equal}/{total} identical; different: {bad:#?}");
    assert!(equal as f64 >= 0.99 * total as f64, "{bad:#?}");
}

fn run_of(n: usize, f: impl Fn(usize) -> Vec<(f64, f32)>) -> Vec<Ms1Scan> {
    (0..n)
        .map(|k| {
            let mut pts = f(k);
            pts.sort_by(|a, b| a.0.total_cmp(&b.0));
            Ms1Scan {
                index: k,
                rt: k as f64 * 0.01,
                mz: pts.iter().map(|x| x.0).collect(),
                int: pts.iter().map(|x| x.1).collect(),
            }
        })
        .collect()
}

#[test]
fn equal_intensity_keeps_the_higher_mz_of_a_scan() {
    // 100.0005 and 100.0 have the same intensity in scan 3: the sort puts the higher m/z first, and a scan holds one point per chromatogram.
    let scans = run_of(8, |k| {
        if k == 3 {
            vec![(100.0, 5e4), (100.0005, 5e4)]
        } else {
            vec![(100.0, 5e4)]
        }
    });
    let c = build(&scans, &AdapParams::default());
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].points, 8);
    assert!(
        (c[0].mz_mean - (7.0 * 100.0 + 100.0005) / 8.0).abs() < 1e-9,
        "{}",
        c[0].mz_mean
    );
}

#[test]
fn interval_is_cut_by_its_neighbour() {
    // A (200.000, scans 0-5) opens [199.998, 200.002). B (200.0035) would reach down to 200.0015, so it starts at 200.002; the point at 200.0019 in scan 6 is A's.
    let scans = run_of(8, |k| {
        let mut v = Vec::new();
        if k < 6 {
            v.push((200.0, 9e4));
            v.push((200.0035, 5e4));
        }
        if k == 6 {
            v.push((200.0019, 4e4));
        }
        v
    });
    let c = build(&scans, &AdapParams::default());
    assert_eq!(c.len(), 2);
    assert_eq!((c[0].points, c[1].points), (7, 6));
    assert!((c[1].mz - 200.0035).abs() < 1e-9);
}

#[test]
fn a_hole_does_not_split_and_gets_a_zero() {
    let scans = run_of(12, |k| if k == 5 { vec![] } else { vec![(300.0, 5e4)] });
    let c = build(&scans, &AdapParams::default());
    assert_eq!(c.len(), 1);
    assert_eq!(c[0].points, 11);
    assert_eq!(c[0].scan, (0..12).collect::<Vec<u32>>());
    assert_eq!(c[0].int[5], 0.0);
}

#[test]
fn zeros_flank_every_run_of_detected_scans() {
    // Detected 2-6 and 9-13 of 16 scans: zeros at 1, 7, 8, 14 (a gap of two gives a zero on each side of it).
    let scans = run_of(16, |k| {
        if (2..=6).contains(&k) || (9..=13).contains(&k) {
            vec![(300.0, 5e4)]
        } else {
            vec![]
        }
    });
    let c = build(&scans, &AdapParams::default());
    let zeros: Vec<u32> = c[0]
        .scan
        .iter()
        .zip(&c[0].int)
        .filter(|(_, i)| **i == 0.0)
        .map(|(s, _)| *s)
        .collect();
    assert_eq!(zeros, vec![1, 7, 8, 14]);
}

#[test]
fn short_segments_are_dropped() {
    let p = AdapParams::default();
    let short = run_of(12, |k| if k < 4 { vec![(300.0, 5e4)] } else { vec![] });
    assert!(build(&short, &p).is_empty());
    // 4 + hole + 4: no run of 5 consecutive scans.
    let broken = run_of(12, |k| {
        if k == 4 || k > 8 {
            vec![]
        } else {
            vec![(300.0, 5e4)]
        }
    });
    assert!(build(&broken, &p).is_empty());
    // A point under the minimum height never opens a chromatogram.
    let weak = run_of(12, |_| vec![(300.0, 2.5e4)]);
    assert!(build(&weak, &p).is_empty());
}

#[test]
fn highest_point_is_not_reset_between_runs() {
    // One point of 5e4, a hole, then five of 2e4: the run of 5 is kept because the highest point seen so far (5e4) counts, as in the Java.
    let scans = run_of(8, |k| match k {
        0 => vec![(400.0, 5e4)],
        1 => vec![],
        _ => vec![(400.0, 2e4)],
    });
    assert_eq!(build(&scans, &AdapParams::default()).len(), 1);
}

/// 600 scans of about 400 points: 150 traces and the rest noise, m/z spread over 100-1000 (about 240 000 points).
fn big() -> Vec<Ms1Scan> {
    let mut x: u64 = 0x2545_F491_4F6C_DD1D;
    let mut rnd = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        (x >> 11) as f64 / (1u64 << 53) as f64
    };
    let traces: Vec<(f64, f64, f64)> = (0..150)
        .map(|_| {
            (
                100.0 + 900.0 * rnd(),
                60.0 + 480.0 * rnd(),
                1e5 * (0.5 + rnd()),
            )
        })
        .collect();
    let noise: Vec<Vec<(f64, f32)>> = (0..600)
        .map(|_| {
            (0..330)
                .map(|_| (100.0 + 900.0 * rnd(), (1e4 * (1.0 + 4.0 * rnd())) as f32))
                .collect()
        })
        .collect();
    run_of(600, |k| {
        let mut v = noise[k].clone();
        for (m, c, h) in &traces {
            let y = h * (-0.5 * ((k as f64 - c) / 12.0).powi(2)).exp();
            if y >= 1e4 {
                v.push((m + 0.0004 * (k % 3) as f64 - 0.0004, y as f32));
            }
        }
        v
    })
}

#[test]
fn slabs_give_the_same_chromatograms() {
    let scans = big();
    let points: usize = scans.iter().map(|s| s.mz.len()).sum();
    assert!(points >= 200_000, "{points}");
    let p = AdapParams::default();
    let whole = build(&scans, &p);
    assert!(whole.len() > 100, "{}", whole.len());
    let small = 1 << 20;
    assert!(slab_count(&scans, &p, small) > 5);
    assert_eq!(build_slabs(&scans, &p, small), whole);
    assert_eq!(build_slabs(&scans, &p, usize::MAX / 4), whole);
}
