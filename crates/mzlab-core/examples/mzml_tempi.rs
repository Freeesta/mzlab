//! Times the load of an mzML (scan table, fingerprints, `.mzidx`) and its writing, with no real file.
//! `cargo run --release --example mzml_tempi -p mzlab-core -- genera OUT.mzML [SCANS] [PEAKS_PER_SCAN]` writes a synthetic mzML (made-up reader of
//! `tests/lettore_finto`, timed); `cargo run --release --example mzml_tempi -p mzlab-core -- FILE.mzML [BUDGET_MB]` times, on any mzML, the
//! scan table (`upload::prepare`, and `upload::prepare_parallelo` with the fingerprints on a second thread), pass-by-pass `.mzidx` (`mzidx::build_mzml`) and the plain parse (`mzml::parse`).
#[path = "../tests/lettore_finto/mod.rs"]
mod finto;

use std::alloc::{GlobalAlloc, Layout, System};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use mzlab_core::{lettori_mzml, mzidx, mzml, upload};

/// Counts the live and peak heap bytes: the WASM instance has a hard 256 MB, so the peak of each phase is what matters.
struct Conta;
static VIVI: AtomicUsize = AtomicUsize::new(0);
static PICCO: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Conta {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let v = VIVI.fetch_add(l.size(), Ordering::Relaxed) + l.size();
        PICCO.fetch_max(v, Ordering::Relaxed);
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        VIVI.fetch_sub(l.size(), Ordering::Relaxed);
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let v = (VIVI.fetch_add(n, Ordering::Relaxed) + n).saturating_sub(l.size());
        VIVI.fetch_sub(l.size(), Ordering::Relaxed);
        PICCO.fetch_max(v, Ordering::Relaxed);
        System.realloc(p, l, n)
    }
}
#[global_allocator]
static A: Conta = Conta;

/// Peak heap in MB since the last call.
fn picco() -> f64 {
    let p = PICCO.swap(VIVI.load(Ordering::Relaxed), Ordering::Relaxed);
    p as f64 / 1048576.0
}

/// Discards what is written (the benchmark is not about the memory of the sink).
struct Nulla;
impl mzidx::Output for Nulla {
    fn write_at(&mut self, _o: u64, _d: &[u8]) -> mzlab_core::Result<()> {
        Ok(())
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.first().map(String::as_str) == Some("genera") {
        return genera(&a);
    }
    let path = a.first().expect("usage: mzml_tempi FILE.mzML [BUDGET_MB]");
    let budget: usize = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(192);
    let size = std::fs::metadata(path).expect("file").len();
    println!("{path}: {:.1} MB, budget {budget} MB", size as f64 / 1e6);
    let t = Instant::now();
    let mut n = 0usize;
    mzml::parse(BufReader::new(File::open(path).expect("open")), |it| {
        if let mzml::Item::Spectrum(s) = it {
            n += s.mz.len();
        }
    })
    .expect("parse");
    println!(
        "parse (decode all)      {:>8.3} s ({n} peaks)",
        t.elapsed().as_secs_f64()
    );
    let _ = picco();
    let t = Instant::now();
    let mut f = File::open(path).expect("open");
    let p = upload::prepare(&mut f, size, "x.mzML").expect("prepare");
    println!(
        "prepare (table+prints)  {:>8.3} s ({} scans, rows JSON {:.1} MB, heap peak {:.0} MB)",
        t.elapsed().as_secs_f64(),
        p.table.level.len(),
        p.table.scans_json.len() as f64 / 1048576.0,
        picco()
    );
    let t = Instant::now();
    let p2 = upload::prepare_parallelo(
        &mut File::open(path).expect("open"),
        File::open(path).expect("open"),
        size,
        "x.mzML",
    )
    .expect("prepare");
    assert_eq!(p2.sha256, p.sha256);
    drop(p2);
    println!(
        "prepare (prints apart)  {:>8.3} s",
        t.elapsed().as_secs_f64()
    );
    // without a spool every round decodes the file again; with a spool (a scratch file next to the mzML) it is decoded once
    for con in [false, true] {
        let spool_path = format!("{path}.spool");
        let mut spool = con.then(|| {
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&spool_path)
                .expect("spool")
        });
        let _ = picco();
        let t = Instant::now();
        let st = mzidx::build_mzml_spill(
            &p.table,
            BufReader::new(File::open(path).expect("open")),
            &mut Nulla,
            budget << 20,
            spool
                .as_mut()
                .map(|f| f as &mut dyn mzlab_core::mzidx_spill::Spill),
        )
        .expect("index");
        println!(
            ".mzidx {:<14}  {:>8.3} s ({:.1} MB, {} passes)",
            if con { "(with spool)" } else { "(no spool)" },
            t.elapsed().as_secs_f64(),
            st.bytes as f64 / 1e6,
            st.passes
        );
        println!(
            "  heap peak {:.0} MB (table {:.0} MB kept)",
            picco(),
            p.table.scans_json.len() as f64 / 1048576.0
        );
        let _ = std::fs::remove_file(&spool_path);
    }
}

/// `genera OUT.mzML [SCANS] [PEAKS]`: writes a synthetic mzML to time.
fn genera(a: &[String]) {
    let out = a.get(1).expect("usage: genera OUT.mzML [SCANS] [PEAKS]");
    let n: usize = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(20000);
    let peaks: usize = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(2000);
    let mut l = finto::Finto::vario(n, peaks, 12345);
    let t = Instant::now();
    let w = BufWriter::new(File::create(out).expect("create"));
    lettori_mzml::scrivi(&mut l, 0, w, &mut |_, _| {}).expect("write");
    let size = std::fs::metadata(out).map(|m| m.len()).unwrap_or(0);
    println!(
        "write {n} scans: {:.2} s, {:.1} MB",
        t.elapsed().as_secs_f64(),
        size as f64 / 1e6
    );
}
