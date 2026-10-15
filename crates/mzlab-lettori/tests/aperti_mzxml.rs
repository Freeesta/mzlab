//! The module `aperti` reads an mzXML (docs/agenti/lettori-aperti.md): files built here in every form (32 and 64 bit, zlib or not, with and without `<index>`, flat and nested), SRM scans,
//! the `metodo/1` document and the header of the mzML we write. The real-data parity is in `aperti_mzxml_ostili.rs` (it needs `MZLAB_DATI`).
#![cfg(feature = "aperti")]

mod comune;

use comune::mzxml::*;
use comune::{insieme, mzml_scritto};
use mzlab_core::lettori::Fiducia;
use mzlab_core::mzml::ChromKind;

fn opzioni() -> Vec<Opz> {
    let mut v = Vec::new();
    for a64 in [false, true] {
        for zlib in [false, true] {
            for indice in [false, true] {
                for annidato in [false, true] {
                    v.push(Opz {
                        a64,
                        zlib,
                        indice,
                        annidato,
                        attributi: true,
                    });
                }
            }
        }
    }
    v
}

#[test]
fn ogni_forma_si_apre_e_torna_come_e_stata_scritta() {
    let scan = corsa();
    for o in opzioni() {
        let sorgente = if o.annidato {
            annida(&scan)
        } else {
            scan.clone()
        };
        let mut l = apri_mzxml(mzxml(&sorgente, o));
        assert_eq!(l.campioni()[0].scansioni, scan.len());
        for (k, x) in scan.iter().enumerate() {
            let s = l.scansione(0, k).unwrap();
            assert_eq!(s.mz.len(), x.mz.len());
            for i in 0..s.mz.len() {
                if o.a64 {
                    assert_eq!(s.mz[i], x.mz[i]);
                    assert_eq!(s.intensity_f64(i), x.it[i]);
                } else {
                    assert_eq!(s.mz[i], f64::from(x.mz[i] as f32));
                    assert_eq!(s.intensity_f64(i), f64::from(x.it[i] as f32));
                }
            }
            assert_eq!(s.level, x.livello);
            assert_eq!(s.polarity, 1);
            assert_eq!(s.precursor, x.prec);
            assert!((s.rt - x.rt_s / 60.0).abs() < 1e-12);
            assert_eq!(s.id, format!("scan={}", x.num));
            assert!(!s.profile);
        }
    }
}

#[test]
fn l_indice_dice_livello_precursore_e_tic_senza_leggere_i_picchi() {
    let scan = corsa();
    let mut l = apri_mzxml(mzxml(&scan, PIATTO));
    let ind = l.indice(0).unwrap();
    assert_eq!(ind.len(), 5);
    assert_eq!(ind[1].livello, 2);
    assert_eq!(ind[1].precursore, Some(412.25));
    assert!((ind[3].tic - scan[3].it.iter().sum::<f64>()).abs() < 1e-9);
    assert!(ind[1].posizione.is_some());
}

#[test]
fn tic_e_bpc_si_calcolano_dai_picchi_quando_il_file_non_li_dice() {
    let scan = corsa();
    let mut l = apri_mzxml(mzxml(
        &scan,
        Opz {
            attributi: false,
            ..PIATTO
        },
    ));
    let voci = l.cromatogrammi(0).unwrap();
    assert!(voci
        .iter()
        .any(|v| v.id == "TIC" && v.tipo == ChromKind::Tic));
    let tic = l.cromatogramma(0, "TIC").unwrap();
    let uno: Vec<&Sc> = scan.iter().filter(|x| x.livello == 1).collect();
    assert_eq!(tic.time.len(), uno.len());
    for (i, x) in uno.iter().enumerate() {
        assert!((tic.intensity[i] - x.it.iter().sum::<f64>()).abs() < 1e-9);
        assert!((tic.time[i] - x.rt_s / 60.0).abs() < 1e-12);
    }
    let bpc = l.cromatogramma(0, "BPC").unwrap();
    assert!((bpc.intensity[0] - uno[0].it.iter().cloned().fold(0.0, f64::max)).abs() < 1e-9);
    assert!(l.cromatogramma(0, "XIC").is_err());
}

