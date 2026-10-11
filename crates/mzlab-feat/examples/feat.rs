//! Time, peak memory, number of features and SHA-256 of the output of `detect` on one or more `.mzidx` files.
//!
//!     cargo run --release -p mzlab-feat --example feat -- <file.mzidx>... [--neg] [--budget-mb N]

use std::time::Instant;

use mzlab_core::idxread::Index;
use mzlab_feat::trace::{detect, Params};
use sha2::{Digest, Sha256};

/// Peak resident memory of the process, MB (Linux `VmHWM`; 0 elsewhere).
fn peak_rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmHWM:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok())
        })
        .map_or(0.0, |kb| kb / 1024.0)
}

fn main() {
    let mut p = Params::default();
    let mut files = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--neg" => p.polarity = Some(-1),
            "--budget-mb" => {
                p.budget = args
                    .next()
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(96)
                    << 20
            }
            _ => files.push(a),
        }
    }
    let mut total = 0;
    for path in &files {
        let f = std::fs::File::open(path).expect("open");
        let size = f.metadata().expect("size").len();
        let mut ix = Index::open(f, size).unwrap_or_else(|e| panic!("{path}: {e}"));
        let t0 = Instant::now();
        let feats = detect(&mut ix, &p).unwrap_or_else(|e| panic!("{path}: {e}"));
        let secs = t0.elapsed().as_secs_f64();
        let sha = Sha256::digest(feats.to_bytes());
        let hex: String = sha.iter().map(|b| format!("{b:02x}")).collect();
        println!(
            "{path}\t{:.2} s\tpeak {:.0} MB\t{} features\t{hex}",
            secs,
            peak_rss_mb(),
            feats.len()
        );
        total += feats.len();
    }
    println!("total\t{total} features");
}
