//! The module `aperti` against hostile mzXML files, and (with the private data repository, `MZLAB_DATI`) the mzXML files of msconvert against the mzML of the same raw file; the real-data
//! tests are skipped without it.
#![cfg(feature = "aperti")]

mod comune;

use comune::mzxml::*;
use comune::{b64, insieme, zlib};
use mzlab_core::lettori::Limiti;
use std::path::PathBuf;

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

fn testo(o: Opz) -> String {
    String::from_utf8(mzxml(&corsa(), o)).unwrap()
}

#[test]
fn gli_offset_dell_indice_non_contano_le_scansioni_vengono_dalla_passata() {
    let buono = testo(PIATTO);
    for danno in [
        ("<offset id=\"1\">", "<offset id=\"1\">9"),
        ("<offset id=\"5\">", "<offset id=\"5\">99999"),
    ] {
        let b = buono.replacen(danno.0, danno.1, 1);
        let mut l = apri_mzxml(b.into_bytes());
        assert_eq!(l.campioni()[0].scansioni, 5);
        assert_eq!(l.scansione(0, 4).unwrap().mz.len(), 10);
    }
    let fuori = buono.replacen("<indexOffset>", "<indexOffset>9999999", 1);
    assert_eq!(apri_mzxml(fuori.into_bytes()).campioni()[0].scansioni, 5);
}

#[test]
fn un_peaks_count_enorme_e_un_errore_senza_allocare() {
    let b = testo(PIATTO).replacen("peaksCount=\"30\"", "peaksCount=\"4000000000\"", 1);
    assert_eq!(errore_mzxml(b.into_bytes()), "error.memoryBudget");
}

#[test]
fn piu_picchi_di_quelli_dichiarati_si_tagliano_o_sono_un_errore_se_compressi() {
    let semplice = testo(Opz {
        zlib: false,
        ..PIATTO
    })
    .replacen("peaksCount=\"30\"", "peaksCount=\"4\"", 1);
    assert_eq!(
        apri_mzxml(semplice.into_bytes())
            .scansione(0, 0)
            .unwrap()
            .mz
            .len(),
        4
    );
    let compresso = testo(PIATTO).replacen("peaksCount=\"30\"", "peaksCount=\"4\"", 1);
    assert_eq!(errore_mzxml(compresso.into_bytes()), "error.memoryBudget");
}

#[test]
fn uno_zlib_che_si_gonfia_oltre_il_dichiarato_e_un_errore() {
    let b = testo(PIATTO);
    let da = b.find("compressedLen=\"").unwrap();
    let a = da + b[da..].find('>').unwrap() + 1;
    let fine = a + b[a..].find("</peaks>").unwrap();
    let bomba = b64(&zlib(&vec![0u8; 8 << 20]));
    let b = format!("{}{}{}", &b[..a], bomba, &b[fine..]);
    assert_eq!(errore_mzxml(b.into_bytes()), "error.memoryBudget");
}

#[test]
fn base64_rotta_precisione_ignota_e_dati_troncati_sono_errori_con_chiave() {
    let b = testo(Opz {
        zlib: false,
        indice: false,
        ..PIATTO
    });
    let da = b.find("compressedLen=\"0\">").unwrap() + 18;
    let rotta = format!("{}!!!{}", &b[..da], &b[da + 3..]);
    assert_eq!(errore_mzxml(rotta.into_bytes()), "err.file.base64");
    let strana = b.replacen("precision=\"64\"", "precision=\"16\"", 1);
    assert_eq!(errore_mzxml(strana.into_bytes()), "err.file.xml");
    let ordine = b.replacen("contentType=\"m/z-int\"", "contentType=\"int-m/z\"", 1);
    assert_eq!(errore_mzxml(ordine.into_bytes()), "err.file.xml");
    // truncated in the middle of the peaks: an error or a short scan, never a panic
    let taglio = b[..da + 100].to_string();
    let r = errore_mzxml(taglio.into_bytes());
    assert!(r == "ok" || r.starts_with("err."), "{r}");
}

#[test]
fn un_annidamento_profondo_e_un_errore_con_chiave() {
    let mut b = String::from(TESTATA);
    for k in 0..400 {
        b += &format!(
            "<scan num=\"{}\" msLevel=\"1\" retentionTime=\"PT1S\">",
            k + 1
        );
    }
    b += &"</scan>".repeat(400);
    b += "</msRun></mzXML>";
    assert_eq!(errore_mzxml(b.into_bytes()), "err.dam.unreadable");
}

#[test]
fn senza_scansioni_o_con_un_altro_xml_non_si_apre() {
    let vuoto = format!("{}</msRun></mzXML>", TESTA_SENZA_SCAN());
    assert_eq!(errore_mzxml(vuoto.into_bytes()), "err.file.xml");
    let r = mzlab_lettori::apri(insieme("x.mzXML", b"<altro/>".to_vec()), &Limiti::sito());
    assert_eq!(r.err().map(|e| e.key), Some("err.file.notAccepted"));
}

#[allow(non_snake_case)]
fn TESTA_SENZA_SCAN() -> &'static str {
    TESTATA
}

