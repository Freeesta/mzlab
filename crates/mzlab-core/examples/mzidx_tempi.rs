//! Times the scan table and the `.mzidx` of a made-up reader (`tests/lettore_finto`): the work of `raw_tempi` without a real file.
//! `cargo run --release --example mzidx_tempi -p mzlab-core -- [SCANS] [PEAKS_PER_SCAN] [BUDGET_MB]`
#[path = "../tests/lettore_finto/mod.rs"]
mod finto;

use std::time::Instant;

use mzlab_core::lettori::Lettore;
use mzlab_core::{mzidx, upload};

/// Discards what is written (the benchmark is not about the memory of the sink).
struct Nulla;
impl mzidx::Output for Nulla {
    fn write_at(&mut self, _o: u64, _d: &[u8]) -> mzlab_core::Result<()> {
        Ok(())
    }
}

fn main() {
    let a: Vec<usize> = std::env::args()
        .skip(1)
        .filter_map(|s| s.parse().ok())
        .collect();
    let (n, peaks, budget) = (
        *a.first().unwrap_or(&20000),
        *a.get(1).unwrap_or(&2000),
        *a.get(2).unwrap_or(&192),
    );
    let mut l = finto::Finto::vario(n, peaks, 12345);
    let np: usize = l.schede.iter().map(|s| s.spettro.mz.len()).sum();
    println!("{n} scans, {np} peaks, budget {budget} MB");
    let t = Instant::now();
    let mut s = 0usize;
    for k in 0..n {
        s += l.scansione(0, k).unwrap().mz.len();
    }
    println!(
        "decode every scan          {:>8.3} s ({s} peaks)",
        t.elapsed().as_secs_f64()
    );
    let imp = vec![];
    let t = Instant::now();
    let mut out = Nulla;
    let p = upload::prepare_lettore(&mut l, 0, &imp, "x.mzML").expect("table");
    println!(
        "scan table                 {:>8.3} s",
        t.elapsed().as_secs_f64()
    );
    let t = Instant::now();
    let st = mzidx::build_lettore(&p.table, &mut l, 0, &mut out, budget << 20).expect("index");
    println!(
        ".mzidx                     {:>8.3} s ({:.1} MB, {} passes)",
        t.elapsed().as_secs_f64(),
        st.bytes as f64 / 1e6,
        st.passes
    );
    let t = Instant::now();
    let st = upload::indicizza_lettore(
        &mut l,
        0,
        &imp,
        "x.mzML",
        &mut out,
        budget << 20,
        &mut |_| {},
    )
    .expect("index")
    .0;
    println!(
        "table + .mzidx (one call)  {:>8.3} s ({:.1} MB, {} passes)",
        t.elapsed().as_secs_f64(),
        st.bytes as f64 / 1e6,
        st.passes
    );
    con_scratch(&mut l, budget);
}

/// The same with a scratch file (a temporary file): the scans are decoded once whatever the budget.
fn con_scratch(l: &mut finto::Finto, budget: usize) {
    let mut out = Nulla;
    let path = std::env::temp_dir().join(format!("mzidx_tempi_{}.sp", std::process::id()));
    let mut sp = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("scratch");
    let t = Instant::now();
    let st = upload::indicizza_lettore_con(
        l,
        0,
        &mut || Ok(vec![]),
        "x.mzML",
        &mut out,
        budget << 20,
        Some(&mut sp),
        &mut |_| {},
    )
    .expect("index")
    .0;
    println!(
        "table + .mzidx + scratch   {:>8.3} s ({:.1} MB, {} passes, scratch {:.1} MB)",
        t.elapsed().as_secs_f64(),
        st.bytes as f64 / 1e6,
        st.passes,
        sp.metadata().map_or(0.0, |m| m.len() as f64 / 1e6)
    );
    drop(sp);
    let _ = std::fs::remove_file(&path);
}
