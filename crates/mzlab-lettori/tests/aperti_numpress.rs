//! The module `aperti` reads an mzML with MS-Numpress arrays (docs/agenti/lettori-aperti.md): files built here with the encoders of `mzlab_core::numpress`, hostile ones, and (with the
//! private data repository, `MZLAB_DATI`) the numpress files of OpenMS and pymzML. Without it the real-data tests are skipped.
#![cfg(feature = "aperti")]
#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

mod comune;

use comune::*;
use mzlab_core::lettori::Limiti;
use mzlab_core::mzml;
use mzlab_core::numpress::{self, Np};

#[test]
fn every_codec_opens_and_round_trips_within_its_precision() {
    let scan = scansioni();
    for (intensities, z) in [
        (Np::Slof, false),
        (Np::Slof, true),
        (Np::Pic, false),
        (Np::Pic, true),
    ] {
        let mut l = apri(mzml_di(
            &scan,
            Cod::Np {
                intensities,
                zlib: z,
            },
            None,
        ));
        assert_eq!(l.samples()[0].scansioni, scan.len());
        for (k, x) in scan.iter().enumerate() {
            let s = l.scan(0, k).unwrap();
            assert_eq!(s.mz.len(), x.mz.len());
            let fp = numpress::linear_fixed_point(&x.mz);
            for (a, b) in s.mz.iter().zip(&x.mz) {
                assert!((a - b).abs() <= 0.5 / fp + 1e-9, "mz {a} {b}");
            }
            for i in 0..s.mz.len() {
                let (a, b) = (s.intensity_f64(i), x.intensities[i]);
                let tol = match intensities {
                    Np::Pic => 0.5,
                    Np::Slof => {
                        (b + 1.0) * ((0.5 / numpress::slof_fixed_point(&x.intensities)).exp() - 1.0)
                            + 1e-6
                    }
                    Np::Linear => 0.5 / numpress::linear_fixed_point(&x.intensities) + 1e-6,
                };
                assert!(
                    (a - b).abs() <= tol,
                    "{intensities:?} {z} intensity {a} {b}"
                );
            }
            assert_eq!(s.level, x.level);
            assert_eq!(s.precursor, x.precursore);
            assert!((s.rt - x.rt_min).abs() < 1e-9);
        }
    }
}

#[test]
fn the_index_gives_level_precursor_tic_without_reading_peaks() {
    let scan = scansioni();
    let mut l = apri(mzml_di(
        &scan,
        Cod::Np {
            intensities: Np::Slof,
            zlib: false,
        },
        None,
    ));
    let ind = l.index(0).unwrap();
    assert_eq!(ind.len(), scan.len());
    for (v, x) in ind.iter().zip(&scan) {
        assert_eq!(v.livello, x.level);
        assert_eq!(v.precursore, x.precursore);
        assert_eq!(v.polarita, 1);
        assert!((v.tic - x.intensities.iter().sum::<f64>()).abs() < 1e-6);
        assert!(v.id.starts_with("controllerType=0"));
        assert!(v.posizione.is_some_and(|p| p.lunghezza > 0));
    }
}

#[test]
fn rewritten_as_zlib_mzml_without_numpress_equals_its_twin() {
    let scan = scansioni();
    let mut np = apri(mzml_di(
        &scan,
        Cod::Np {
            intensities: Np::Slof,
            zlib: true,
        },
        None,
    ));
    let written = written_mzml(&mut np);
    let text = String::from_utf8_lossy(&written).to_string();
    for a in [
        "MS:1002312",
        "MS:1002313",
        "MS:1002314",
        "MS:1002746",
        "MS:1002747",
        "MS:1002748",
    ] {
        assert!(!text.contains(a), "{a} still in the output");
    }
    let run = mzml::read(&written[..]).unwrap();
    let gemello = mzml::read(&mzml_di(&scan, Cod::Piano, None)[..]).unwrap();
    assert_eq!(run.spectra.len(), gemello.spectra.len());
    for (a, b) in run.spectra.iter().zip(&gemello.spectra) {
        assert_eq!(
            (a.level, a.polarity, a.precursor, a.profile),
            (b.level, b.polarity, b.precursor, b.profile)
        );
        assert_eq!(a.id, b.id);
        assert!((a.rt - b.rt).abs() < 1e-9);
        assert_eq!(a.mz.len(), b.mz.len());
        for i in 0..a.mz.len() {
            assert!((a.mz[i] - b.mz[i]).abs() < 1e-4);
            let (x, y) = (a.intensity_f64(i), b.intensity_f64(i));
            assert!((x - y).abs() <= y * 1e-3 + 1e-3, "{x} {y}");
        }
    }
    // the TIC chromatogram is decoded too (numpress arrays after the spectra)
    assert_eq!(run.chromatograms.len(), 1);
    assert_eq!(run.chromatograms[0].time.len(), scan.len());
    // the scan window and the collision energy of the MS2 survive
    assert!(text.contains("scan window lower limit"));
    assert!(text.contains("MS:1000045 collision energy"));
}

