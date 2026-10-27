//! `Index::search` (through the fence of the section) gives what a plain count over the whole m/z array gives, side left and right, on a made-up reader
//! with stray values (NaN, negative, out of range) and sections of several 16384-peak blocks; and a search reads one block, not a bisection of them.
#![allow(clippy::indexing_slicing)]
#![allow(clippy::disallowed_methods)]

mod lettore_finto;

use std::cell::Cell;
use std::io::Cursor;
use std::rc::Rc;

use lettore_finto::Finto;
use mzlab_core::idxread::{Index, ReadAt, BLOCK};
use mzlab_core::{mzidx, upload};

struct Counting(Vec<u8>, Rc<Cell<u64>>);
impl ReadAt for Counting {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        self.1.set(self.1.get() + 1);
        self.0.read_at(off, buf)
    }
}

fn index_bytes() -> Vec<u8> {
    let mut l = Finto::vario(900, 400, 4242);
    let p = upload::prepare_lettore(&mut l, 0, &[], "x.mzML").unwrap();
    let mut out = Cursor::new(Vec::new());
    mzidx::build_lettore(&p.table, &mut l, 0, &mut out, mzidx::DEFAULT_BUDGET).unwrap();
    out.into_inner()
}

#[test]
fn search_equals_a_count() {
    let b = index_bytes();
    let reads = Rc::new(Cell::new(0));
    let mut ix = Index::open(Counting(b.clone(), reads.clone()), b.len() as u64).unwrap();
    let mut blocks = 0;
    for si in 0..ix.sections.len() {
        let (np, f4) = (ix.sections[si].n_peaks, ix.sections[si].f4);
        let mut mz = Vec::new();
        ix.each_peak(si, 0, np, |_, m, _| mz.push(m)).unwrap();
        blocks += np.div_ceil(BLOCK);
        // the edges, the first m/z of every block, and values on both sides of them
        let mut qs = vec![-1e9, -5.0, 0.0, 99.9999999, 100.0, 100.25, 1e9, 12345.6];
        for k in (0..np).step_by(BLOCK as usize).chain([np.saturating_sub(1)]) {
            let m = mz.get(k as usize).copied().unwrap_or(0.0);
            qs.extend([m, m - 1e-3, m + 1e-3]);
        }
        qs.extend((0..40).map(|k| 50.0 + 40.0 * f64::from(k)));
        for v in qs {
            let q = if f4 { v as f32 as f64 } else { v };
            for right in [false, true] {
                let want = mz.iter().take_while(|&&x| if right { x <= q } else { x < q }).count() as u64;
                assert_eq!(ix.search(si, v, right).unwrap(), want, "section {si} v {v} right {right}");
            }
        }
    }
    assert!(blocks > 4, "the test needs sections of several blocks");
    // a search of a cold section: the fence, then one block
    let si = (0..ix.sections.len()).max_by_key(|&s| ix.sections[s].n_peaks).unwrap();
    let mut fresh = Index::open(Counting(b.clone(), reads.clone()), b.len() as u64).unwrap();
    let before = reads.get();
    fresh.search(si, 700.0, false).unwrap();
    assert!(reads.get() - before <= 2, "reads: {}", reads.get() - before);
}
