//! The inputs of the detection estimate (`trace_stima`) and the native peak memory of the detection of each index, for the fit of its coefficients:
//!
//!     cargo run --release -p mzlab-feat-wasm --example stima_detect -- FILE.mzidx...
//!     cargo run --release -p mzlab-feat-wasm --example stima_detect -- --fixture OUT.mzidx
//!
//! One line per file: MS1 scans, peaks, peaks at or above the floor, the estimate (MB) and the growth of the resident memory while detecting (MB).
//! `--fixture` writes the index of `tests/dati/stima_detect.json` instead, for the test of the `.wasm` (`tests/test_stima_detect_wasm.py`).

use std::fs::File;

use mzlab_core::idxread::Index;
use mzlab_feat::trace_estimate;
use mzlab_feat_wasm::run;

fn kb(field: &str) -> f64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    s.lines()
        .find(|l| l.starts_with(field))
        .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
        .unwrap_or(0.0)
}

#[path = "../tests/estimate_common/mod.rs"]
mod common;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--fixture") {
        std::fs::write(&args[1], common::index(&common::spec())).expect("write");
        return;
    }
    for path in args {
        let f = File::open(&path).expect("open");
        let size = f.metadata().expect("size").len();
        let mut ix = Index::open(f, size).expect("index");
        let r = run::recipe_for_estimated("", &mut ix).expect("recipe");
        let x = trace_estimate::inputs(&mut ix, &r).expect("inputs");
        let est = trace_estimate::bytes(&x, &r) as f64 / 1048576.0;
        // the high-water mark starts again from the memory in use now (Linux: 5 > clear_refs)
        let _ = std::fs::write("/proc/self/clear_refs", "5");
        let base = kb("VmRSS:");
        let out = run::detect(&mut ix, "x", &r).expect("detect");
        let peak = kb("VmHWM:");
        let weak = run::detect_weak(&mut ix, &r).expect("weak");
        let peak_weak = kb("VmHWM:");
        println!(
            "{path}\tscans {}\tpeaks {}\tkept {}\test {est:.1}\tdetect {:.1}\twith_weak {:.1}\tout {} {}",
            x.scans,
            x.peaks,
            x.kept,
            (peak - base) / 1024.0,
            (peak_weak - base) / 1024.0,
            out.len(),
            weak.len()
        );
    }
}
