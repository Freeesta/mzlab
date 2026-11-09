#![cfg(feature = "thermo")]
//! The Thermo reader asks its source for far fewer reads than `opentfraw` makes of it (a read is a call into JS in the browser): counted on a built run (`comune/thermo.rs`).
mod common;

use common::thermo::Raw;
use mzlab_core::error::Result;
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{FileSet, Limits, Member};
use std::cell::Cell;
use std::rc::Rc;

struct Counted(Vec<u8>, Rc<Cell<usize>>);

impl ReadAt for Counted {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.1.set(self.1.get() + 1);
        self.0.read_at(off, buf)
    }
}

#[test]
fn opening_and_indexing_a_run_takes_few_reads() {
    let scans = 2000u32;
    let bytes = Raw::tsq(63, scans).bytes();
    let calls = Rc::new(Cell::new(0));
    let set = FileSet {
        membri: vec![Member {
            nome: "x.raw".into(),
            dim: bytes.len() as u64,
            dati: Box::new(Counted(bytes.clone(), calls.clone())),
            sha1: None,
        }],
    };
    let mut l = mzlab_lettori::apri(set, &Limits::sito()).unwrap();
    let n = l.index(0).unwrap().len();
    assert_eq!(n, scans as usize);
    eprintln!(
        "read_at calls: {} for {} scans, {} bytes",
        calls.get(),
        n,
        bytes.len()
    );
    assert!(calls.get() < n / 4, "{} reads", calls.get());
    // the rows are the same as from a plain slice
    let mut r = mzlab_lettori::apri(
        FileSet {
            membri: vec![Member {
                nome: "x.raw".into(),
                dim: bytes.len() as u64,
                dati: Box::new(bytes),
                sha1: None,
            }],
        },
        &Limits::sito(),
    )
    .unwrap();
    assert_eq!(l.index(0).unwrap(), r.index(0).unwrap());
}
