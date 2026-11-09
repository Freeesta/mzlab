//! The module `aperti` reads a gzip-compressed mzML (`x.mzML.gz`): plain and numpress arrays inside, and the hostile cases (docs/agenti/lettori-aperti.md).
#![cfg(feature = "aperti")]

mod common;

use common::*;
use flate2::write::GzEncoder;
use flate2::Compression;
use mzlab_core::lettori::Limits;
use mzlab_core::numpress::Np;
use std::io::Write;

fn gz(b: &[u8]) -> Vec<u8> {
    let mut e = GzEncoder::new(Vec::new(), Compression::default());
    e.write_all(b).unwrap();
    e.finish().unwrap()
}

fn abre(
    name: &str,
    b: Vec<u8>,
    l: &Limits,
) -> Result<Box<dyn mzlab_core::lettori::Reader>, &'static str> {
    mzlab_lettori::apri(file_set(name, b), l).map_err(|e| e.key)
}

#[test]
fn an_mzml_gz_with_flat_arrays_returns_exact() {
    let scan = scans();
    let mut l = abre(
        "run.mzML.gz",
        gz(&mzml_di(&scan, Cod::Piano, None)),
        &Limits::sito(),
    )
    .unwrap();
    assert_eq!(l.samples()[0].scansioni, scan.len());
    for (k, x) in scan.iter().enumerate() {
        let s = l.scan(0, k).unwrap();
        assert_eq!(s.mz, x.mz, "scan {k}");
    }
    // the mzML written from it is a plain file the engine reads
    let out = written_mzml(&mut l);
    assert_eq!(
        mzlab_core::mzml::read(&out[..]).unwrap().spectra.len(),
        scan.len()
    );
}

#[test]
fn an_mzml_gz_with_numpress_opens() {
    let scan = scans();
    let cod = Cod::Np {
        intensities: Np::Slof,
        zlib: true,
    };
    let mut l = abre(
        "run.mzML.gz",
        gz(&mzml_di(&scan, cod, None)),
        &Limits::sito(),
    )
    .unwrap();
    assert_eq!(l.scan(0, 0).unwrap().mz.len(), scan[0].mz.len());
}

/// A download stopped midway: the spectra that inflated open, and `mancanti` counts those the run header declared and the cut took.
#[test]
fn a_truncated_gz_opens_with_the_spectra_it_has() {
    let scan = scans();
    let good = gz(&mzml_di(&scan, Cod::Piano, None));
    let mut whole = abre("a.mzML.gz", good.clone(), &Limits::sito()).unwrap();
    assert_eq!(whole.missing(0), 0);
    let mut l = abre(
        "a.mzML.gz",
        good[..good.len() * 3 / 4].to_vec(),
        &Limits::sito(),
    )
    .unwrap();
    let ha = l.samples()[0].scansioni;
    assert!(ha > 0 && ha < scan.len(), "{ha} of {}", scan.len());
    assert_eq!(ha + l.missing(0), scan.len());
    assert_eq!(l.scan(0, 0).unwrap().mz, scan[0].mz);
}

#[test]
fn hostile_cases_give_a_key() {
    let scan = scans();
    let good = gz(&mzml_di(&scan, Cod::Piano, None));
    let key = |name: &str, b: Vec<u8>, l: &Limits| abre(name, b, l).err();
    // cut before anything inflated: a damaged gzip, the key of the page's inflate (tests/gz_bordi.rs)
    assert_eq!(
        key("a.mzML.gz", good[..12].to_vec(), &Limits::sito()),
        Some("err.file.zlib")
    );
    // not gzip at all, and gzip of something that is not an mzML
    assert_eq!(
        key("a.mzML.gz", b"<mzML></mzML>".to_vec(), &Limits::sito()),
        Some("err.file.notAccepted")
    );
    assert_eq!(
        key("a.mzML.gz", gz(b"hello world"), &Limits::sito()),
        Some("err.file.notAccepted")
    );
    // inflated size over the limit: a memory-budget error, not an allocation
    let mut little = Limits::sito();
    little.byte = 200;
    assert_eq!(key("a.mzML.gz", good, &little), Some("error.memoryBudget"));
}

/// `x.mzXML.gz` in the app and the CLI (registry, `tutte`): the scans of the inflated mzXML, exactly. The site gunzips it into OPFS with `mzlab_core::gunzip` and gives the
/// `.mzXML` to the module, which is the same reader: the mzML both write is the same.
#[cfg(feature = "tutte")]
#[test]
fn an_mzxml_gz_opens_like_its_mzxml() {
    let flat = common::mzxml::mzxml(&common::mzxml::run(), common::mzxml::FLAT);
    let mut from_gz = abre("run.mzXML.gz", gz(&flat), &Limits::sito()).unwrap();
    let mut from_flat = abre("run.mzXML", flat.clone(), &Limits::sito()).unwrap();
    let n = from_flat.samples()[0].scansioni;
    assert!(n > 0);
    assert_eq!(from_gz.samples()[0].scansioni, n);
    assert_eq!(written_mzml(&mut from_gz), written_mzml(&mut from_flat));
    // the bytes the site writes into OPFS before the module reads them
    let mut g = mzlab_core::gunzip::Gunzip::new(u64::MAX);
    let mut site = Vec::new();
    for p in gz(&flat).chunks(333) {
        site.extend(g.push(p).unwrap());
    }
    site.extend(g.finish().unwrap().0);
    assert_eq!(site, flat);
}
