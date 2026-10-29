//! Times XIC extraction on a made-up `.mzidx` and counts the reads (calls and bytes) behind it, with a `ReadAt` that counts.
//! `cargo run --release --example idx_xic_tempi -p mzlab-core -- [SCANS] [PEAKS_PER_SCAN] [XICS]`
#[path = "../tests/lettore_finto/mod.rs"]
mod finto;

use std::cell::Cell;
use std::io::Cursor;
use std::rc::Rc;
use std::time::Instant;

use mzlab_core::idxread::{Index, ReadAt};
use mzlab_core::{mzidx, upload};

/// An in-memory index that counts the reads the reader asks for (the site pays one JS/OPFS call per read).
struct Counting {
    bytes: Vec<u8>,
    reads: Rc<Cell<u64>>,
    volume: Rc<Cell<u64>>,
}
impl ReadAt for Counting {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        self.reads.set(self.reads.get() + 1);
        self.volume.set(self.volume.get() + buf.len() as u64);
        self.bytes.read_at(off, buf)
    }
}

fn main() {
    let a: Vec<usize> = std::env::args()
        .skip(1)
        .filter_map(|s| s.parse().ok())
        .collect();
    let (n, peaks, xics) = (
        *a.first().unwrap_or(&20000),
        *a.get(1).unwrap_or(&2000),
        *a.get(2).unwrap_or(&200),
    );
    let mut l = finto::Finto::vario(n, peaks, 12345);
    let imp = vec![];
    let p = upload::prepare_lettore(&mut l, 0, &imp, "x.mzML").expect("table");
    let mut out = Cursor::new(Vec::new());
    mzidx::build_lettore(&p.table, &mut l, 0, &mut out, 192 << 20).expect("index");
    let bytes = out.into_inner();
    let size = bytes.len() as u64;
    let (reads, volume) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
    let r = Counting {
        bytes,
        reads: reads.clone(),
        volume: volume.clone(),
    };
    let mut ix = Index::open(r, size).expect("open");
    let si = ix
        .section_of(1, None)
        .or_else(|| ix.section_of(1, Some(1)))
        .expect("ms1");
    // the made-up survey holds a few stray values (NaN, negative) at the ends: query the body of the range
    let (lo, hi) = (100.0, 1500.0);
    println!(
        "{n} scans, {peaks} peaks per scan, index {:.1} MB, m/z {lo:.2}..{hi:.2}",
        size as f64 / 1e6
    );
    for half in [0.5, 20.0] {
        let (r0, v0) = (reads.get(), volume.get());
        let t = Instant::now();
        let mut check = 0f64;
        for k in 0..xics {
            let mz = lo + (hi - lo) * (k as f64 + 0.5) / xics as f64;
            let a = ix.search(si, mz - half, false).expect("search");
            let b = ix.search(si, mz + half, true).expect("search");
            let y = ix.sum_window(si, a, b).expect("sum");
            check += y.iter().sum::<f64>();
            let m = ix.max_window(si, a, b).expect("max");
            check += m.iter().sum::<f64>();
        }
        println!(
            "{xics} XIC +-{half} (search x2 + sum + max)  {:>8.3} s, {:.1} reads/XIC, {:.2} MB/XIC, check {check:.6e}",
            t.elapsed().as_secs_f64(),
            (reads.get() - r0) as f64 / xics as f64,
            (volume.get() - v0) as f64 / xics as f64 / 1e6
        );
    }
}
