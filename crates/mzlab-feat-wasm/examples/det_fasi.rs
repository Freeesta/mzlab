//! Seconds of each part of the `features.detect` step of `flow run` on one index (bench of the step, not a test):
//!
//!     cargo run --release -p mzlab-feat-wasm --example det_fasi -- FILE.mzidx [--rounds N]
//!
//! Prints the best of N rounds (default 5) of: the recipe with the estimated minimum height, the detection with families (`run::detect`) and the
//! weak candidates (`run::detect_weak`), and the SHA-256 of the two outputs so a change can be shown not to move them.

use std::fs::File;
use std::time::Instant;

use mzlab_core::idxread::Index;
use mzlab_feat_wasm::run;
use sha2::{Digest, Sha256};

fn open(path: &str) -> Index<File> {
    let f = File::open(path).expect("open");
    let size = f.metadata().expect("size").len();
    Index::open(f, size).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn sha(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args.first().expect("FILE.mzidx");
    let rounds = args
        .iter()
        .position(|a| a == "--rounds")
        .and_then(|k| args.get(k + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    let mut best = [f64::INFINITY; 4];
    let (mut s1, mut s2) = (String::new(), String::new());
    for _ in 0..rounds {
        let t0 = Instant::now();
        let mut ix = open(path);
        let t_open = t0.elapsed().as_secs_f64();
        let t = Instant::now();
        let recipe = run::recipe_for_estimated("", &mut ix).expect("recipe");
        let t_recipe = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let bytes = run::detect(&mut ix, "x", &recipe).expect("detect");
        let t_det = t.elapsed().as_secs_f64();
        let t = Instant::now();
        let weak = run::detect_weak(&mut ix, &recipe).expect("weak");
        let t_weak = t.elapsed().as_secs_f64();
        for (b, v) in best.iter_mut().zip([t_open, t_recipe, t_det, t_weak]) {
            *b = b.min(v);
        }
        (s1, s2) = (sha(&bytes), sha(&weak));
    }
    println!(
        "open {:.3} s  recipe {:.3} s  detect {:.3} s  weak {:.3} s  (best of {rounds})",
        best[0], best[1], best[2], best[3]
    );
    println!("mzfeatf {s1}\nweak    {s2}");
    mzlab_feat::prof::report("summed over the rounds:");
}
