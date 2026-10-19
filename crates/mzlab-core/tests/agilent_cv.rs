//! Every cvParam that msconvert writes for the real runs must be in our mzML too (the audit of FILE_PRODUTTORI): the header, one spectrum of each kind
//! and the TIC chromatogram. The exceptions are listed here with the reason. Skipped without the data repository (`MZLAB_DATI`).

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mzlab_core::agilent::{AgilentD, Files};

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

/// msconvert writes things we leave out on purpose: the title of the spectrum (a text made of the file name), the instrument as a direct inlet (wrong for a GC),
/// the data processing, the software of the maker with the wrong version, the file-level indexing and the one SIC chromatogram per ion (derived data).
const KNOWN: &[(&str, &str)] = &[
    (
        "MS:1000796",
        "spectrum title: a text made of file and native ID names",
    ),
    (
        "MS:1000056",
        "direct inlet: msconvert's guess; a GC has no direct inlet",
    ),
    (
        "MS:1000569",
        "SHA-1 of the Contents.xml etc: we hash the files we read",
    ),
    (
        "MS:1000544",
        "conversion to mzML: ours has its own data processing",
    ),
    ("MS:1000615", "ProteoWizard software terms of msconvert"),
    ("MS:1000616", "preset scan configuration"),
    ("MS:1000624", "msconvert's own software name"),
    ("MS:1000629", "peak picking / processing steps of msconvert"),
    ("MS:1000035", "peak picking of msconvert"),
    ("MS:1000598", "indexing of msconvert"),
    ("MS:1001472", "SIM chromatogram: we write no SIC, the program builds XICs from the spectra"),
    ("MS:1000522", "64-bit integer array: the ms level array of msconvert's TIC (always 1; the level is in each spectrum)"),
    ("MS:1000786", "non-standard data array (ms level), as above"),
];

fn accessions(xml: &str) -> BTreeSet<String> {
    xml.split("accession=\"")
        .skip(1)
        .filter_map(|t| t.split('"').next())
        .map(String::from)
        .collect()
}

fn slice<'a>(x: &'a str, from: &str, to: &str) -> &'a str {
    let a = x.find(from).unwrap_or(0);
    let b = x[a..].find(to).map_or(x.len(), |i| a + i);
    &x[a..b]
}

fn ours(folder: &Path) -> String {
    let acq = folder.join("AcqData");
    let rd = |p: &Path| std::fs::read(p).ok();
    let (scan, xsd, ts, dev, con) = (
        rd(&acq.join("MSScan.bin")).unwrap(),
        rd(&acq.join("MSScan.xsd")),
        rd(&acq.join("MSTS.xml")),
        rd(&acq.join("Devices.xml")),
        rd(&acq.join("Contents.xml")),
    );
    let (meth, seq) = (
        rd(&folder.join("acqmeth.txt")),
        rd(&acq.join("sequence.xml")),
    );
    let mut peak = std::fs::read(acq.join("MSPeak.bin")).unwrap();
    let others = [("AcqData/sequence.xml", seq.as_deref().unwrap_or(&[]))];
    let files = Files {
        ms_scan: &scan,
        ms_scan_xsd: xsd.as_deref(),
        ms_peak_len: peak.len() as u64,
        ms_ts: ts.as_deref(),
        devices: dev.as_deref(),
        contents: con.as_deref(),
        acqmeth: meth.as_deref(),
        others: &others,
        ..Default::default()
    };
    let d = AgilentD::open(&folder.file_name().unwrap().to_string_lossy(), &files).unwrap();
    let mut out = Vec::new();
    let n = peak.len() as u64;
    d.to_mzml(&mut peak, n, &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

#[test]
fn every_cv_of_msconvert_is_in_our_mzml() {
    let Some(gc) = dati_percorsi::percorso("dati/GC-MS").filter(|g| g.join("mzML").is_dir()) else {
        eprintln!("SKIP agilent_cv: no data repository (MZLAB_DATI)");
        return;
    };
    let mut missing = Vec::new();
    for e in std::fs::read_dir(&gc).unwrap().flatten() {
        let folder = e.path();
        let Some(stem) = folder
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".D"))
            .map(String::from)
        else {
            continue;
        };
        let Ok(theirs) = std::fs::read_to_string(gc.join("mzML").join(format!("{stem}.mzML")))
        else {
            continue;
        };
        let mine = ours(&folder);
        let (have_head, have_spec, have_chrom) = (
            accessions(slice(&mine, "<mzML", "<spectrumList")),
            accessions(slice(&mine, "<spectrum ", "</spectrum>")),
            accessions(slice(&mine, "<chromatogram ", "</chromatogram>")),
        );
        let want = [
            (
                accessions(slice(&theirs, "<mzML", "<spectrumList")),
                have_head,
                "header",
            ),
            (
                accessions(slice(&theirs, "<spectrum ", "</spectrum>")),
                have_spec,
                "spectrum",
            ),
            (
                accessions(slice(&theirs, "<chromatogram ", "</chromatogram>")),
                have_chrom,
                "chromatogram",
            ),
        ];
        for (w, h, part) in want {
            missing.extend(
                w.difference(&h)
                    .filter(|a| !KNOWN.iter().any(|k| k.0 == a.as_str()))
                    .map(|a| format!("{stem} {part}: {a}")),
            );
        }
    }
    assert!(
        missing.is_empty(),
        "cvParams of msconvert that we do not write: {missing:?}"
    );
}
