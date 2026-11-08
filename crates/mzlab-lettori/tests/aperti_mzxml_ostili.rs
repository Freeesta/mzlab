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
        assert_eq!(l.samples()[0].scansioni, 5);
        assert_eq!(l.scan(0, 4).unwrap().mz.len(), 10);
    }
    let fuori = buono.replacen("<indexOffset>", "<indexOffset>9999999", 1);
    assert_eq!(apri_mzxml(fuori.into_bytes()).samples()[0].scansioni, 5);
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
            .scan(0, 0)
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
    // named .mzXML but not one: a cut or damaged copy, said with its name and size; an empty one is told apart
    let r = mzlab_lettori::apri(insieme("x.mzXML", b"<altro/>".to_vec()), &Limiti::sito());
    let e = r.err().unwrap();
    assert_eq!((e.key, e.params.len()), ("err.file.notThisFormat", 3));
    let r = mzlab_lettori::apri(insieme("x.mzXML", Vec::new()), &Limiti::sito());
    assert_eq!(r.err().map(|e| e.key), Some("err.file.empty"));
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

/// A run of `n` MS1 scans whose `scanCount` says `dichiarate` (`None`: no attribute).
fn lunga(n: u32, dichiarate: Option<u32>, o: Opz) -> Vec<u8> {
    let v: Vec<Sc> = (1..=n)
        .map(|k| sc(k, 1, 6.0 + k as f64, 30, None))
        .collect();
    let conta = dichiarate.map_or(String::new(), |d| format!("scanCount=\"{d}\" "));
    let t = String::from_utf8(mzxml(&v, o)).unwrap();
    t.replacen("scanCount=\"5\" ", &conta, 1).into_bytes()
}

/// The scans a damaged copy still gives: each is the same as in the whole file, and with `mancanti` they make up the run.
fn parte_intatta(intero: &[u8], rotto: Vec<u8>, n: usize) -> usize {
    let mut tutto = apri_mzxml(intero.to_vec());
    let mut l = apri_mzxml(rotto);
    let righe = l.index(0).unwrap().to_vec();
    assert!(righe.len() < n, "a damaged copy reads fewer scans");
    for (k, r) in righe.iter().enumerate() {
        let j = tutto
            .index(0)
            .unwrap()
            .iter()
            .position(|v| v.id == r.id)
            .unwrap();
        let (a, b) = (l.scan(0, k).unwrap(), tutto.scan(0, j).unwrap());
        assert_eq!((a.mz, a.intensity), (b.mz, b.intensity), "scan {}", r.id);
    }
    let m = l.missing(0);
    assert!(m > 0, "the scans a damaged copy lacks are counted");
    righe.len() + m
}

#[test]
fn una_copia_tagliata_o_azzerata_dice_quante_scansioni_mancano() {
    let n = 40;
    for o in [
        PIATTO,
        Opz {
            indice: false,
            zlib: false,
            ..PIATTO
        },
    ] {
        let b = lunga(n, Some(n), o);
        assert_eq!(
            apri_mzxml(b.clone()).missing(0),
            0,
            "the whole file lacks nothing"
        );
        for f in [0.5, 0.1] {
            let cut = b[..(b.len() as f64 * f) as usize].to_vec();
            assert_eq!(parte_intatta(&b, cut, n as usize), n as usize, "cut at {f}");
        }
        // a block of zeros: `scanCount` (or the index after the run) still says how many
        let mut z = b.clone();
        let l = z.len();
        z[l / 3..l / 2].fill(0);
        assert_eq!(parte_intatta(&b, z, n as usize), n as usize, "zeros");
    }
    // nothing declared: the scan the cut falls in is left out and counted
    let b = lunga(
        n,
        None,
        Opz {
            indice: false,
            ..PIATTO
        },
    );
    let da = String::from_utf8_lossy(&b).rfind("<peaks").unwrap();
    assert_eq!(
        parte_intatta(&b, b[..da + 40].to_vec(), n as usize),
        n as usize
    );
}

// ---- real files (private data repository) ----

/// The pairs (mzXML, mzML of the same raw file) of `dati/formati/aperti/mzXML/`.
fn coppie() -> Vec<(PathBuf, PathBuf)> {
    let Some(d) = dati_percorsi::percorso("dati/formati/aperti/mzXML") else {
        eprintln!("SKIP aperti_mzxml: no data repository (MZLAB_DATI)");
        return Vec::new();
    };
    let mut out = Vec::new();
    for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
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
        assert_eq!(l.samples()[0].scansioni, rif.len(), "{nome}");
        for (k, r) in rif.iter().enumerate() {
            let s = l.scan(0, k).unwrap();
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
    for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
        let p = e.path();
        if !p
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("mzxml"))
        {
            continue;
        }
        let nome = p.file_name().unwrap().to_string_lossy().into_owned();
        let mut l = apri_mzxml(std::fs::read(&p).unwrap());
        let n = l.samples()[0].scansioni;
        assert!(n > 0, "{nome}");
        let attese = [("PXD070375", 1096), ("PXD005665", 5344)];
        if let Some((_, v)) = attese.iter().find(|(k, _)| nome.starts_with(k)) {
            assert_eq!(n, *v, "{nome}");
        }
        let tic: Vec<f64> = l.index(0).unwrap().iter().map(|v| v.tic).collect();
        for k in (0..n).step_by((n / 20).max(1)) {
            let s = l.scan(0, k).unwrap();
            assert!(s.mz.windows(2).all(|w| w[0] <= w[1]), "{nome} scan {k}");
            assert!(
                (0..s.mz.len()).all(|i| s.intensity_f64(i).is_finite() && s.mz[i] > 0.0),
                "{nome} scan {k}"
            );
            assert!(tic[k].is_finite() && tic[k] >= 0.0, "{nome} scan {k} tic");
        }
        assert!(l.method(0).unwrap().contains("\"mzxml\""), "{nome}");
    }
}
