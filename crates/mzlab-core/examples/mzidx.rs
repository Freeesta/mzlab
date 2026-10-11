//! Builds the `.mzidx` of an mzML: `cargo run --release --example mzidx -- FILE.mzML [OUT.mzidx] [BUDGET_MB]`.
//! Prints the time and the peak memory of the process. The scan table is a plain one (level, polarity, RT, summed intensity as TIC):
//! the browser gets the real one from the Python reader.
use std::fs::File;
use std::time::Instant;

use mzlab_core::mzidx::{build_mzml, ScanTable};
use mzlab_core::mzml::{parse, Item};

fn peak_rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("VmHWM:")).map(|l| l.to_string()))
        .and_then(|l| l.split_whitespace().nth(1).and_then(|v| v.parse::<f64>().ok()))
        .map(|kb| kb / 1024.0)
        .unwrap_or(f64::NAN)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: mzidx FILE.mzML [OUT.mzidx] [BUDGET_MB]");
    let out_path = args.next().unwrap_or_else(|| format!("{path}.mzidx"));
    let budget = args.next().and_then(|v| v.parse::<usize>().ok()).unwrap_or(64) << 20;

    let t0 = Instant::now();
    let mut table = ScanTable { scans_json: b"[]".to_vec(), source_json: "{}".into(), run_json: "{}".into(), ..Default::default() };
    parse(File::open(&path).expect("file"), |item| {
        if let Item::Spectrum(s) = item {
            table.level.push(s.level);
            table.polarity.push(s.polarity);
            table.rt.push(s.rt);
            table.tic.push(s.intensity.iter().map(|&v| v as f64).sum());
        }
    })
    .expect("mzML");
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
