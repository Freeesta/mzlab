//! Why is a file an infusion (or not)? `cargo run --release -p mzlab-core --example diagnosi_infusione -- FILE.mzML`
//! Prints scans per level, the RT range, every distinct MSn key with its scans and RT range, the TIC variation and the verdict of `infusione`.
//! For Federico's files: paste the output into the chat; nothing leaves the computer.
use std::collections::BTreeMap;
use std::fs::File;

use mzlab_core::idxread::Index;
use mzlab_core::infusione::{con_msn, infusione};
use mzlab_core::mzidx::build_mzml;
use mzlab_core::upload;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: diagnosi_infusione FILE.mzML");
    let name = std::path::Path::new(&path)
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let size = std::fs::metadata(&path).expect("size").len();
    let table = upload::prepare(&mut File::open(&path).expect("file"), size, &name)
        .expect("scan table")
        .table;
    let tmp = std::env::temp_dir().join(format!("diagnosi_{}.mzidx", std::process::id()));
    let mut out = File::create(&tmp).expect("tmp");
    build_mzml(&table, File::open(&path).expect("file"), &mut out, 64 << 20).expect("index");
    let isize = std::fs::metadata(&tmp).expect("size").len();
    let idx = Index::open(File::open(&tmp).expect("tmp"), isize).expect("open index");
    let _ = std::fs::remove_file(&tmp);
    let scans = &idx.scans;

    let mut levels: BTreeMap<u8, usize> = BTreeMap::new();
    for s in scans {
        *levels.entry(s.level).or_default() += 1;
    }
    let (lo, hi) = scans
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), s| {
            (a.min(s.rt), b.max(s.rt))
        });
    println!(
        "file: {name}\nscans: {} · RT {lo:.3}-{hi:.3} min",
        scans.len()
    );
    println!("scans per level: {levels:?}");

    // key of an MSn scan: the filter path with activation and energy, else the precursor
    let key = |s: &mzlab_core::idxread::Scan| match s.path.as_deref() {
        Some(p) if !p.is_empty() => p
            .iter()
            .map(|(m, a, ce)| format!("{}@{}{}", m.round_ties_even() as i64, a.to_lowercase(), ce))
            .collect::<Vec<_>>()
            .join(">"),
        _ => s.precursor.map_or(String::new(), |p| format!("{p:.4}")),
    };
    let mut keys: BTreeMap<String, (usize, f64, f64)> = BTreeMap::new();
    for s in scans.iter().filter(|s| s.level >= 2) {
        let e = keys
            .entry(key(s))
            .or_insert((0, f64::INFINITY, f64::NEG_INFINITY));
        e.0 += 1;
        e.1 = e.1.min(s.rt);
        e.2 = e.2.max(s.rt);
    }
    println!(
        "distinct MSn keys: {} (the rule accepts 2-512 with a median of at least 10 scans)",
        keys.len()
    );
    for (k, (n, a, b)) in keys.iter().take(60) {
        println!("  {n:>5} scans  RT {a:.3}-{b:.3}  {k}");
    }
    if keys.len() > 60 {
        println!("  ... {} more keys", keys.len() - 60);
    }

    // TIC of the MS1 scans (all scans without MS1): variation and number of peaks above a fifth of the maximum
    let tic: Vec<f64> = scans
        .iter()
        .filter(|s| s.level == 1)
        .map(|s| s.tic)
        .collect();
    let tic = if tic.is_empty() {
        scans.iter().map(|s| s.tic).collect()
    } else {
        tic
    };
    let n = tic.len() as f64;
    let mean = tic.iter().sum::<f64>() / n.max(1.0);
    let sd = (tic.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n.max(1.0)).sqrt();
    let max = tic.iter().cloned().fold(0.0, f64::max);
    let peaks = (1..tic.len().saturating_sub(1))
        .filter(|&i| tic[i] > tic[i - 1] && tic[i] >= tic[i + 1] && tic[i] > 0.2 * max)
        .count();
    println!(
        "TIC: CV {:.2} · {peaks} local maxima above 20 % of the maximum",
        if mean > 0.0 { sd / mean } else { 0.0 }
    );

    match infusione(scans, None, &name) {
        Some(i) => println!("verdict: infusion, reason {} · levels {} · nodes {} · kind in the Dati view: {}", i.motivo, i.livelli, i.nodi, if con_msn(scans, None, &name) { "infusion" } else { "not infusion (constant RT or MS1 only)" }),
        None => println!("verdict: not an infusion (no fixed precursors in blocks of at least two segments, no hint in the name)"),
    }
}
