//! Times the scan table and the `.mzidx` of a made-up reader (`tests/lettore_finto`): the work of `raw_tempi` without a real file.
//! `cargo run --release --example mzidx_tempi -p mzlab-core -- [SCANS] [PEAKS_PER_SCAN] [BUDGET_MB]`
#[path = "../tests/lettore_finto/mod.rs"]
mod finto;

use std::time::Instant;

use mzlab_core::lettori::Lettore;
use mzlab_core::{mzidx, upload};

/// The index goes to a temporary file, as on the desktop (a sparse file: nothing is zeroed in memory); `sha` is the SHA-256 of what was written (a change
/// of speed must not change a byte).
struct Nulla(std::fs::File);
impl Default for Nulla {
    fn default() -> Self {
        let f = tempfile_in_tmp();
        Nulla(f)
    }
}
fn tempfile_in_tmp() -> std::fs::File {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("mzidx_tempi_{}_{n}.out", std::process::id()));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("output");
    let _ = std::fs::remove_file(&path);
    f
}
impl mzidx::Output for Nulla {
    fn write_at(&mut self, o: u64, d: &[u8]) -> mzlab_core::Result<()> {
        use std::io::{Seek, SeekFrom, Write};
        if std::env::var_os("DISCARD").is_some() {
            return Ok(());
        }
        self.0.seek(SeekFrom::Start(o)).expect("seek");
        self.0.write_all(d).expect("write");
        Ok(())
    }
}
impl Nulla {
    fn sha(&mut self) -> String {
        use sha2::Digest as _;
        use std::io::{Read, Seek, SeekFrom};
        self.0.seek(SeekFrom::Start(0)).expect("seek");
        let mut h = sha2::Sha256::new();
        let mut b = vec![0u8; 1 << 20];
        loop {
            let n = self.0.read(&mut b).expect("read");
            if n == 0 {
                break;
            }
            h.update(&b[..n]);
        }
        h.finalize()
            .iter()
            .take(6)
            .map(|b| format!("{b:02x}"))
            .collect()
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
    // `SOLO=scratch`: only the run with a scratch file (the way the site and the desktop build the index), for a profiler
    if std::env::var("SOLO").as_deref() == Ok("scratch") {
        con_scratch(&mut l, budget);
        return;
    }
    let t = Instant::now();
    let mut out = Nulla::default();
    let p = upload::prepare_lettore(&mut l, 0, &imp, "x.mzML").expect("table");
    println!(
        "scan table                 {:>8.3} s",
        t.elapsed().as_secs_f64()
    );
    let t = Instant::now();
    let st = mzidx::build_lettore(&p.table, &mut l, 0, &mut out, budget << 20).expect("index");
    println!(
        ".mzidx                     {:>8.3} s ({:.1} MB, {} passes, sha256 {})",
        t.elapsed().as_secs_f64(),
        st.bytes as f64 / 1e6,
        st.passes,
        out.sha()
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
    let mut out = Nulla::default();
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
        "table + .mzidx + scratch   {:>8.3} s ({:.1} MB, {} passes, scratch {:.1} MB, sha256 {})",
        t.elapsed().as_secs_f64(),
        st.bytes as f64 / 1e6,
        st.passes,
        sp.metadata().map_or(0.0, |m| m.len() as f64 / 1e6),
        out.sha()
    );
    drop(sp);
    let _ = std::fs::remove_file(&path);
}