#[test]
fn the_original_header_stays_and_mzlab_writes() {
    let scan = scansioni();
    let mut np = apri(mzml_di(
        &scan,
        Cod::Np {
            intensities: Np::Slof,
            zlib: false,
        },
        None,
    ));
    let t = String::from_utf8_lossy(&written_mzml(&mut np)).to_string();
    assert!(t.contains("MS:1001911"), "instrument model");
    assert!(t.contains("SN-4711"), "serial number");
    assert!(
        t.contains("campione.raw") && t.contains("0123456789abcdef0123456789abcdef01234567"),
        "source and SHA-1"
    );
    assert!(t.contains("MS:1000563"), "source format");
    assert!(
        t.contains("ProteoWizard software") && t.contains("3.0.1"),
        "original software"
    );
    assert!(t.contains("startTimeStamp=\"2024-03-05T10:11:12Z\""));
    assert!(
        t.contains("electrospray ionization")
            && t.contains("orbitrap")
            && t.contains("inductive detector")
    );
    assert!(
        t.contains("<software id=\"mzlab\"") && t.contains("mzLab"),
        "mzLab as the converter"
    );
    assert!(
        !t.contains("mzLab metodo/1"),
        "no document when the original had none"
    );
}

#[test]
fn the_method_repeats_only_if_the_mzml_carries_it() {
    let scan = scansioni();
    let doc = "{\"schema\":\"metodo/1\",\"strumento\":{\"modello\":\"x\"}}";
    let mut con = apri(mzml_di(
        &scan,
        Cod::Np {
            intensities: Np::Pic,
            zlib: false,
        },
        Some(doc),
    ));
    assert_eq!(con.method(0).unwrap(), doc);
    let t = String::from_utf8_lossy(&written_mzml(&mut con)).to_string();
    assert!(t.contains("mzLab metodo/1"));
    let mut without = apri(mzml_di(
        &scan,
        Cod::Np {
            intensities: Np::Pic,
            zlib: false,
        },
        None,
    ));
    assert_eq!(without.method(0).unwrap(), "{}");
}

#[test]
fn an_mzml_without_numpress_or_another_file_is_not_its_own() {
    let scan = scansioni();
    let flat = mzml_di(&scan, Cod::Piano, None);
    assert_eq!(error(flat), "err.file.notAccepted");
    assert_eq!(
        error(b"<?xml version=\"1.0\"?><x/>".to_vec()),
        "err.file.notAccepted"
    );
    assert_eq!(error(vec![0xAB; 4096]), "err.file.notAccepted");
}

#[test]
fn numpress_chromatograms_are_read() {
    let scan = scansioni();
    let mut l = apri(mzml_di(
        &scan,
        Cod::Np {
            intensities: Np::Linear,
            zlib: false,
        },
        None,
    ));
    let v = l.chromatograms(0).unwrap();
    assert_eq!(v.len(), 1);
    let c = l.chromatogram(0, &v[0].id).unwrap();
    assert_eq!(c.time.len(), scan.len());
    assert!((c.time[1] - 0.75).abs() < 1e-3);
}

/// `convert.mzml` form (`indexedmzML`): reading it back gives the same scans as reading the plain mzML written from the same reader.
#[test]
fn the_indexed_mzml_of_the_convert_step_reads_back_like_the_plain_one() {
    use mzlab_core::lettori_mzml;
    use mzlab_core::mzmlwrite::Opzioni;
    let mut src = apri(mzml_di(
        &scansioni(),
        Cod::Np {
            intensities: Np::Slof,
            zlib: true,
        },
        None,
    ));
    // this module opens an mzML only when it holds numpress arrays
    let np = Opzioni {
        numpress: true,
        ..Opzioni::default()
    };
    let plain = lettori_mzml::scrivi_con(&mut *src, 0, Vec::new(), &mut |_, _| {}, np).unwrap();
    let indexed =
        lettori_mzml::scrivi_indicizzato(&mut *src, 0, Vec::new(), &mut |_, _| {}, np, &[])
            .unwrap();
    assert!(
        indexed.starts_with(b"<?xml") && String::from_utf8_lossy(&indexed).contains("<indexedmzML")
    );
    let (mut a, mut b) = (apri(plain), apri(indexed));
    assert_eq!(a.index(0).unwrap().len(), b.index(0).unwrap().len());
    for k in 0..a.index(0).unwrap().len() {
        let (x, y) = (a.scan(0, k).unwrap(), b.scan(0, k).unwrap());
        assert!(x.mz == y.mz && x.intensity == y.intensity && x.rt == y.rt && x.id == y.id);
    }
}

/// A numpress mzML cut in the middle of the spectrum list opens with the spectra it holds, and `mancanti` counts those the run header declared.
#[test]
fn a_truncated_numpress_declares_the_missing_spectra() {
    let scan = scansioni();
    let cod = Cod::Np {
        intensities: Np::Slof,
        zlib: false,
    };
    let b = mzml_di(&scan, cod, None);
    let mut whole = mzlab_lettori::apri(file_set("a.mzML", b.clone()), &Limiti::sito()).unwrap();
    assert_eq!(whole.missing(0), 0);
    let mut l = mzlab_lettori::apri(
        file_set("a.mzML", b[..b.len() * 3 / 4].to_vec()),
        &Limiti::sito(),
    )
    .unwrap();
    let ha = l.samples()[0].scansioni;
    assert!(ha > 0 && ha < scan.len(), "{ha} of {}", scan.len());
    assert_eq!(ha + l.missing(0), scan.len());
}
