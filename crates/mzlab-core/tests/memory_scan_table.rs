//! Memory held by the scan table of an open file (`Index::open`), measured with a counting allocator on a synthetic DDA-like mzML (no real data).
//! Pins the numbers the memory decisions rest on (`docs/agenti/memory-limits.md`): bytes per scan, and that a second slot of the same file
//! (`Index::over`) adds none. The table lives outside the engine's cache budget, so these constants are what an estimate must use.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering::Relaxed};

use mzlab_core::idxread::Index;
mod mem_gen;
use mem_gen::mzidx;

struct Counter;
static ALIVE: AtomicIsize = AtomicIsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

fn add(d: isize) {
    let v = ALIVE.fetch_add(d, Relaxed) + d;
    PEAK.fetch_max(v.max(0) as usize, Relaxed);
}

unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            add(l.size() as isize);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) };
        add(-(l.size() as isize));
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let q = unsafe { System.realloc(p, l, n) };
        if !q.is_null() {
            add(n as isize - l.size() as isize);
        }
        q
    }
}
#[global_allocator]
static A: Counter = Counter;

#[test]
fn a_scan_table_holds_a_fixed_number_of_bytes_per_scan_and_a_second_slot_adds_none() {
    let n = 20_000;
    let bytes = mzidx(n, 9);
    let size = bytes.len() as u64;
    let before = ALIVE.load(Relaxed);
    PEAK.store(before.max(0) as usize, Relaxed);
    let idx = Index::open(bytes, size).unwrap();
    // the index bytes were moved in: what is alive beyond `before` is the table plus the index bytes held by the reader
    let held = (ALIVE.load(Relaxed) - before) as f64; // the index bytes were allocated before `before`
    let per_scan = held / n as f64;
    let peak_per_scan = (PEAK.load(Relaxed) as isize - before) as f64 / n as f64;
    eprintln!(
        "index {:.0} B per scan; scan table: {per_scan:.0}",
        size as f64 / n as f64
    );
    let own = std::mem::size_of::<mzlab_core::idxread::Scan>();
    eprintln!(
        "size_of Scan {own}, heap per scan {:.0}",
        per_scan - own as f64
    );
    eprintln!("scan table: {per_scan:.0} B per scan resident, {peak_per_scan:.0} B per scan at the peak of open");
    assert_eq!(idx.scans.len(), n);
    // resident bytes per scan: between the size of the row and a bounded string cost; a growth of the row shows here
    assert!(
        (200.0..=700.0).contains(&per_scan),
        "resident {per_scan:.0} B per scan"
    );
    assert!(
        peak_per_scan < 2_500.0,
        "peak {peak_per_scan:.0} B per scan"
    );
    let mark = ALIVE.load(Relaxed);
    let second = idx.over(Vec::<u8>::new());
    let extra = ALIVE.load(Relaxed) - mark;
    assert!(std::sync::Arc::ptr_eq(&idx.scans, &second.scans));
    assert!(
        extra < 64 * 1024,
        "a second slot of the same file added {extra} B"
    );
}
