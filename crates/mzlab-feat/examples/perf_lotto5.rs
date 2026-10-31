//! Time, peak memory and SHA-256 of the output of `untargeted::pipeline::run_file` on a synthetic run, and of the broad-band search on a long trace.
//!
//!     cargo run --release -p mzlab-feat --features untargeted --example perf_lotto5 -- gen <scans> <ions> <out.mzidx>
//!     cargo run --release -p mzlab-feat --features untargeted --example perf_lotto5 -- run <file.mzidx> <broad 0|1> <gt 0|1> [baseline 0|1] [slab_mb]
//!     cargo run --release -p mzlab-feat --features untargeted --example perf_lotto5 -- trace <scans>

#[path = "../tests/untargeted_common/synth.rs"]
mod synth;

use std::time::Instant;

use mzlab_core::idxread::Index;
use mzlab_feat::broad::{detect_broad, BroadParams};
use mzlab_feat::untargeted::pipeline::{run_file, BaselineParams, UntParams};
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
    let a: Vec<String> = std::env::args().skip(1).collect();
    match a.first().map(String::as_str) {
        Some("gen") => {
            let (scans, ions) = (a[1].parse().unwrap(), a[2].parse().unwrap());
            let bytes = synth::index_bytes(scans, ions);
            std::fs::write(&a[3], &bytes).unwrap();
            println!("{} MB", bytes.len() / (1 << 20));
        }
        Some("run") => {
            let flag = |i: usize| a.get(i).is_some_and(|x| x == "1");
            let mut p = UntParams::default();
            p.broad.enabled = flag(2);
            p.broad.gt = flag(3);
            p.baseline = flag(4).then(BaselineParams::default);
            if let Some(mb) = a.get(5) {
                p.slab_bytes = mb.parse::<usize>().unwrap() << 20;
            }
            let file = std::fs::File::open(&a[1]).unwrap();
            let size = file.metadata().unwrap().len();
            let mut ix = Index::open(file, size).unwrap();
            let t = Instant::now();
            let list = run_file(&mut ix, &p).unwrap();
            let secs = t.elapsed().as_secs_f64();
            let mut h = Sha256::new();
            for f in &list {
                for v in [f.mz, f.rt, f.rt_start, f.rt_end, f.height, f.area] {
                    h.update(v.to_bits().to_le_bytes());
                }
                h.update([f.broad as u8]);
            }
            println!(
                "run_file {secs:.3} s  rss {:.0} MB  features {} (broad {})  sha {}",
                peak_rss_mb(),
                list.len(),
                list.iter().filter(|f| f.broad).count(),
                h.finalize()
                    .iter()
                    .take(8)
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            );
        }
        Some("trace") => {
            let n: usize = a[1].parse().unwrap();
            let rt: Vec<f64> = (0..n).map(|i| i as f64 * 0.5).collect();
            let mz = vec![300.0; n];
            let y: Vec<f64> = (0..n)
                .map(|i| {
                    let t = i as f64;
                    3e3 * libm::exp(-0.5 * ((t - n as f64 / 3.0) / 120.0).powi(2))
                        + 2e3 * libm::exp(-0.5 * ((t - n as f64 * 0.7) / 200.0).powi(2))
                        + 40.0 * libm::sin(t * 1.7)
                })
                .collect();
            let p = BroadParams {
                enabled: true,
                ..BroadParams::default()
            };
            let t = Instant::now();
            let mut bands = Vec::new();
            for _ in 0..5 {
                bands = detect_broad(&rt, &mz, &y, &[], &p).unwrap();
            }
            println!(
                "detect_broad {n} scans: {:.3} s/call  bands {}",
                t.elapsed().as_secs_f64() / 5.0,
                bands.len()
            );
        }
        _ => eprintln!("gen | run | trace"),
    }
}
