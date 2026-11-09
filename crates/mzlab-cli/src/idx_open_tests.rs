//! A kept index serves the next open of the same file, and a file written again under the same name is parsed again; either way the index is the
//! one `Index::open` gives.

use std::path::Path;

use super::{open, PARSED};
use crate::convert_fixtures::folder;

fn parses(p: &Path) -> usize {
    PARSED.lock().unwrap().iter().filter(|q| *q == p).count()
}

fn scans(ix: &mzlab_core::idxread::Index<std::fs::File>) -> String {
    format!("{:?}", ix.scans)
}

fn plain(p: &Path) -> String {
    let f = std::fs::File::open(p).unwrap();
    let n = f.metadata().unwrap().len();
    scans(&mzlab_core::idxread::Index::open(f, n).unwrap())
}

#[test]
fn an_index_is_parsed_once_until_its_file_changes() {
    let d = folder("idx_open");
    let (src, idx) = (d.join("a.mzML"), d.join("a.mzidx"));
    let budget = mzlab_core::mzidx::DEFAULT_BUDGET;
    std::fs::write(&src, crate::tests::mzml(1.0)).unwrap();
    crate::build_index(&src, &idx, budget).unwrap();
    let first = scans(&open(&idx).unwrap());
    assert_eq!(first, plain(&idx));
    for _ in 0..3 {
        assert_eq!(scans(&open(&idx).unwrap()), first);
    }
    assert_eq!(parses(&idx), 1);
    std::fs::write(&src, crate::tests::mzml(2.0)).unwrap();
    crate::build_index(&src, &idx, budget).unwrap();
    let second = scans(&open(&idx).unwrap());
    assert_ne!(second, first);
    assert_eq!(second, plain(&idx));
    assert_eq!(parses(&idx), 2);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn two_passes_that_open_a_file_together_parse_it_once() {
    let d = folder("idx_open_together");
    let (src, idx) = (d.join("a.mzML"), d.join("a.mzidx"));
    std::fs::write(&src, crate::tests::mzml(3.0)).unwrap();
    crate::build_index(&src, &idx, mzlab_core::mzidx::DEFAULT_BUDGET).unwrap();
    let want = plain(&idx);
    let got: Vec<String> = std::thread::scope(|s| {
        let hs: Vec<_> = (0..4)
            .map(|_| s.spawn(|| scans(&open(&idx).unwrap())))
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert!(got.iter().all(|g| *g == want));
    assert_eq!(parses(&idx), 1);
    let _ = std::fs::remove_dir_all(&d);
}
