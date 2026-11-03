//! The module `aperti` reads a gzip-compressed mzML (`x.mzML.gz`): plain and numpress arrays inside, and the hostile cases (docs/agenti/lettori-aperti.md).
#![cfg(feature = "aperti")]

mod comune;

use comune::*;
use flate2::write::GzEncoder;
use flate2::Compression;
use mzlab_core::lettori::Limiti;
use mzlab_core::numpress::Np;
use std::io::Write;

fn gz(b: &[u8]) -> Vec<u8> {
    let mut e = GzEncoder::new(Vec::new(), Compression::default());
    e.write_all(b).unwrap();
    e.finish().unwrap()
}

fn abre(
    nome: &str,
    b: Vec<u8>,
    l: &Limiti,
) -> Result<Box<dyn mzlab_core::lettori::Lettore>, &'static str> {
    mzlab_lettori::apri(insieme(nome, b), l).map_err(|e| e.key)
}

#[test]
fn un_mzml_gz_con_array_piani_torna_esatto() {
    let scan = scansioni();
    let mut l = abre(
        "run.mzML.gz",
        gz(&mzml_di(&scan, Cod::Piano, None)),
        &Limiti::sito(),
    )
    .unwrap();
    assert_eq!(l.campioni()[0].scansioni, scan.len());
    for (k, x) in scan.iter().enumerate() {
        let s = l.scansione(0, k).unwrap();
        assert_eq!(s.mz, x.mz, "scan {k}");
    }
    // the mzML written from it is a plain file the engine reads
    let out = mzml_scritto(&mut l);
    assert_eq!(
        mzlab_core::mzml::read(&out[..]).unwrap().spectra.len(),
        scan.len()
    );
}

#[test]
fn un_mzml_gz_con_numpress_si_apre() {
    let scan = scansioni();
    let cod = Cod::Np {
        intensita: Np::Slof,
        zlib: true,
    };
    let mut l = abre(
        "run.mzML.gz",
        gz(&mzml_di(&scan, cod, None)),
        &Limiti::sito(),
    )
    .unwrap();
    assert_eq!(l.scansione(0, 0).unwrap().mz.len(), scan[0].mz.len());
}

/// A download stopped midway: the spectra that inflated open, and `mancanti` counts those the run header declared and the cut took.
#[test]
fn un_gz_troncato_si_apre_con_gli_spettri_che_ha() {
    let scan = scansioni();
    let buono = gz(&mzml_di(&scan, Cod::Piano, None));
    let mut intero = abre("a.mzML.gz", buono.clone(), &Limiti::sito()).unwrap();
    assert_eq!(intero.mancanti(0), 0);
    let mut l = abre(
        "a.mzML.gz",
        buono[..buono.len() * 3 / 4].to_vec(),
        &Limiti::sito(),
    )
    .unwrap();
    let ha = l.campioni()[0].scansioni;
    assert!(ha > 0 && ha < scan.len(), "{ha} of {}", scan.len());
    assert_eq!(ha + l.mancanti(0), scan.len());
    assert_eq!(l.scansione(0, 0).unwrap().mz, scan[0].mz);
}

#[test]
fn i_casi_ostili_danno_una_chiave() {
    let scan = scansioni();
    let buono = gz(&mzml_di(&scan, Cod::Piano, None));
    let chiave = |nome: &str, b: Vec<u8>, l: &Limiti| abre(nome, b, l).err();
    // cut before anything inflated: nothing to open
    assert_eq!(
        chiave("a.mzML.gz", buono[..12].to_vec(), &Limiti::sito()),
        Some("err.file.notAccepted")
    );
    // not gzip at all, and gzip of something that is not an mzML
    assert_eq!(
        chiave("a.mzML.gz", b"<mzML></mzML>".to_vec(), &Limiti::sito()),
        Some("err.file.notAccepted")
    );
    assert_eq!(
        chiave("a.mzML.gz", gz(b"hello world"), &Limiti::sito()),
        Some("err.file.notAccepted")
    );
    // inflated size over the limit: a memory-budget error, not an allocation
    let mut poco = Limiti::sito();
    poco.byte = 200;
    assert_eq!(
        chiave("a.mzML.gz", buono, &poco),
        Some("error.memoryBudget")
    );
}
