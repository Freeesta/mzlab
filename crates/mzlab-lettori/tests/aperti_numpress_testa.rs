//! `riconosci` of the numpress reader looks to the end of the first spectrum, not at the first 64 KB only (docs/agenti/lettori-aperti.md).
#![cfg(feature = "aperti")]

mod comune;

use comune::*;
use std::cell::Cell;
use std::rc::Rc;

use mzlab_core::error::Result;
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{FileSet, Format, Limits, Member};
use mzlab_core::numpress::Np;

/// One long scan whose m/z array is plain (zlib, incompressible, well over 64 KB) and whose intensities are numpress Slof.
fn numpress_intensities_only() -> Vec<u8> {
    fixture(30_000, true)
}

fn fixture(n: usize, flat_mz: bool) -> Vec<u8> {
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
        intensities: it,
        level: 1,
        rt_min: 1.0,
        precursore: None,
    }];
    let np = Cod::Np {
        intensities: Np::Slof,
        zlib: false,
    };
    let complete_one = String::from_utf8(mzml_di(&scan, np, None)).unwrap();
    let da = array(&mz, np, Genere::Mz);
    assert!(complete_one.contains(&da));
    if !flat_mz {
        return complete_one.into_bytes();
    }
    complete_one
        .replacen(&da, &array(&mz, Cod::Piano, Genere::Mz), 1)
        .into_bytes()
}

#[test]
fn numpress_only_in_intensities_after_a_long_mz_is_recognised() {
    let byte = numpress_intensities_only();
    let first_accession = byte.windows(10).position(|w| w == b"MS:1002314").unwrap();
    assert!(
        first_accession > 64 << 10,
        "fixture must hide it past 64 KB"
    );
    let mut l = apri(byte);
    assert_eq!(l.format(), Format::Altro("mzml-numpress"));
    let s = l.scan(0, 0).unwrap();
    assert_eq!(s.mz.len(), 30_000);
}

#[test]
fn a_spectrum_without_id_keeps_the_window() {
    let scan = scansioni();
    let np = Cod::Np {
        intensities: Np::Slof,
        zlib: false,
    };
    let text = String::from_utf8(mzml_di(&scan, np, None)).unwrap();
    let without_id = text.replace("id=\"controllerType=0 controllerNumber=1 scan=", "x=\"");
    let mut l = apri(without_id.into_bytes());
    for k in 0..scan.len() {
        let s = l.scan(0, k).unwrap();
        assert_eq!(s.id, format!("index={k}"));
        assert_eq!(s.index, k);
        assert_eq!(l.extra_mzml(&s).window, Some((100.0, 1000.0)), "scan {k}");
    }
}

/// Counts the bytes asked of the file.
struct Counter(Vec<u8>, Rc<Cell<u64>>);

impl ReadAt for Counter {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        self.1.set(self.1.get() + buf.len() as u64);
        self.0.read_at(off, buf)
    }
}

fn read_from_header(byte: Vec<u8>) -> (u64, u64) {
    let dim = byte.len() as u64;
    let read_items = Rc::new(Cell::new(0));
    let fs = FileSet {
        membri: vec![Member {
            nome: "campione.mzML".into(),
            dim,
            dati: Box::new(Counter(byte, read_items.clone())),
            sha1: None,
        }],
    };
    let mut l = mzlab_lettori::apri(fs, &Limits::sito()).unwrap();
    let before = read_items.get();
    l.mzml_header(0).unwrap();
    (read_items.get() - before, dim)
}

#[test]
fn a_header_naming_the_source_does_not_reread_the_file_for_sha1() {
    // numpress in both arrays, so the old reader recognises it too
    let con = fixture(30_000, false);
    let text = String::from_utf8(con.clone()).unwrap();
    let a = text.find("<sourceFileList").unwrap();
    let b = text.find("</sourceFileList>").unwrap() + "</sourceFileList>".len();
    let without = format!("{}{}", &text[..a], &text[b..]).into_bytes();
    let (read_with, _) = read_from_header(con);
    let (read_without, dim) = read_from_header(without);
    // the file without a named source is read whole for its SHA-1; the one with a source is not
    assert!(
        read_without >= read_with + dim * 9 / 10,
        "with {read_with}, without {read_without}, file {dim}"
    );
}

#[test]
fn a_plain_mzml_is_not_captured_by_the_numpress_module() {
    // a long first spectrum, no numpress anywhere: the scan to the end of it must still say No
    let scan = scansioni();
    let mut x = scan;
    x[0].mz = (0..30_000)
        .map(|i| 100.0 + f64::from(i as u32) * 0.011)
        .collect();
    x[0].intensities = x[0].mz.iter().map(|m| 50.0 + (m * 7.0) % 900.0).collect();
    let r = mzlab_lettori::apri(
        file_set("campione.mzML", mzml_di(&x, Cod::Piano, None)),
        &Limits::sito(),
    );
    // the family `aperti` has no reader of plain mzML (the engine of the page reads it): not this one either
    assert!(r.is_err_and(|e| e.key == "err.file.notAccepted"));
}
