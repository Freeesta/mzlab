//! Classes of the rows `detect` emits for one `.mzidx`: points per trace, gaps, height, background flag, rows that share an ion within 5 ppm.
//! Used to see why a file gives many more rows than a chromatogram builder that needs consecutive scans.
//!
//!     cargo run --release -p mzlab-feat --example rowstats -- <file.mzidx>... [--floor X] [--min-height X] [--min-points N] [--max-gap N]

use mzlab_core::idxread::Index;
use mzlab_feat::table::{FileFeatures, FLAG_BACKGROUND};
use mzlab_feat::trace::{detect, Params};

fn bucket(v: usize, edges: &[usize]) -> usize {
    edges.iter().position(|&e| v < e).unwrap_or(edges.len())
}

/// Rows that share an ion with a neighbour, and the classes of those pairs.
fn pairs(r: &FileFeatures) {
    let n = r.mz.len();
    // Rows of one ion: within 5 ppm of a neighbour (rows are in m/z order) and overlapping in scans. Each such pair is classed
    // by its distance, by the height ratio and by the overlap of the scans (share of the shorter trace).
    let mut shared = vec![false; n];
    let (mut by_ppm, mut by_ratio, mut by_ov, mut pairs) =
        ([0usize; 4], [0usize; 4], [0usize; 4], 0usize);
    for i in 0..n {
        let mut j = i + 1;
        while j < n && (r.mz[j] - r.mz[i]) / r.mz[i] * 1e6 <= 5.0 {
            if r.scan_lo[i] <= r.scan_hi[j] && r.scan_lo[j] <= r.scan_hi[i] {
                shared[i] = true;
                shared[j] = true;
                pairs += 1;
                let ppm = (r.mz[j] - r.mz[i]) / r.mz[i] * 1e6;
                by_ppm[bucket((ppm * 10.0) as usize, &[10, 20, 40])] += 1;
                let (hi, lo) = (r.height[i].max(r.height[j]), r.height[i].min(r.height[j]));
                by_ratio[bucket((hi / lo * 10.0) as usize, &[20, 100, 1000])] += 1;
                let ov = r.scan_hi[i].min(r.scan_hi[j]) as f64
                    - r.scan_lo[i].max(r.scan_lo[j]) as f64
                    + 1.0;
                let short =
                    (r.scan_hi[i] - r.scan_lo[i]).min(r.scan_hi[j] - r.scan_lo[j]) as f64 + 1.0;
                by_ov[bucket((ov / short * 10.0) as usize, &[2, 5, 10])] += 1;
            }
            j += 1;
        }
    }
    let sh = shared.iter().filter(|s| **s).count();
    println!("  rows sharing an ion (within 5 ppm, overlapping scans) {sh}");
    println!(
        "  pairs {pairs}: ppm <1 {} 1-2 {} 2-4 {} 4-5 {}",
        by_ppm[0], by_ppm[1], by_ppm[2], by_ppm[3]
    );
    println!(
        "  pairs by height ratio: <2 {} 2-10 {} 10-100 {} >100 {}",
        by_ratio[0], by_ratio[1], by_ratio[2], by_ratio[3]
    );
    println!(
        "  pairs by scan overlap: <20% {} 20-50% {} 50-99% {} full {}",
        by_ov[0], by_ov[1], by_ov[2], by_ov[3]
    );
}

/// Prints the classes of the rows `detect` finds in one index.
fn analyse(path: &str, p: &Params) {
    let f = std::fs::File::open(path).expect("open");
    let size = f.metadata().expect("size").len();
    let mut ix = Index::open(f, size).unwrap_or_else(|e| panic!("{path}: {e}"));
    let t = std::time::Instant::now();
    let r = detect(&mut ix, p).unwrap_or_else(|e| panic!("{path}: {e}"));
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
    println!(
        "  points: <5 {}  5 {}  6-9 {}  10-19 {}  20-49 {}  50+ {}",
        by_n[0], by_n[1], by_n[2], by_n[3], by_n[4], by_n[5]
    );
    println!("  no gap in the scans {consecutive}  no gap and >=5 scans {consec5}  background flag {long}");
    println!(
        "  height / min_height: <1.5 {}  1.5-3 {}  3-10 {}  10-100 {}  >100 {}  (<2: {weak_h})",
        by_h[0], by_h[1], by_h[2], by_h[3], by_h[4]
    );
    pairs(&r);
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
        analyse(path, &p);
    }
}
