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
    // cut before anything inflated: a damaged gzip, the key of the page's inflate (tests/gz_bordi.rs)
    assert_eq!(
        chiave("a.mzML.gz", buono[..12].to_vec(), &Limiti::sito()),
        Some("err.file.zlib")
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

/// `x.mzXML.gz` in the app and the CLI (registry, `tutte`): the scans of the inflated mzXML, exactly. The site gunzips it into OPFS with `mzlab_core::gunzip` and gives the
/// `.mzXML` to the module, which is the same reader: the mzML both write is the same.
#[cfg(feature = "tutte")]
#[test]
fn un_mzxml_gz_si_apre_come_il_suo_mzxml() {
    let piano = comune::mzxml::mzxml(&comune::mzxml::corsa(), comune::mzxml::PIATTO);
    let mut dal_gz = abre("run.mzXML.gz", gz(&piano), &Limiti::sito()).unwrap();
    let mut dal_piano = abre("run.mzXML", piano.clone(), &Limiti::sito()).unwrap();
    let n = dal_piano.campioni()[0].scansioni;
    assert!(n > 0);
    assert_eq!(dal_gz.campioni()[0].scansioni, n);
    assert_eq!(mzml_scritto(&mut dal_gz), mzml_scritto(&mut dal_piano));
    // the bytes the site writes into OPFS before the module reads them
    let mut g = mzlab_core::gunzip::Gunzip::new(u64::MAX);
    let mut sito = Vec::new();
    for p in gz(&piano).chunks(333) {
        sito.extend(g.push(p).unwrap());
    }
    sito.extend(g.finish().unwrap().0);
    assert_eq!(sito, piano);
}
