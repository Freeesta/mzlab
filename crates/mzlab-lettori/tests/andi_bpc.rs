//! The BPC of an ANDI-MS file comes from the base peaks noted while the scans are decoded: one decode per scan for the `.mzidx` build and the BPC
//! asked after it (`cromatogrammi_scritti` included), and the same bits as the BPC of a reader that decodes the scans only to answer it.

mod comune;

use comune::andi::*;
use mzlab_core::error::Result;
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{FileSet, Limits, Member, Reader};
use mzlab_core::upload::indicizza_lettore;
use std::cell::Cell;
use std::io::Cursor;
use std::rc::Rc;

/// The bytes of the file that counts what is read at or after `da` (the points: the record variables are at the end of a netCDF-3).
struct Counter {
    b: Vec<u8>,
    da: u64,
    read_items: Rc<Cell<u64>>,
}

impl ReadAt for Counter {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> Result<()> {
        if off >= self.da {
            self.read_items
                .set(self.read_items.get() + buf.len() as u64);
        }
        self.b.read_at(off, buf)
    }
}

/// An ANDI-MS file of float points (8 bytes a point in the records) and a reader of it that counts the bytes of points it reads.
fn contato(
    points: fn(usize) -> Vec<(f64, f64)>,
    n: usize,
    tic: bool,
) -> (Box<dyn Reader>, Rc<Cell<u64>>) {
    let b = andi_ms_with(&FLAT_RUN, n, points, tic);
    let total: usize = (0..n).map(|k| points(k).len()).sum();
    let read_items = Rc::new(Cell::new(0));
    let da = (b.len() - 8 * total) as u64;
    let fs = FileSet {
        membri: vec![Member {
            nome: "campione.cdf".into(),
            dim: b.len() as u64,
            dati: Box::new(Counter {
                b,
                da,
                read_items: read_items.clone(),
            }),
            sha1: None,
        }],
    };
    (
        mzlab_lettori::apri(fs, &Limits::sito()).unwrap(),
        read_items,
    )
}

/// Builds the `.mzidx` of sample 0, then asks the BPC; returns the bytes of points read by both.
fn index_and_bpc(l: &mut dyn Reader, read_items: &Cell<u64>) -> (u64, Vec<f64>) {
    // the SHA-1 of the header reads the whole file once: taken before counting
    l.method(0).unwrap();
    read_items.set(0);
    let mut out = Cursor::new(Vec::new());
    indicizza_lettore(l, 0, &[], "campione.cdf", &mut out, 64 << 20, &mut |_| {}).unwrap();
    let bpc = l.chromatogram(0, "BPC").unwrap().intensity;
    (read_items.get(), bpc)
}

/// The bytes of points of one decode of every scan.
fn one_pass(points: fn(usize) -> Vec<(f64, f64)>, n: usize) -> u64 {
    let (mut l, read_items) = contato(points, n, true);
    l.index(0).unwrap();
    read_items.set(0);
    for k in 0..n {
        l.scan(0, k).unwrap();
    }
    read_items.get()
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

#[test]
fn index_and_bpc_decode_each_scan_once() {
    let n = 300;
    let passata = one_pass(points, n);
    assert!(passata > 0);
    let (mut l, read_items) = contato(points, n, true);
    let (read_one, _) = index_and_bpc(&mut *l, &read_items);
    assert_eq!(
        read_one, passata,
        "bytes of points read: {read_one}, one pass: {passata}"
    );
    // asked again: still nothing read
    read_items.set(0);
    l.chromatogram(0, "BPC").unwrap();
    assert_eq!(read_items.get(), 0);
}

#[test]
fn without_total_intensity_the_index_tic_also_leaves_the_bpc() {
    let n = 300;
    let passata = one_pass(points, n);
    let (mut l, read_items) = contato(points, n, false);
    let (read_one, _) = index_and_bpc(&mut *l, &read_items);
    // one pass sums the TIC of the index, one builds the rows; the BPC reads nothing more
    assert_eq!(read_one, 2 * passata);
}

/// Ties, all-zero, empty, negative and NaN points; m/z out of order every fifth scan.
fn edge_points(k: usize) -> Vec<(f64, f64)> {
    match k % 7 {
        0 => vec![(100.0, 50.0), (101.0, 50.0), (102.0, 10.0)],
        1 => vec![(100.0, 0.0), (120.0, 0.0)],
        2 => Vec::new(),
        3 => vec![(100.0, -5.0), (130.0, -1.0)],
        4 => vec![(110.0, 7.25), (90.0, 3.0e9), (95.0, 3.0e9)],
        5 => vec![(100.0, f64::NAN), (101.0, 2.0)],
        _ => (0..5)
            .map(|i| (60.0 + i as f64, 1.0 + i as f64 * 0.1))
            .collect(),
    }
}

fn without_nan(k: usize) -> Vec<(f64, f64)> {
    if k % 7 == 5 {
        vec![(100.0, 4.0), (101.0, 2.0)]
    } else {
        edge_points(k)
    }
}

/// The BPC and TIC of a fresh reader that answers before any scan is decoded.
fn freddo(b: &[u8]) -> (Vec<u64>, Vec<u64>) {
    let mut l = apri_andi(b.to_vec());
    let bpc = l.chromatogram(0, "BPC").unwrap().intensity;
    let tic = l.chromatogram(0, "TIC").unwrap().intensity;
    (bits(&bpc), bits(&tic))
}

#[test]
fn the_annotated_bpc_has_the_bits_of_the_decoded_one() {
    for tipo_it in [3, 4, 5, 6] {
        for tic in [true, false] {
            let c = Run {
                tipo_it,
                ..FLAT_RUN
            };
            let b = andi_ms_with(&c, 40, without_nan, tic);
            let (bpc0, tic0) = freddo(&b);
            assert_eq!(bpc0.len(), 40);
            let mut l = apri_andi(b);
            let mut out = Cursor::new(Vec::new());
            indicizza_lettore(
                &mut *l,
                0,
                &[],
                "campione.cdf",
                &mut out,
                64 << 20,
                &mut |_| {},
            )
            .unwrap();
            for _ in 0..2 {
                let bpc = l.chromatogram(0, "BPC").unwrap().intensity;
                assert_eq!(bits(&bpc), bpc0, "type {tipo_it}, tic {tic}");
            }
            let t = l.chromatogram(0, "TIC").unwrap().intensity;
            assert_eq!(bits(&t), tic0, "type {tipo_it}, tic {tic}");
        }
    }
}

#[test]
fn a_nan_intensity_gives_in_the_bpc_what_it_gave_before() {
    for tipo_it in [5, 6] {
        let c = Run {
            tipo_it,
            ..FLAT_RUN
        };
        let b = andi_ms_with(&c, 40, edge_points, true);
        let bpc0 = apri_andi(b.clone())
            .chromatogram(0, "BPC")
            .unwrap()
            .intensity;
        let mut l = apri_andi(b);
        // the guard refuses the scans with a NaN; the base peaks are noted all the same
        for k in 0..40 {
            let _ = l.scan(0, k);
        }
        let bpc = l.chromatogram(0, "BPC").unwrap().intensity;
        assert_eq!(bits(&bpc), bits(&bpc0));
        assert_eq!(bpc[5], 2.0);
    }
}
