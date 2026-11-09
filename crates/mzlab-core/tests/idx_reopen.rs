//! `Index::over` gives the index `open` gives (scan table, sections, peaks) without parsing again, and `Head` tells two files apart by the bytes
//! `open` parses: the same file read twice has one head, an index of other scans another.
#![allow(clippy::indexing_slicing)]
#![allow(clippy::disallowed_methods)]

mod lettore_finto;

use std::io::Cursor;

use lettore_finto::Finto;
use mzlab_core::idxread::{Head, Index};
use mzlab_core::{mzidx, upload};

fn index_bytes(n: usize, peaks: usize, seed: u64) -> Vec<u8> {
    let mut l = Finto::vario(n, peaks, seed);
    let p = upload::prepare_reader(&mut l, 0, &[], "x.mzML").unwrap();
    let mut out = Cursor::new(Vec::new());
    mzidx::build_reader(&p.table, &mut l, 0, &mut out, mzidx::DEFAULT_BUDGET).unwrap();
    out.into_inner()
}

fn peaks(ix: &mut Index<Vec<u8>>) -> Vec<(usize, usize, u64, u64)> {
    let mut v = Vec::new();
    for si in 0..ix.sections.len() {
        let n = ix.sections[si].n_peaks;
        ix.each_peak(si, 0, n, |p, x, y| {
            v.push((si, p, x.to_bits(), y.to_bits()))
        })
        .unwrap();
    }
    v
}

#[test]
fn a_reopened_index_is_the_index_opened() {
    let b = index_bytes(300, 200, 777);
    let mut opened = Index::open(b.clone(), b.len() as u64).unwrap();
    let mut first = opened.over(b.clone());
    // the caches of the first reader do not pass to the copy
    let before = peaks(&mut opened);
    let mut copy = opened.over(b.clone());
    assert_eq!(format!("{:?}", copy.scans), format!("{:?}", opened.scans));
    assert_eq!(
        format!("{:?}", copy.sections),
        format!("{:?}", opened.sections)
    );
    assert_eq!(copy.keys, opened.keys);
    assert_eq!((&copy.source, &copy.run), (&opened.source, &opened.run));
    assert_eq!(peaks(&mut copy), before);
    assert_eq!(peaks(&mut first), before);
}

#[test]
fn the_head_tells_the_files_apart() {
    let (a, b) = (index_bytes(300, 200, 777), index_bytes(300, 200, 778));
    let head = |x: &[u8]| Head::read(&mut x.to_vec(), x.len() as u64).unwrap();
    assert!(head(&a) == head(&a));
    assert!(head(&a) != head(&b));
    assert!(!head(&a).is_empty());
    let ix = Index::open_head(a.clone(), &head(&a)).unwrap();
    let plain = Index::open(a.clone(), a.len() as u64).unwrap();
    assert_eq!(format!("{:?}", ix.scans), format!("{:?}", plain.scans));
    // a damaged header is refused before any parse, as by `open`
    let mut bad = a.clone();
    bad[0] = b'X';
    let err = Head::read(&mut bad.clone(), bad.len() as u64)
        .err()
        .unwrap();
    assert_eq!(err.key, "error.badIndex");
}
