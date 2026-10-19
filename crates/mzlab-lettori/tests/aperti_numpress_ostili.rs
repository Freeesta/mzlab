//! The module `aperti` against hostile mzML files, and (with the private data repository, `MZLAB_DATI`) the numpress files of OpenMS and pymzML; the real-data tests are skipped without it.
#![cfg(feature = "aperti")]

mod comune;

use comune::*;
use mzlab_core::lettori::Limiti;
use mzlab_core::numpress::Np;

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

// ---- hostile files ----

#[test]
fn piu_valori_di_quelli_dichiarati_e_un_errore() {
    let scan = scansioni();
    let mut b = String::from_utf8(mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Slof,
            zlib: false,
        },
        None,
    ))
    .unwrap();
    b = b.replacen("defaultArrayLength=\"40\"", "defaultArrayLength=\"3\"", 1);
    assert_eq!(errore(b.into_bytes()), "err.file.numpress");
}

#[test]
fn uno_zlib_che_si_gonfia_oltre_il_dichiarato_e_un_errore() {
    let scan = scansioni();
    let mut b = String::from_utf8(mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Slof,
            zlib: true,
        },
        None,
    ))
    .unwrap();
    // an array of 40 values inflates to a few hundred bytes at most; this stream inflates to 8 MB of zeros
    let bomba = b64(&zlib(&vec![0u8; 8 << 20]));
    let da = b.find("<binary>").unwrap() + 8;
    let a = da + b[da..].find("</binary>").unwrap();
    b.replace_range(da..a, &bomba);
    assert_eq!(errore(b.into_bytes()), "error.memoryBudget");
}

#[test]
fn base64_rotta_e_dati_troncati_sono_errori_con_chiave() {
    let scan = scansioni();
    let ok = String::from_utf8(mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Slof,
            zlib: false,
        },
        None,
    ))
    .unwrap();
    let da = ok.find("<binary>").unwrap() + 8;
    let rotta = format!("{}!!!{}", &ok[..da], &ok[da + 3..]);
    assert_eq!(errore(rotta.into_bytes()), "err.file.base64");
    // a linear array cut to 5 bytes: shorter than its fixed point
    let a = da + ok[da..].find("</binary>").unwrap();
    let corta = format!("{}{}{}", &ok[..da], b64(&[1, 2, 3, 4, 5]), &ok[a..]);
    assert_eq!(errore(corta.into_bytes()), "err.file.numpress");
}

#[test]
fn un_file_troncato_non_manda_in_panico() {
    let scan = scansioni();
    let ok = mzml_di(
        &scan,
        Cod::Np {
            intensita: Np::Pic,
            zlib: true,
        },
        None,
    );
    for n in [0, 10, 700, 2500, ok.len() / 2, ok.len() - 5] {
        let _ = errore(ok[..n].to_vec());
    }
}

#[test]
fn righe_oltre_il_limite_sono_un_errore() {
    let scan = scansioni();
    let limiti = Limiti {
        voci: 2,
        ..Limiti::sito()
    };
    let r = mzlab_lettori::apri(
        insieme(
            "campione.mzML",
            mzml_di(
                &scan,
                Cod::Np {
                    intensita: Np::Pic,
                    zlib: false,
                },
                None,
            ),
        ),
        &limiti,
    );
    assert_eq!(r.err().map(|e| e.key), Some("error.memoryBudget"));
}

// ---- real files (the private data repository) ----

fn vero(nome: &str) -> Option<Vec<u8>> {
    let Some(d) = dati_percorsi::percorso("dati/formati/aperti/mzML_numpress") else {
        eprintln!("SKIP aperti_numpress {nome}: no data repository (MZLAB_DATI)");
        return None;
    };
    std::fs::read(d.join(nome)).ok()
}

/// The TIC an mzML header states against the sum of the decoded intensities.
#[test]
fn openms_lineare_e_slof_hanno_il_tic_dell_intestazione() {
    let Some(b) = vero("OpenMS_FileFilter_37_output.mzML") else {
        return;
    };
    let mut l = apri(b);
    let n = l.campioni()[0].scansioni;
    assert_eq!(n, 10);
    for k in 0..n {
        let s = l.scansione(0, k).unwrap();
        assert!(!s.mz.is_empty());
        assert!(s.mz.windows(2).all(|w| w[0] <= w[1]), "m/z sorted");
        assert!(s.intensity.iter().all(|x| *x >= 0.0));
        let tic = l.indice(0).unwrap()[k].tic;
        if tic > 0.0 {
            let somma: f64 = (0..s.mz.len()).map(|i| s.intensity_f64(i)).sum();
            assert!(
                (somma - tic).abs() <= tic * 1e-3,
                "scan {k}: sum {somma} TIC {tic}"
            );
        }
    }
    let t = String::from_utf8_lossy(&mzml_scritto(&mut l)).to_string();
    assert!(!t.contains("MS:1002312"));
}

#[test]
fn openms_lineare_e_pic_si_leggono() {
    let Some(b) = vero("OpenMS_FileFilter_38_output.mzML") else {
        return;
    };
    let mut l = apri(b);
    for k in 0..l.campioni()[0].scansioni {
        let s = l.scansione(0, k).unwrap();
        assert!(!s.mz.is_empty() && s.mz.windows(2).all(|w| w[0] <= w[1]));
        // pic gives whole numbers
        assert!(s.intensity.iter().all(|x| x.fract() == 0.0));
    }
}

#[test]
fn pymzml_cromatogrammi_srm_numpress() {
    let Some(b) = vero("pymzML_mini_numpress.chrom.mzML") else {
        return;
    };
    let mut l = apri(b);
    assert_eq!(l.campioni()[0].scansioni, 0);
    let v = l.cromatogrammi(0).unwrap();
    assert!(!v.is_empty());
    for c in v {
        let t = l.cromatogramma(0, &c.id).unwrap();
        assert_eq!(t.time.len(), t.intensity.len());
        assert!(!t.time.is_empty(), "{}", c.id);
        assert!(t.time.windows(2).all(|w| w[0] <= w[1]));
    }
}
