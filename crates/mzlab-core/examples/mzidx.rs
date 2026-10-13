//! Builds the `.mzidx` of an mzML: `cargo run --release --example mzidx -- FILE.mzML [OUT.mzidx] [BUDGET_MB]`.
//! Prints the time and the peak memory of the process. The scan table is the one of the browser (`upload::prepare`).
use std::fs::File;
use std::time::Instant;

use mzlab_core::mzidx::build_mzml;
use mzlab_core::upload;

fn peak_rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmHWM:"))
                .map(|l| l.to_string())
        })
        .and_then(|l| {
            l.split_whitespace()
                .nth(1)
                .and_then(|v| v.parse::<f64>().ok())
        })
        .map(|kb| kb / 1024.0)
        .unwrap_or(f64::NAN)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: mzidx FILE.mzML [OUT.mzidx] [BUDGET_MB]");
    let out_path = args.next().unwrap_or_else(|| format!("{path}.mzidx"));
    let budget = args
        .next()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(64)
        << 20;

    let t0 = Instant::now();
    // the same scan table as the browser (`prepare_upload`): levels, polarity, RT, TIC, the run flags (`hr1`) and the source fingerprints
    let size = std::fs::metadata(&path).expect("size").len();
    let name = std::path::Path::new(&path)
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let table = upload::prepare(&mut File::open(&path).expect("file"), size, &name)
        .expect("scan table")
        .table;
    let t_table = t0.elapsed().as_secs_f64();

    let t1 = Instant::now();
    let mut out = File::create(&out_path).expect("output");
    let st = build_mzml(&table, File::open(&path).expect("file"), &mut out, budget).expect("build");
    println!(
        "{} scans, {} peaks, {} sections, {} passes, {:.1} MB; table pass {:.2} s, build {:.2} s, peak memory {:.0} MB",
        st.scans,
        st.peaks,
        st.sections,
        st.passes,
        st.bytes as f64 / 1e6,
        t_table,
        t1.elapsed().as_secs_f64(),
        peak_rss_mb()
    );
}