#[test]
fn le_scansioni_srm_diventano_cromatogrammi() {
    let mut scan = Vec::new();
    for (k, rt) in [6.0, 7.0, 8.0].iter().enumerate() {
        for (j, (q1, q3)) in [(300.2, 150.1), (300.2, 200.3), (410.4, 120.0)]
            .iter()
            .enumerate()
        {
            let mut x = sc((k * 3 + j + 1) as u32, 2, *rt, 1, Some(*q1));
            x.mz = vec![*q3];
            x.it = vec![100.0 * (j + 1) as f64 + k as f64];
            x.tipo = "SRM";
            scan.push(x);
        }
    }
    let mut l = apri_mzxml(mzxml(&scan, PIATTO));
    let srm: Vec<_> = l
        .cromatogrammi(0)
        .unwrap()
        .into_iter()
        .filter(|v| v.tipo == ChromKind::Srm)
        .collect();
    assert_eq!(srm.len(), 3);
    let c = l.cromatogramma(0, &srm[0].id).unwrap();
    assert_eq!(c.time.len(), 3);
    assert_eq!(c.q1, Some(300.2));
    assert_eq!(c.q3, Some(150.1));
}

#[test]
fn riconosce_dai_byte_non_dal_nome() {
    let mut buono = insieme("x.mzXML", mzxml(&corsa(), PIATTO));
    assert!(mzlab_lettori::riconosci(&mut buono).is_some_and(|v| v.id == "mzxml"));
    let mut altro = insieme("x.mzXML", b"<?xml version=\"1.0\"?><altro/>".to_vec());
    assert!(mzlab_lettori::riconosci(&mut altro).is_none());
    let v = (mzlab_lettori::aperti::mzxml::VOCE.riconosci)(&mut buono);
    assert_eq!(v, Fiducia::Si);
}

#[test]
fn il_metodo_e_la_testata_portano_strumento_file_e_programmi() {
    let mut l = apri_mzxml(mzxml(&corsa(), PIATTO));
    let m = l.metodo(0).unwrap();
    for t in [
        "\"formato\":\"mzxml\"",
        "Thermo Scientific",
        "Q Exactive",
        "Xcalibur",
        "msconvert",
        "\"tipo\":\"ESI\"",
        "Orbitrap",
        "\"utente\":\"Mario Verdi\"",
    ] {
        assert!(m.contains(t), "{t} missing in {m}");
    }
    let h = l.testata_mzml(0).unwrap().unwrap();
    assert_eq!(
        h.instrument.as_ref().map(|i| i.1.as_str()),
        Some("Q Exactive")
    );
    assert_eq!(h.sources, vec!["campione.raw".to_string()]);
    assert_eq!(
        h.source_sha1,
        vec!["0123456789abcdef0123456789abcdef01234567".to_string()]
    );
    assert!(h.run_params.iter().any(|(k, _)| k == "mzLab metodo/1"));
    let mzml = String::from_utf8(mzml_scritto(&mut l)).unwrap();
    assert!(mzml.contains("<spectrumList count=\"5\""));
    assert!(mzml.contains("Q Exactive"));
}

#[test]
fn l_intestazione_porta_ogni_attributo_della_scansione() {
    let mut l = apri_mzxml(mzxml(&corsa(), PIATTO));
    let campi = l.intestazione(0, 1).unwrap();
    let valore = |n: &str| campi.iter().find(|c| c.nome == n).map(|c| c.valore.clone());
    assert_eq!(valore("msLevel").as_deref(), Some("2"));
    assert_eq!(valore("scanType").as_deref(), Some("Full"));
    assert_eq!(valore("precursorMz.precursorCharge").as_deref(), Some("2"));
    assert_eq!(valore("peaks.precision").as_deref(), Some("64"));
    assert_eq!(valore("ion injection time").as_deref(), Some("12.5"));
}
