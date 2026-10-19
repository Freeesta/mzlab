//! Isotope finder (`untargeted::isofinder`) against the «Isotopic peaks finder» of mzmine 4.10.90 on the four synthetic files: for each resolved feature of
//! `isotopes_finder.csv` the spectrum of its most intense scan (centroids above the noise of the batch) goes through `select::detect`; the charge and the
//! number of peaks of the pattern are compared. The mzML files are written again by `tools/bench/oro_mzmine.py --genera` (skipped without python3 and numpy).
#![cfg(feature = "untargeted")]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_feat::untargeted::adap::AdapParams;
use mzlab_feat::untargeted::isofinder::select::detect;
use mzlab_feat::untargeted::isofinder::{Config, Engine};

mod untargeted_common;

use untargeted_common::scans_of;

const GOLD: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/data/untargeted/mzmine410"
);

struct Row {
    file: String,
    mz: f64,
    rt: f64,
    charge: Option<i32>,
    peaks: Option<usize>,
}

fn rows() -> Vec<Row> {
    let text = std::fs::read_to_string(Path::new(GOLD).join("isotopes_finder.csv")).unwrap();
    text.lines()
        .skip(1)
        .map(|l| {
            let c: Vec<&str> = l.split(',').collect();
            Row {
                file: c[0].to_string(),
                mz: c[2].parse().unwrap(),
                rt: c[3].parse().unwrap(),
                charge: c[5].parse().ok(),
                peaks: c[6].parse().ok(),
            }
        })
        .collect()
}

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
fn charge_and_pattern_like_mzmine_410() {
    if !Path::new(GOLD).exists() {
        eprintln!(
            "SKIP charge_and_pattern_like_mzmine_410: tests/data/untargeted/mzmine410 is missing"
        );
        return;
    }
    let Some(dir) = generate() else {
        eprintln!("SKIP untargeted_finder: python3 with numpy is missing");
        return;
    };
    let engine = Engine::new(Config::default()).unwrap();
    let p = AdapParams::default();
    let (mut total, mut same_charge, mut same_peaks) = (0usize, 0usize, 0usize);
    let mut off = Vec::new();
    for file in ["oro_1", "oro_2", "oro_3", "oro_4"] {
        let scans = scans_of(&dir.join(format!("{file}.mzML")), &p);
        for r in rows().iter().filter(|r| r.file == file) {
            let scan = scans
                .iter()
                .min_by(|a, b| (a.rt - r.rt).abs().total_cmp(&(b.rt - r.rt).abs()))
                .unwrap();
            let y: Vec<f64> = scan.int.iter().map(|v| *v as f64).collect();
            let got = detect(&engine, &scan.mz, &y, r.mz, 1);
            let (charge, peaks) = (
                got.as_ref().map(|d| d.best_charge),
                got.as_ref().map(|d| d.patterns[0].peaks.len()),
            );
            total += 1;
            same_charge += usize::from(charge == r.charge);
            same_peaks += usize::from(charge == r.charge && peaks == r.peaks);
            if charge != r.charge || peaks != r.peaks {
                off.push(format!(
                    "{file} m/z {} RT {}: charge {charge:?} peaks {peaks:?} vs {:?} {:?}",
                    r.mz, r.rt, r.charge, r.peaks
                ));
            }
        }
    }
    eprintln!("finder: {same_charge}/{total} same charge, {same_peaks}/{total} same charge and peaks; different: {off:#?}");
    let with_charge = rows().iter().filter(|r| r.charge.is_some()).count();
    assert!(
        with_charge >= 20,
        "the gold data has too few patterns: {with_charge}"
    );
    assert!(same_peaks as f64 >= 0.99 * total as f64, "{off:#?}");
}