#[test]
fn nessuna_lunghezza_del_file_manda_in_panico_il_lettore() {
    let b = mzxml(
        &corsa(),
        Opz {
            a64: false,
            ..PIATTO
        },
    );
    for n in (0..b.len()).step_by(97) {
        let _ = errore_mzxml(b[..n].to_vec());
    }
}

// ---- real files (private data repository) ----

/// The pairs (mzXML, mzML of the same raw file) of `dati/formati/aperti/mzXML/`.
fn coppie() -> Vec<(PathBuf, PathBuf)> {
    let Some(d) = dati_percorsi::percorso("dati/formati/aperti/mzXML") else {
        eprintln!("SKIP aperti_mzxml: no data repository (MZLAB_DATI)");
        return Vec::new();
    };
    let mut out = Vec::new();
    for e in std::fs::read_dir(d)
        .into_iter()
        .flatten()
        .flatten()
    {
        let p = e.path();
        if p.extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("mzxml"))
        {
            let m = ["mzML", "mzml"]
                .iter()
                .map(|x| p.with_extension(x))
                .find(|m| m.exists());
            out.extend(m.map(|m| (p.clone(), m)));
        }
    }
    out
}

fn mzml_spettri(m: &PathBuf) -> Vec<mzlab_core::mzml::Spectrum> {
    let mut v = Vec::new();
    let f = std::fs::File::open(m).unwrap();
    mzlab_core::mzml::parse(std::io::BufReader::new(f), |it| {
        if let mzlab_core::mzml::Item::Spectrum(s) = it {
            v.push(s);
        }
    })
    .unwrap();
    v
}

/// The mzXML of msconvert and its mzML: the same scans, m/z within 1e-6, intensities within 1e-6 (relative), the same RT, level, precursor and polarity.
#[test]
fn ogni_mzxml_vero_dice_quello_che_dice_il_suo_mzml() {
    for (x, m) in coppie() {
        let nome = x.file_name().unwrap().to_string_lossy().into_owned();
        let mut l = apri_mzxml(std::fs::read(&x).unwrap());
        let rif = mzml_spettri(&m);
        assert_eq!(l.campioni()[0].scansioni, rif.len(), "{nome}");
        for (k, r) in rif.iter().enumerate() {
            let s = l.scansione(0, k).unwrap();
            assert_eq!(
                (s.level, s.polarity),
                (r.level, r.polarity),
                "{nome} scan {k}"
            );
            assert!(
                (s.rt - r.rt).abs() < 1e-4,
                "{nome} scan {k} rt {} {}",
                s.rt,
                r.rt
            );
            assert_eq!(
                s.precursor.is_some(),
                r.precursor.is_some(),
                "{nome} scan {k}"
            );
            if let (Some(a), Some(b)) = (s.precursor, r.precursor) {
                assert!((a - b).abs() < 1e-4, "{nome} scan {k} precursor");
            }
            assert_eq!(s.mz.len(), r.mz.len(), "{nome} scan {k}");
            for i in 0..s.mz.len() {
                assert!(
                    (s.mz[i] - r.mz[i]).abs() <= 1e-6 * r.mz[i].max(1.0),
                    "{nome} scan {k} mz {i}"
                );
                let (a, b) = (s.intensity_f64(i), r.intensity_f64(i));
                assert!(
                    (a - b).abs() <= 1e-6 * b.abs().max(1.0),
                    "{nome} scan {k} int {i}"
                );
            }
        }
    }
}

/// Every real mzXML of the data repository (no mzML beside it): scans counted, peaks sorted and finite, the header read.
#[test]
fn i_mzxml_veri_si_aprono_e_i_conti_tornano() {
    let Some(d) = dati_percorsi::percorso("dati/formati/aperti/mzXML") else {
        eprintln!("SKIP aperti_mzxml: no data repository (MZLAB_DATI)");
        return;
    };
    for e in std::fs::read_dir(d)
        .into_iter()
        .flatten()
        .flatten()
    {
        let p = e.path();
        if !p
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("mzxml"))
        {
            continue;
        }
        let nome = p.file_name().unwrap().to_string_lossy().into_owned();
        let mut l = apri_mzxml(std::fs::read(&p).unwrap());
        let n = l.campioni()[0].scansioni;
        assert!(n > 0, "{nome}");
        let attese = [("PXD070375", 1096), ("PXD005665", 5344)];
        if let Some((_, v)) = attese.iter().find(|(k, _)| nome.starts_with(k)) {
            assert_eq!(n, *v, "{nome}");
        }
        let tic: Vec<f64> = l.indice(0).unwrap().iter().map(|v| v.tic).collect();
        for k in (0..n).step_by((n / 20).max(1)) {
            let s = l.scansione(0, k).unwrap();
            assert!(s.mz.windows(2).all(|w| w[0] <= w[1]), "{nome} scan {k}");
            assert!(
                (0..s.mz.len()).all(|i| s.intensity_f64(i).is_finite() && s.mz[i] > 0.0),
                "{nome} scan {k}"
            );
            assert!(tic[k].is_finite() && tic[k] >= 0.0, "{nome} scan {k} tic");
        }
        assert!(l.metodo(0).unwrap().contains("\"mzxml\""), "{nome}");
    }
}
