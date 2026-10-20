//! The module `aperti` reads an mzML with MS-Numpress arrays (docs/agenti/lettori-aperti.md): files built here with the encoders of `mzlab_core::numpress`, hostile ones, and (with the
//! private data repository, `MZLAB_DATI`) the numpress files of OpenMS and pymzML. Without it the real-data tests are skipped.
#![cfg(feature = "aperti")]
#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

mod comune;

use comune::*;
use mzlab_core::mzml;
use mzlab_core::numpress::{self, Np};

#[test]
fn ogni_codec_si_apre_e_torna_entro_la_sua_precisione() {
    let scan = scansioni();
    for (intensita, z) in [
        (Np::Slof, false),
        (Np::Slof, true),
        (Np::Pic, false),
        (Np::Pic, true),
    ] {
        let mut l = apri(mzml_di(&scan, Cod::Np { intensita, zlib: z }, None));
        assert_eq!(l.campioni()[0].scansioni, scan.len());
        for (k, x) in scan.iter().enumerate() {
            let s = l.scansione(0, k).unwrap();
            assert_eq!(s.mz.len(), x.mz.len());
            let fp = numpress::linear_fixed_point(&x.mz);
            for (a, b) in s.mz.iter().zip(&x.mz) {
                assert!((a - b).abs() <= 0.5 / fp + 1e-9, "mz {a} {b}");
            }
            for i in 0..s.mz.len() {
                let (a, b) = (s.intensity_f64(i), x.intensita[i]);
                let tol = match intensita {
                    Np::Pic => 0.5,
                    Np::Slof => {
                        (b + 1.0) * ((0.5 / numpress::slof_fixed_point(&x.intensita)).exp() - 1.0)
                            + 1e-6
                    }
                    Np::Linear => 0.5 / numpress::linear_fixed_point(&x.intensita) + 1e-6,
                };
                assert!((a - b).abs() <= tol, "{intensita:?} {z} intensity {a} {b}");
            }
            assert_eq!(s.level, x.livello);
            assert_eq!(s.precursor, x.precursore);
            assert!((s.rt - x.rt_min).abs() < 1e-9);
        }
    }
}

#[test]
fn l_indice_dice_livello_precursore_tic_senza_leggere_i_picchi() {
    let scan = scansioni();
    let mut l = apri(mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Slof,
            zlib: false,
        },
        None,
    ));
    let ind = l.indice(0).unwrap();
    assert_eq!(ind.len(), scan.len());
    for (v, x) in ind.iter().zip(&scan) {
        assert_eq!(v.livello, x.livello);
        assert_eq!(v.precursore, x.precursore);
        assert_eq!(v.polarita, 1);
        assert!((v.tic - x.intensita.iter().sum::<f64>()).abs() < 1e-6);
        assert!(v.id.starts_with("controllerType=0"));
        assert!(v.posizione.is_some_and(|p| p.lunghezza > 0));
    }
}

#[test]
fn riscritto_come_mzml_zlib_senza_numpress_e_uguale_al_gemello() {
    let scan = scansioni();
    let mut np = apri(mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Slof,
            zlib: true,
        },
        None,
    ));
    let scritto = mzml_scritto(&mut np);
    let testo = String::from_utf8_lossy(&scritto).to_string();
    for a in [
        "MS:1002312",
        "MS:1002313",
        "MS:1002314",
        "MS:1002746",
        "MS:1002747",
        "MS:1002748",
    ] {
        assert!(!testo.contains(a), "{a} still in the output");
    }
    let run = mzml::read(&scritto[..]).unwrap();
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
    assert!(testo.contains("scan window lower limit"));
    assert!(testo.contains("MS:1000045 collision energy"));
}

#[test]
fn la_testata_originale_resta_e_mzlab_scrive() {
    let scan = scansioni();
    let mut np = apri(mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Slof,
            zlib: false,
        },
        None,
    ));
    let t = String::from_utf8_lossy(&mzml_scritto(&mut np)).to_string();
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
fn il_metodo_si_ripete_solo_se_l_mzml_lo_porta() {
    let scan = scansioni();
    let doc = "{\"schema\":\"metodo/1\",\"strumento\":{\"modello\":\"x\"}}";
    let mut con = apri(mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Pic,
            zlib: false,
        },
        Some(doc),
    ));
    assert_eq!(con.metodo(0).unwrap(), doc);
    let t = String::from_utf8_lossy(&mzml_scritto(&mut con)).to_string();
    assert!(t.contains("mzLab metodo/1"));
    let mut senza = apri(mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Pic,
            zlib: false,
        },
        None,
    ));
    assert_eq!(senza.metodo(0).unwrap(), "{}");
}

#[test]
fn un_mzml_senza_numpress_o_un_altro_file_non_e_suo() {
    let scan = scansioni();
    let piano = mzml_di(&scan, Cod::Piano, None);
    assert_eq!(errore(piano), "err.file.notAccepted");
    assert_eq!(
        errore(b"<?xml version=\"1.0\"?><x/>".to_vec()),
        "err.file.notAccepted"
    );
    assert_eq!(errore(vec![0xAB; 4096]), "err.file.notAccepted");
}

#[test]
fn i_cromatogrammi_numpress_si_leggono() {
    let scan = scansioni();
    let mut l = apri(mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Linear,
            zlib: false,
        },
        None,
    ));
    let v = l.cromatogrammi(0).unwrap();
    assert_eq!(v.len(), 1);
    let c = l.cromatogramma(0, &v[0].id).unwrap();
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
            intensita: Np::Slof,
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
    assert_eq!(a.indice(0).unwrap().len(), b.indice(0).unwrap().len());
    for k in 0..a.indice(0).unwrap().len() {
        let (x, y) = (a.scansione(0, k).unwrap(), b.scansione(0, k).unwrap());
        assert!(x.mz == y.mz && x.intensity == y.intensity && x.rt == y.rt && x.id == y.id);
    }
}
