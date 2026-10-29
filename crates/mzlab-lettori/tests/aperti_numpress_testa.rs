//! `riconosci` of the numpress reader looks to the end of the first spectrum, not at the first 64 KB only (docs/agenti/lettori-aperti.md).
#![cfg(feature = "aperti")]

mod comune;

use comune::*;
use std::cell::Cell;
use std::rc::Rc;

use mzlab_core::error::Result;
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{FileSet, Formato, Limiti, Membro};
use mzlab_core::numpress::Np;

/// One long scan whose m/z array is plain (zlib, incompressible, well over 64 KB) and whose intensities are numpress Slof.
fn solo_intensita_numpress() -> Vec<u8> {
    fixture(30_000, true)
}

fn fixture(n: usize, mz_piano: bool) -> Vec<u8> {
    let mut x = 0x2545_f491_4f6c_dd1du64;
    let mut mz = Vec::new();
    let mut acc = 100.0;
    for _ in 0..n {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        acc += 0.01 + (x % 10_000) as f64 * 1e-6;
        mz.push(acc);
    }
    let it: Vec<f64> = mz.iter().map(|m| 50.0 + (m * 7.0) % 900.0).collect();
    let scan = vec![Scan {
        mz: mz.clone(),
        intensita: it,
        livello: 1,
        rt_min: 1.0,
        precursore: None,
    }];
    let np = Cod::Np {
        intensita: Np::Slof,
        zlib: false,
    };
    let completo = String::from_utf8(mzml_di(&scan, np, None)).unwrap();
    let da = array(&mz, np, Genere::Mz);
    assert!(completo.contains(&da));
    if !mz_piano {
        return completo.into_bytes();
    }
    completo
        .replacen(&da, &array(&mz, Cod::Piano, Genere::Mz), 1)
        .into_bytes()
}

#[test]
fn numpress_solo_nelle_intensita_dopo_un_mz_lungo_si_riconosce() {
    let byte = solo_intensita_numpress();
    let prima_accession = byte.windows(10).position(|w| w == b"MS:1002314").unwrap();
    assert!(
        prima_accession > 64 << 10,
        "fixture must hide it past 64 KB"
    );
    let mut l = apri(byte);
    assert_eq!(l.formato(), Formato::Altro("mzml-numpress"));
    let s = l.scansione(0, 0).unwrap();
    assert_eq!(s.mz.len(), 30_000);
}

#[test]
fn uno_spettro_senza_id_conserva_la_finestra() {
    let scan = scansioni();
    let np = Cod::Np {
        intensita: Np::Slof,
        zlib: false,
    };
    let testo = String::from_utf8(mzml_di(&scan, np, None)).unwrap();
    let senza_id = testo.replace("id=\"controllerType=0 controllerNumber=1 scan=", "x=\"");
    let mut l = apri(senza_id.into_bytes());
    for k in 0..scan.len() {
        let s = l.scansione(0, k).unwrap();
        assert_eq!(s.id, format!("index={k}"));
        assert_eq!(s.index, k);
        assert_eq!(l.extra_mzml(&s).window, Some((100.0, 1000.0)), "scan {k}");
    }
}

/// Counts the bytes asked of the file.
struct Contatore(Vec<u8>, Rc<Cell<u64>>);

impl ReadAt for Contatore {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.1.set(self.1.get() + buf.len() as u64);
        self.0.read_at(off, buf)
    }
}

fn letti_da_testata(byte: Vec<u8>) -> (u64, u64) {
    let dim = byte.len() as u64;
    let letti = Rc::new(Cell::new(0));
    let fs = FileSet {
        membri: vec![Membro {
            nome: "campione.mzML".into(),
            dim,
            dati: Box::new(Contatore(byte, letti.clone())),
            sha1: None,
        }],
    };
    let mut l = mzlab_lettori::apri(fs, &Limiti::sito()).unwrap();
    let prima = letti.get();
    l.testata_mzml(0).unwrap();
    (letti.get() - prima, dim)
}

#[test]
fn la_testata_che_nomina_la_sorgente_non_rilegge_il_file_per_lo_sha1() {
    // numpress in both arrays, so the old reader recognises it too
    let con = fixture(30_000, false);
    let testo = String::from_utf8(con.clone()).unwrap();
    let a = testo.find("<sourceFileList").unwrap();
    let b = testo.find("</sourceFileList>").unwrap() + "</sourceFileList>".len();
    let senza = format!("{}{}", &testo[..a], &testo[b..]).into_bytes();
    let (letti_con, _) = letti_da_testata(con);
    let (letti_senza, dim) = letti_da_testata(senza);
    // the file without a named source is read whole for its SHA-1; the one with a source is not
    assert!(
        letti_senza >= letti_con + dim * 9 / 10,
        "with {letti_con}, without {letti_senza}, file {dim}"
    );
}
