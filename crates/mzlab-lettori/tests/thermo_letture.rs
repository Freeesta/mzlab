#![cfg(feature = "thermo")]
//! The Thermo reader asks its source for far fewer reads than `opentfraw` makes of it (a read is a call into JS in the browser): counted on a built run (`comune/thermo.rs`).
mod comune;

use comune::thermo::Raw;
use mzlab_core::error::Result;
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{FileSet, Limiti, Membro};
use std::cell::Cell;
use std::rc::Rc;

struct Contato(Vec<u8>, Rc<Cell<usize>>);

impl ReadAt for Contato {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.1.set(self.1.get() + 1);
        self.0.read_at(off, buf)
    }
}

#[test]
fn opening_and_indexing_a_run_takes_few_reads() {
    let scans = 2000u32;
    let bytes = Raw::tsq(63, scans).bytes();
    let chiamate = Rc::new(Cell::new(0));
    let set = FileSet {
        membri: vec![Membro {
            nome: "x.raw".into(),
            dim: bytes.len() as u64,
            dati: Box::new(Contato(bytes.clone(), chiamate.clone())),
            sha1: None,
        }],
    };
    let mut l = mzlab_lettori::apri(set, &Limiti::sito()).unwrap();
    let n = l.indice(0).unwrap().len();
    assert_eq!(n, scans as usize);
    eprintln!(
        "read_at calls: {} for {} scans, {} bytes",
        chiamate.get(),
        n,
        bytes.len()
    );
    assert!(chiamate.get() < n / 4, "{} reads", chiamate.get());
    // the rows are the same as from a plain slice
    let mut r = mzlab_lettori::apri(
        FileSet {
            membri: vec![Membro {
                nome: "x.raw".into(),
                dim: bytes.len() as u64,
                dati: Box::new(bytes),
                sha1: None,
            }],
        },
        &Limiti::sito(),
    )
    .unwrap();
    assert_eq!(l.indice(0).unwrap(), r.indice(0).unwrap());
}
