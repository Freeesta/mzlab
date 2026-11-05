//! The findings of `mutazioni_lettori.rs`, one case each: a NaN or infinite number of a file is `err.file.nonFinite` (scan) or `err.file.nonFiniteTrace`, a Waters scan of
//! zero records does not divide by zero.
#![cfg(feature = "tutte")]

mod comune;
mod comune_waters;

use comune::andi::{andi_ms, punti, PIATTA};
use comune::mzxml::{corsa, mzxml, PIATTO};
use comune::*;
use mzlab_core::lettori::Limiti;

/// The key of the first error met reading index, every scan and every trace; `"ok"` when none.
fn chiave(fs: mzlab_core::lettori::FileSet) -> String {
    let mut l = match mzlab_lettori::apri(fs, &Limiti::sito()) {
        Ok(l) => l,
        Err(e) => return e.key.to_string(),
    };
    let n = match l.indice(0) {
        Ok(ix) => ix.len(),
        Err(e) => return e.key.to_string(),
    };
    for k in 0..n {
        if let Err(e) = l.scansione(0, k) {
            return e.key.to_string();
        }
    }
    for v in l.cromatogrammi(0).unwrap_or_default() {
        if let Err(e) = l.cromatogramma(0, &v.id) {
            return e.key.to_string();
        }
    }
    "ok".into()
}

fn sostituisci(b: &[u8], da: &[u8], a: &[u8]) -> Vec<u8> {
    let i = b
        .windows(da.len())
        .position(|w| w == da)
        .expect("the bytes are in the file");
    [&b[..i], a, &b[i + da.len()..]].concat()
}

#[test]
fn un_mzxml_con_tempo_infinito_o_nan_e_un_errore() {
    let b = mzxml(&corsa(), PIATTO);
    assert_eq!(chiave(insieme("x.mzXML", b.clone())), "ok");
    for v in ["PT-infS", "PTNaNS", "PTinfS"] {
        let m = sostituisci(
            &b,
            b"retentionTime=\"PT6S",
            format!("retentionTime=\"{v}").as_bytes(),
        );
        assert_eq!(chiave(insieme("x.mzXML", m)), "err.file.nonFinite", "{v}");
    }
}

#[test]
fn un_mzxml_con_un_picco_infinito_e_un_errore() {
    // 32-bit, no zlib: the first m/z of the first scan becomes +inf (0x7f800000, big-endian)
    let o = comune::mzxml::Opz {
        a64: false,
        zlib: false,
        ..PIATTO
    };
    let mut s = corsa();
    s[0].mz[0] = f64::INFINITY;
    assert_eq!(
        chiave(insieme("x.mzXML", mzxml(&s, o))),
        "err.file.nonFinite"
    );
}

#[test]
fn un_andi_con_intensita_nan_e_un_errore() {
    let b = andi_ms(&PIATTA);
    assert_eq!(chiave(insieme("x.cdf", b.clone())), "ok");
    // the intensities are f32 big-endian (nc_type 5): the first one of the first scan
    let primo = (punti(0)[0].1 as f32).to_be_bytes();
    let m = sostituisci(&b, &primo, &f32::NAN.to_be_bytes());
    assert_eq!(chiave(insieme("x.cdf", m)), "err.file.nonFinite");
}

#[test]
fn un_mzml_numpress_con_tempo_infinito_e_un_errore() {
    let b = mzml_di(
        &scansioni(),
        Cod::Np {
            intensita: mzlab_core::numpress::Np::Slof,
            zlib: false,
        },
        None,
    );
    assert_eq!(chiave(insieme("x.mzML", b.clone())), "ok");
    let m = sostituisci(
        &b,
        b"name=\"scan start time\" value=\"0.5\"",
        b"name=\"scan start time\" value=\"-inf\"",
    );
    assert_eq!(chiave(insieme("x.mzML", m)), "err.file.nonFinite");
}

#[test]
fn waters_con_tempo_nan_nell_idx_e_un_errore() {
    let mut f = comune_waters::file_cartella();
    let idx = f.iter_mut().find(|(n, _)| n == "_FUNC001.IDX").unwrap();
    idx.1[12..16].copy_from_slice(&f32::NAN.to_le_bytes());
    assert_eq!(chiave(comune_waters::insieme(f)), "err.file.nonFinite");
}

#[test]
fn waters_con_una_scansione_di_zero_record_non_divide_per_zero() {
    // the first row of _FUNC001.IDX says 0 records: `larghezza` looks for the record width in the next scans
    let mut f = comune_waters::file_cartella();
    let idx = f.iter_mut().find(|(n, _)| n == "_FUNC001.IDX").unwrap();
    idx.1[4..8].copy_from_slice(&0x1800_0000u32.to_le_bytes());
    let r = std::panic::catch_unwind(|| chiave(comune_waters::insieme(f)));
    assert!(r.is_ok(), "a scan of 0 records panics");
}
