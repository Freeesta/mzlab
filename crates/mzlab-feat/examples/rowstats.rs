//! Classes of the rows `detect` emits for one `.mzidx`: points per trace, gaps, height, background flag, rows that share an ion within 5 ppm.
//! Used to see why a file gives many more rows than a chromatogram builder that needs consecutive scans.
//!
//!     cargo run --release -p mzlab-feat --example rowstats -- <file.mzidx>... [--floor X] [--min-height X] [--min-points N] [--max-gap N]

use mzlab_core::idxread::Index;
use mzlab_feat::table::FLAG_BACKGROUND;
use mzlab_feat::trace::{detect, Params};

fn bucket(v: usize, edges: &[usize]) -> usize {
    edges.iter().position(|&e| v < e).unwrap_or(edges.len())
}

fn main() {
    let mut p = Params::default();
    let mut files = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut num = || {
            args.next()
                .and_then(|v| v.parse::<f64>().ok())
                .expect("number")
        };
        match a.as_str() {
            "--floor" => p.floor = num(),
            "--min-height" => p.min_height = num(),
            "--min-points" => p.min_points = num() as u32,
            "--max-gap" => p.max_gap = num() as u32,
            _ => files.push(a),
        }
    }
    for path in &files {
        let f = std::fs::File::open(path).expect("open");
        let size = f.metadata().expect("size").len();
        let mut ix = Index::open(f, size).unwrap_or_else(|e| panic!("{path}: {e}"));
        let t = std::time::Instant::now();
        let r = detect(&mut ix, &p).unwrap_or_else(|e| panic!("{path}: {e}"));
        let n = r.mz.len();
        println!(
            "{path}: {n} rows in {:.1} s (floor {}, min_height {}, min_points {}, max_gap {})",
            t.elapsed().as_secs_f64(),
            p.floor,
            p.min_height,
            p.min_points,
            p.max_gap
        );
        let mut by_n = [0usize; 6];
        let (mut consecutive, mut consec5, mut long, mut weak_h) = (0, 0, 0, 0);
        let mut by_h = [0usize; 5];
        for i in 0..n {
            let pts = r.npts[i] as usize;
            by_n[bucket(pts, &[5, 6, 10, 20, 50])] += 1;
            let span = (r.scan_hi[i] - r.scan_lo[i] + 1) as usize;
            if span == pts {
                consecutive += 1;
                if pts >= 5 {
                    consec5 += 1;
                }
            }
            if r.flags[i] & FLAG_BACKGROUND != 0 {
                long += 1;
            }
            let h = r.height[i] as f64 / p.min_height;
            by_h[bucket((h * 10.0) as usize, &[15, 30, 100, 1000])] += 1;
            if h < 2.0 {
                weak_h += 1;
            }
        }
        // Rows of one ion: within 5 ppm of a neighbour (rows are in m/z order) and overlapping in scans.
        let mut shared = vec![false; n];
        for i in 0..n {
            let mut j = i + 1;
            while j < n && (r.mz[j] - r.mz[i]) / r.mz[i] * 1e6 <= 5.0 {
                if r.scan_lo[i] <= r.scan_hi[j] && r.scan_lo[j] <= r.scan_hi[i] {
                    shared[i] = true;
                    shared[j] = true;
                }
                j += 1;
            }
        }
        let sh = shared.iter().filter(|s| **s).count();
        println!(
            "  points: <5 {}  5 {}  6-9 {}  10-19 {}  20-49 {}  50+ {}",
            by_n[0], by_n[1], by_n[2], by_n[3], by_n[4], by_n[5]
        );
        println!("  no gap in the scans {consecutive}  no gap and >=5 scans {consec5}  background flag {long}");
        println!(
            "  height / min_height: <1.5 {}  1.5-3 {}  3-10 {}  10-100 {}  >100 {}  (<2: {weak_h})",
            by_h[0], by_h[1], by_h[2], by_h[3], by_h[4]
        );
        println!("  rows sharing an ion (within 5 ppm, overlapping scans) {sh}");
    }
}
