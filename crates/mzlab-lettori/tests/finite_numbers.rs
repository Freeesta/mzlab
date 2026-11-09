//! The findings of `reader_mutations.rs`, one case each: a NaN or infinite number of a file is `err.file.nonFinite` (scan) or `err.file.nonFiniteTrace`, a Waters scan of
//! zero records does not divide by zero.
#![cfg(feature = "tutte")]

mod common;
mod common_waters;

use common::andi::{andi_ms, points, FLAT_RUN};
use common::mzxml::{mzxml, run, FLAT};
use common::*;
use mzlab_core::readers::Limits;

/// The key of the first error met reading index, every scan and every trace; `"ok"` when none.
fn key(fs: mzlab_core::readers::FileSet) -> String {
    let mut l = match mzlab_lettori::apri(fs, &Limits::sito()) {
        Ok(l) => l,
        Err(e) => return e.key.to_string(),
    };
    let n = match l.index(0) {
        Ok(ix) => ix.len(),
        Err(e) => return e.key.to_string(),
    };
    for k in 0..n {
        if let Err(e) = l.scan(0, k) {
            return e.key.to_string();
        }
    }
    for v in l.chromatograms(0).unwrap_or_default() {
        if let Err(e) = l.chromatogram(0, &v.id) {
            return e.key.to_string();
        }
    }
    "ok".into()
}

fn replace(b: &[u8], da: &[u8], a: &[u8]) -> Vec<u8> {
    let i = b
        .windows(da.len())
        .position(|w| w == da)
        .expect("the bytes are in the file");
    [&b[..i], a, &b[i + da.len()..]].concat()
}

#[test]
fn an_mzxml_with_infinite_or_nan_time_is_an_error() {
    let b = mzxml(&run(), FLAT);
    assert_eq!(key(file_set("x.mzXML", b.clone())), "ok");
    for v in ["PT-infS", "PTNaNS", "PTinfS"] {
        let m = replace(
            &b,
            b"retentionTime=\"PT6S",
            format!("retentionTime=\"{v}").as_bytes(),
        );
        assert_eq!(key(file_set("x.mzXML", m)), "err.file.nonFinite", "{v}");
    }
}

#[test]
fn an_mzxml_with_an_infinite_peak_is_an_error() {
    // 32-bit, no zlib: the first m/z of the first scan becomes +inf (0x7f800000, big-endian)
    let o = common::mzxml::Opz {
        a64: false,
        zlib: false,
        ..FLAT
    };
    let mut s = run();
    s[0].mz[0] = f64::INFINITY;
    assert_eq!(key(file_set("x.mzXML", mzxml(&s, o))), "err.file.nonFinite");
}

#[test]
fn an_andi_with_nan_intensity_is_an_error() {
    let b = andi_ms(&FLAT_RUN);
    assert_eq!(key(file_set("x.cdf", b.clone())), "ok");
    // the intensities are f32 big-endian (nc_type 5): the first one of the first scan
    let first_one = (points(0)[0].1 as f32).to_be_bytes();
    let m = replace(&b, &first_one, &f32::NAN.to_be_bytes());
    assert_eq!(key(file_set("x.cdf", m)), "err.file.nonFinite");
}

#[test]
fn a_numpress_mzml_with_infinite_time_is_an_error() {
    let b = mzml_di(
        &scans(),
        Cod::Np {
            intensities: mzlab_core::numpress::Np::Slof,
            zlib: false,
        },
        None,
    );
    assert_eq!(key(file_set("x.mzML", b.clone())), "ok");
    let m = replace(
        &b,
        b"name=\"scan start time\" value=\"0.5\"",
        b"name=\"scan start time\" value=\"-inf\"",
    );
    assert_eq!(key(file_set("x.mzML", m)), "err.file.nonFinite");
}

#[test]
fn waters_with_nan_time_in_the_idx_is_an_error() {
    let mut f = common_waters::folder_file();
    let idx = f.iter_mut().find(|(n, _)| n == "_FUNC001.IDX").unwrap();
    idx.1[12..16].copy_from_slice(&f32::NAN.to_le_bytes());
    assert_eq!(key(common_waters::file_set(f)), "err.file.nonFinite");
}

#[test]
fn waters_with_a_zero_record_scan_does_not_divide_by_zero() {
    // the first row of _FUNC001.IDX says 0 records: `larghezza` looks for the record width in the next scans
    let mut f = common_waters::folder_file();
    let idx = f.iter_mut().find(|(n, _)| n == "_FUNC001.IDX").unwrap();
    idx.1[4..8].copy_from_slice(&0x1800_0000u32.to_le_bytes());
    let r = std::panic::catch_unwind(|| key(common_waters::file_set(f)));
    assert!(r.is_ok(), "a scan of 0 records panics");
}
