//! One route for an Agilent `.D` on the site and in the CLI: the CLI always reads with `agilent_ext` (feature `estesi`), the site asks the engine first and
//! hands to the `agilent` module what the engine refuses with `err.file.agilent`. A run of QqQ MRM visits (ScanType 256) is such a refusal on the engine, and
//! reads as SRM chromatograms in `estesi`. Synthetic folders only.

#![allow(clippy::disallowed_methods)]

mod agilent_synth;

use agilent_synth::{encode, sample};
use mzlab_core::lettori::{self, FileSet, Limiti, Membro};

fn apri(scan: Vec<u8>, peak: Vec<u8>) -> mzlab_core::error::Result<Box<dyn lettori::Lettore>> {
    let membri = [
        ("x.D/AcqData/MSScan.bin", scan),
        ("x.D/AcqData/MSPeak.bin", peak),
    ]
    .into_iter()
    .map(|(n, b)| Membro {
        nome: n.into(),
        dim: b.len() as u64,
        dati: Box::new(b),
        sha1: None,
    })
    .collect();
    lettori::apri(FileSet { membri }, &Limiti::sito())
}

/// A GC/MS run whose scans after the first are MRM visits.
fn mrm() -> (Vec<u8>, Vec<u8>) {
    let mut s = sample();
    for x in s.iter_mut().skip(1) {
        x.kind = 256;
        x.level = 2;
    }
    encode(&s)
}

#[cfg(not(feature = "estesi"))]
#[test]
fn the_engine_hands_mrm_visits_to_the_module() {
    let (scan, peak) = mrm();
    let e = apri(scan, peak).err().expect("refused");
    assert_eq!(e.key, "err.file.agilent");
    assert!(
        e.params
            .iter()
            .any(|(k, v)| *k == "detail" && v.contains("MRM")),
        "{:?}",
        e.params
    );
    // A run of plain scans still opens on the engine.
    let (scan, peak) = encode(&sample());
    assert!(apri(scan, peak).is_ok());
}

#[cfg(feature = "estesi")]
#[test]
fn the_module_reads_mrm_visits_as_srm_chromatograms() {
    let (scan, peak) = mrm();
    let mut l = apri(scan, peak).expect("opens");
    assert_eq!(l.samples()[0].scansioni, 1);
    let srm = l
        .chromatograms(0)
        .unwrap()
        .into_iter()
        .filter(|c| c.id.starts_with("SRM SIC "))
        .count();
    assert_eq!(srm, 3);
}
