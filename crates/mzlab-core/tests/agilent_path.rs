//! One route for an Agilent `.D` on the site and in the CLI: the CLI always reads with `agilent_ext` (feature `estesi`), the site asks the engine first and
//! hands to the `agilent` module what the engine refuses with `err.file.agilent`. A run of QqQ MRM visits (ScanType 256) is such a refusal on the engine, and
//! reads as SRM chromatograms in `estesi`. Synthetic folders only.

#![allow(clippy::disallowed_methods)]

mod agilent_synth;

use agilent_synth::{encode, sample};
use mzlab_core::readers::{self, FileSet, Limits, Member};

fn apri(scan: Vec<u8>, peak: Vec<u8>) -> mzlab_core::error::Result<Box<dyn readers::Reader>> {
    open_with(scan, "MSPeak.bin", peak)
}

/// The run with its points in `AcqData/<punti>` (`MSPeak.bin`, or `MSProfile.bin` of a folder without peaks).
fn open_with(
    scan: Vec<u8>,
    points: &str,
    peak: Vec<u8>,
) -> mzlab_core::error::Result<Box<dyn readers::Reader>> {
    let members = [
        ("x.D/AcqData/MSScan.bin".to_string(), scan),
        (format!("x.D/AcqData/{points}"), peak),
    ]
    .into_iter()
    .map(|(n, b)| Member {
        nome: n,
        dim: b.len() as u64,
        dati: Box::new(b),
        sha1: None,
    })
    .collect();
    readers::apri(FileSet { membri: members }, &Limits::sito())
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

/// The scans and the TIC of a plain GC/MS run, as text: the engine and `agilent_ext` must give the same, so both builds compare it with one pinned value.
fn reading(l: &mut Box<dyn readers::Reader>) -> String {
    let ix = l.index(0).unwrap().to_vec();
    let mut out = String::new();
    for (j, v) in ix.iter().enumerate() {
        let s = l.scan(0, j).unwrap();
        let i: Vec<f64> = (0..s.mz.len()).map(|k| s.intensity_f64(k)).collect();
        out += &format!("{} {:?} {:?} {:?}\n", v.id, v.rt, s.mz, i);
    }
    let tic = l.chromatogram(0, "TIC").unwrap();
    out + &format!("TIC {:?} {:?}", tic.time, tic.intensity)
}

#[test]
fn a_plain_run_reads_the_same_on_both_builds() {
    let (scan, peak) = encode(&sample());
    let mut l = apri(scan, peak).expect("opens");
    let got = reading(&mut l);
    eprintln!("LETTURA\n{got}");
    assert_eq!(got, include_str!("agilent_percorso.txt").trim_end());
}

/// A folder with `MSProfile.bin` and no `MSPeak.bin`: the engine names the missing peaks with the hand-off key, and `agilent_ext` reads the profile file as the
/// CLI does, with the same scans and TIC as the plain run.
#[test]
fn a_profile_only_folder_takes_the_module_route() {
    let (scan, peak) = encode(&sample());
    let r = open_with(scan, "MSProfile.bin", peak);
    #[cfg(not(feature = "estesi"))]
    assert_eq!(r.err().expect("refused").key, "err.file.agilentNoPeaks");
    #[cfg(feature = "estesi")]
    assert_eq!(
        reading(&mut r.expect("opens")),
        include_str!("agilent_percorso.txt").trim_end()
    );
}
