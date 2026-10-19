//! Memory of the load of a (huge) mzML, native: heap peak of each phase (counting allocator), process peak RSS, and the size of what grows with the file
//! (scan table JSON, rows, scratch file, `.mzidx`). `cargo run --release --example mzml_memoria -p mzlab-core -- FILE.mzML [BUDGET_MB]`; no times are printed.
use std::alloc::{GlobalAlloc, Layout, System};
use std::fs::{File, OpenOptions};
use std::io::BufReader;
use std::os::unix::fs::FileExt;
use std::sync::atomic::{AtomicUsize, Ordering};

use mzlab_core::{mzidx, upload};

struct Conta;
static VIVI: AtomicUsize = AtomicUsize::new(0);
static PICCO: AtomicUsize = AtomicUsize::new(0);
// SAFETY: forwards every call to the system allocator and only counts the bytes.
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

/// Heap peak in MB since the last call.
fn picco() -> f64 {
    PICCO.swap(VIVI.load(Ordering::Relaxed), Ordering::Relaxed) as f64 / 1048576.0
}

/// Peak resident set of the process so far, in MB (`VmHWM`).
fn rss_max() -> f64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let kb = s
        .lines()
        .find(|l| l.starts_with("VmHWM:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse::<f64>().ok());
    kb.unwrap_or(0.0) / 1024.0
}

/// The index goes to a file at its offsets (as the page writes it to OPFS), so that it does not count as heap.
struct ASuFile(File);
impl mzidx::Output for ASuFile {
    fn write_at(&mut self, o: u64, d: &[u8]) -> mzlab_core::Result<()> {
        self.0
            .write_all_at(d, o)
            .map_err(|_| mzlab_core::Error::new("err.file.index"))
    }
}

/// Discards the index (`MZ_NULL=1`): for files so big that the index would not fit next to them on the disk; the heap does not depend on the sink.
struct Nulla;
impl mzidx::Output for Nulla {
    fn write_at(&mut self, _o: u64, _d: &[u8]) -> mzlab_core::Result<()> {
        Ok(())
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let path = a
        .first()
        .expect("usage: mzml_memoria FILE.mzML [BUDGET_MB]");
    let budget: usize = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(192);
    let size = std::fs::metadata(path).expect("file").len();
    println!("file {:.2} GB, budget {budget} MB", size as f64 / 1e9);
    let _ = picco();
    let p = upload::prepare(&mut File::open(path).expect("open"), size, "x.mzML").expect("prepare");
    let rows = p.table.level.len();
    println!("prepare   heap peak {:>7.1} MB, rss peak {:>7.1} MB | {rows} scans, rows JSON {:.1} MB ({:.0} B per scan)", picco(), rss_max(), p.table.scans_json.len() as f64 / 1048576.0, p.table.scans_json.len() as f64 / rows.max(1) as f64);
    let (idx, spool) = (format!("{path}.mzidx"), format!("{path}.spool"));
    let mut sp = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&spool)
        .expect("spool");
    let mut out: Box<dyn mzidx::Output> = if std::env::var("MZ_NULL").is_ok() {
        Box::new(Nulla)
    } else {
        Box::new(ASuFile(File::create(&idx).expect("idx")))
    };
    let st = mzidx::build_mzml_spill(
        &p.table,
        BufReader::new(File::open(path).expect("open")),
        &mut *out,
        budget << 20,
        Some(&mut sp as &mut dyn mzlab_core::mzidx_spill::Spill),
    )
    .expect("index");
    let sp_mb = std::fs::metadata(&spool).map(|m| m.len()).unwrap_or(0) as f64 / 1048576.0;
    println!("mzidx     heap peak {:>7.1} MB, rss peak {:>7.1} MB | {} passes, mzidx {:.1} MB, scratch file {:.1} MB", picco(), rss_max(), st.passes, st.bytes as f64 / 1048576.0, sp_mb);
    let _ = (std::fs::remove_file(&idx), std::fs::remove_file(&spool));
}
