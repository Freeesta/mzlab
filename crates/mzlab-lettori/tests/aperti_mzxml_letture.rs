//! The mzXML index is built from a few large reads of the file, not one per scan (a read is a call into JS in the browser), and gives the same rows as before.
#![cfg(feature = "aperti")]

mod comune;

use comune::mzxml::*;
use mzlab_core::error::Result;
use mzlab_core::idxread::ReadAt;
use std::cell::Cell;
use std::rc::Rc;

struct Contato(Vec<u8>, Rc<Cell<usize>>);

impl ReadAt for Contato {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.1.set(self.1.get() + 1);
        self.0.read_at(off, buf)
    }
}

fn corsa_lunga(n: u32) -> Vec<Sc> {
    (1..=n)
        .map(|i| {
            sc(
                i,
                if i % 3 == 1 { 1 } else { 2 },
                f64::from(i),
                20,
                (i % 3 != 1).then_some(400.5),
            )
        })
        .collect()
}

#[test]
fn the_index_of_many_scans_takes_few_reads_and_the_same_rows() {
    let scan = corsa_lunga(900);
    for attributi in [true, false] {
        let byte = mzxml(
            &scan,
            Opz {
                attributi,
                ..PIATTO
            },
        );
        let chiamate = Rc::new(Cell::new(0));
        let mut fs = comune::insieme("campione.mzXML", Vec::new());
        fs.membri[0].dim = byte.len() as u64;
        fs.membri[0].dati = Box::new(Contato(byte.clone(), chiamate.clone()));
        let mut l = mzlab_lettori::apri(fs, &mzlab_core::lettori::Limiti::sito()).unwrap();
        // opening: the header, the pass over the file for the starts of the scans, the windows of the index
        assert!(chiamate.get() < scan.len() / 10, "{} reads", chiamate.get());
        let mut r = apri_mzxml(byte);
        let (a, b) = (l.indice(0).unwrap().to_vec(), r.indice(0).unwrap().to_vec());
        assert_eq!(a, b);
        assert_eq!(a.len(), scan.len());
        for (v, x) in a.iter().zip(&scan) {
            assert!((v.tic - x.it.iter().sum::<f64>()).abs() < 1e-6);
            assert_eq!(v.precursore, x.prec);
        }
    }
}
