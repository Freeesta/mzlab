//! The module `aperti` against hostile mzML files, and (with the private data repository, `MZLAB_DATI`) the numpress files of OpenMS and pymzML; the real-data tests are skipped without it.
#![cfg(feature = "aperti")]

mod common;

use common::*;
use mzlab_core::lettori::Limits;
use mzlab_core::numpress::Np;

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

// ---- hostile files ----

#[test]
fn more_values_than_declared_is_an_error() {
    let scan = scans();
    let mut b = String::from_utf8(mzml_di(
        &scan,
        Cod::Np {
            intensities: Np::Slof,
            zlib: false,
        },
        None,
    ))
    .unwrap();
    b = b.replacen("defaultArrayLength=\"40\"", "defaultArrayLength=\"3\"", 1);
    assert_eq!(error(b.into_bytes()), "err.file.numpress");
}

#[test]
fn a_zlib_inflating_past_the_declared_size_is_an_error() {
    let scan = scans();
    let mut b = String::from_utf8(mzml_di(
        &scan,
        Cod::Np {
            intensities: Np::Slof,
            zlib: true,
        },
        None,
    ))
    .unwrap();
    // an array of 40 values inflates to a few hundred bytes at most; this stream inflates to 8 MB of zeros
    let bomb = b64(&zlib(&vec![0u8; 8 << 20]));
    let da = b.find("<binary>").unwrap() + 8;
    let a = da + b[da..].find("</binary>").unwrap();
    b.replace_range(da..a, &bomb);
    assert_eq!(error(b.into_bytes()), "error.memoryBudget");
}

#[test]
fn broken_base64_and_truncated_data_are_keyed_errors() {
    let scan = scans();
    let ok = String::from_utf8(mzml_di(
        &scan,
        Cod::Np {
            intensities: Np::Slof,
            zlib: false,
        },
        None,
    ))
    .unwrap();
    let da = ok.find("<binary>").unwrap() + 8;
    let route = format!("{}!!!{}", &ok[..da], &ok[da + 3..]);
    assert_eq!(error(route.into_bytes()), "err.file.base64");
    // a linear array cut to 5 bytes: shorter than its fixed point
    let a = da + ok[da..].find("</binary>").unwrap();
    let short = format!("{}{}{}", &ok[..da], b64(&[1, 2, 3, 4, 5]), &ok[a..]);
    assert_eq!(error(short.into_bytes()), "err.file.numpress");
}

#[test]
fn a_truncated_file_does_not_panic() {
    let scan = scans();
    let ok = mzml_di(
        &scan,
        Cod::Np {
            intensities: Np::Pic,
            zlib: true,
        },
        None,
    );
    for n in [0, 10, 700, 2500, ok.len() / 2, ok.len() - 5] {
        let _ = error(ok[..n].to_vec());
    }
}

#[test]
fn rows_over_the_limit_are_an_error() {
    let scan = scans();
    let limits = Limits {
        entries: 2,
        ..Limits::sito()
    };
    let r = mzlab_lettori::apri(
        file_set(
            "campione.mzML",
            mzml_di(
                &scan,
                Cod::Np {
                    intensities: Np::Pic,
                    zlib: false,
                },
                None,
            ),
        ),
        &limits,
    );
    assert_eq!(r.err().map(|e| e.key), Some("error.memoryBudget"));
}

// ---- real files (the private data repository) ----

fn real(name: &str) -> Option<Vec<u8>> {
    let Some(d) = dati_percorsi::percorso("dati/formati/aperti/mzML_numpress") else {
        eprintln!("SKIP aperti_numpress {name}: no data repository (MZLAB_DATI)");
        return None;
    };
    std::fs::read(d.join(name)).ok()
}

/// The TIC an mzML header states against the sum of the decoded intensities.
#[test]
fn openms_linear_and_slof_have_the_header_tic() {
    let Some(b) = real("OpenMS_FileFilter_37_output.mzML") else {
        return;
    };
    let mut l = open(b);
    let n = l.samples()[0].scansioni;
    assert_eq!(n, 10);
    for k in 0..n {
        let s = l.scan(0, k).unwrap();
        assert!(!s.mz.is_empty());
        assert!(s.mz.windows(2).all(|w| w[0] <= w[1]), "m/z sorted");
        assert!(s.intensity.iter().all(|x| *x >= 0.0));
        let tic = l.index(0).unwrap()[k].tic;
        if tic > 0.0 {
            let summed: f64 = (0..s.mz.len()).map(|i| s.intensity_f64(i)).sum();
            assert!(
                (summed - tic).abs() <= tic * 1e-3,
                "scan {k}: sum {summed} TIC {tic}"
            );
        }
    }
    let t = String::from_utf8_lossy(&written_mzml(&mut l)).to_string();
    assert!(!t.contains("MS:1002312"));
}

#[test]
fn openms_linear_and_pic_are_read() {
    let Some(b) = real("OpenMS_FileFilter_38_output.mzML") else {
        return;
    };
    let mut l = open(b);
    for k in 0..l.samples()[0].scansioni {
        let s = l.scan(0, k).unwrap();
        assert!(!s.mz.is_empty() && s.mz.windows(2).all(|w| w[0] <= w[1]));
        // pic gives whole numbers
        assert!(s.intensity.iter().all(|x| x.fract() == 0.0));
    }
}

#[test]
fn pymzml_srm_numpress_chromatograms() {
    let Some(b) = real("pymzML_mini_numpress.chrom.mzML") else {
        return;
    };
    let mut l = open(b);
    assert_eq!(l.samples()[0].scansioni, 0);
    let v = l.chromatograms(0).unwrap();
    assert!(!v.is_empty());
    for c in v {
        let t = l.chromatogram(0, &c.id).unwrap();
        assert_eq!(t.time.len(), t.intensity.len());
        assert!(!t.time.is_empty(), "{}", c.id);
        assert!(t.time.windows(2).all(|w| w[0] <= w[1]));
    }
}
