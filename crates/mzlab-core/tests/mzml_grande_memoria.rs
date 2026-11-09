//! A big mzML must be indexed inside the memory the WASM instance has (256 MB hard, 192 MB of work): the scan table keeps no header of the spectra,
//! the rows are freed while their JSON is written, and the rounds of the builder leave room for what the build holds besides them. A counting
//! allocator measures the heap peak of each phase over a file written by the made-up reader (`lettore_finto`, no real data); the `.mzidx` must be the
//! same bytes whatever the budget and with or without a scratch file. One test only: the counters are global.
#![allow(clippy::indexing_slicing, clippy::disallowed_methods)]

mod lettore_finto;

use std::alloc::{GlobalAlloc, Layout, System};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter};
use std::os::unix::fs::FileExt;
use std::sync::atomic::{AtomicUsize, Ordering};

use lettore_finto::Finto;
use mzlab_core::{lettori_mzml, mzidx, upload};

struct Tally;
static VIVI: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

// SAFETY: forwards every call to the system allocator and only counts the bytes.
unsafe impl GlobalAlloc for Tally {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        unsafe {
            let v = VIVI.fetch_add(l.size(), Ordering::Relaxed) + l.size();
            PEAK.fetch_max(v, Ordering::Relaxed);
            System.alloc(l)
        }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe {
            VIVI.fetch_sub(l.size(), Ordering::Relaxed);
            System.dealloc(p, l)
        }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        unsafe {
            let v = VIVI.fetch_add(n, Ordering::Relaxed) + n;
            VIVI.fetch_sub(l.size(), Ordering::Relaxed);
            PEAK.fetch_max(v, Ordering::Relaxed);
            System.realloc(p, l, n)
        }
    }
}
#[global_allocator]
static A: Tally = Tally;

/// Starts a measurement: the live bytes now.
fn start() -> usize {
    let v = VIVI.load(Ordering::Relaxed);
    PEAK.store(v, Ordering::Relaxed);
    v
}

/// Bytes the phase held above where it started.
fn above(base: usize) -> usize {
    PEAK.load(Ordering::Relaxed).saturating_sub(base)
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

const MB: usize = 1 << 20;
const SCANS: usize = 6000;

#[test]
fn large_mzml_fits_the_budget() {
    let path = std::env::temp_dir().join(format!("mzlab_grande_{}.mzML", std::process::id()));
    let mut l = Finto::vario(SCANS, 60, 777);
    let w = BufWriter::new(File::create(&path).unwrap());
    lettori_mzml::scrivi(&mut l, 0, w, &mut |_, _| {}).unwrap();
    let size = std::fs::metadata(&path).unwrap().len();

    // the scan table: about 1.5 KB a scan (the rows), never the headers of every spectrum on top
    let base = start();
    let mut f = File::open(&path).unwrap();
    let p = upload::prepare(&mut f, size, "grande.mzML").unwrap();
    let prep = above(base);
    assert_eq!(p.table.level.len(), SCANS);
    assert!(
        prep <= 2200 * SCANS + 4 * MB,
        "prepare held {} MB",
        prep / MB
    );

    // the index with a small budget: the peak stays inside it besides the table the caller holds
    let budget = 16 * MB;
    let build = |budget: usize, scratch: bool| {
        let out_path = path.with_extension(format!("idx{scratch}{budget}"));
        let mut out = ASuFile(File::create(&out_path).unwrap());
        let mut sp = scratch.then(|| {
            let sp_path = path.with_extension("spool");
            let f = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&sp_path)
                .unwrap();
            (f, sp_path)
        });
        let base = start();
        let src = BufReader::new(File::open(&path).unwrap());
        mzidx::build_mzml_spill(
            &p.table,
            src,
            &mut out,
            budget,
            sp.as_mut()
                .map(|(f, _)| f as &mut dyn mzlab_core::mzidx_spill::Spill),
        )
        .unwrap();
        let peak = above(base);
        if let Some((_, sp_path)) = sp {
            let _ = std::fs::remove_file(sp_path);
        }
        drop(out);
        (std::fs::read(&out_path).unwrap(), peak, out_path)
    };
    let (large, _, f0) = build(mzidx::DEFAULT_BUDGET, false);
    let _ = std::fs::remove_file(f0);
    for scratch in [false, true] {
        let (idx, picco, f1) = build(budget, scratch);
        let _ = std::fs::remove_file(f1);
        assert!(
            idx == large,
            "scratch={scratch}: the index differs from the one of a big budget"
        );
        // the part that depends on the file: the budget of the peaks, plus the lists of the build (member lists, per-scan arrays)
        assert!(
            picco <= budget + 150 * SCANS + 2 * MB,
            "scratch={scratch}: build held {} MB over a budget of {} MB",
            picco / MB,
            budget / MB
        );
    }
    let _ = std::fs::remove_file(&path);
}
