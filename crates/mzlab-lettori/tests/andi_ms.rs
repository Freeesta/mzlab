//! The family `andi` reads ANDI-MS (ASTM E2077) and ANDI-Chromatography (ASTM E1947) files, built here with an encoder in every form of the format (docs/agenti/lettori-andi.md).
#![cfg(feature = "andi")]

mod comune;

use comune::andi::*;
use mzlab_core::lettori_mzml;
use mzlab_core::mzml::ChromKind;

fn controlla(c: &Corsa) {
    let mut l = apri_andi(andi_ms(c));
    assert_eq!(l.formato().id(), "andi-ms");
    assert_eq!(l.campioni()[0].scansioni, SCANSIONI);
    let indice = l.indice(0).unwrap().to_vec();
    for (k, voce) in indice.iter().enumerate() {
        let s = l.scansione(0, k).unwrap();
        let p = punti(k);
        assert_eq!(s.mz.len(), p.len(), "scan {k}");
        for (i, (mz, it)) in p.iter().enumerate() {
            // short m/z with a scale factor keeps the precision of the factor, the others are exact (floats within f32)
            let tol = if c.tipo_mz == 3 {
                0.5 * c.scala_mz.unwrap_or(1.0)
            } else {
                1e-3
            };
            assert!(
                (s.mz[i] - mz).abs() <= tol,
                "scan {k} point {i}: {} {mz}",
                s.mz[i]
            );
            assert!((s.intensity_f64(i) - it).abs() < 1e-3, "scan {k} point {i}");
        }
        assert!((s.rt - (60.0 + 1.5 * k as f64) / 60.0).abs() < 1e-9);
        assert_eq!(s.level, 1);
        assert_eq!(s.polarity, 1);
        assert_eq!(s.profile, c.profilo);
        assert!((voce.tic - p.iter().map(|x| x.1).sum::<f64>()).abs() < 1e-9);
        assert_eq!(l.extra_mzml(&s).sim, c.sim);
        assert_eq!(l.extra_mzml(&s).window, Some((40.0, 400.0)));
    }
}

#[test]
fn cdf1_e_cdf2_con_ogni_tipo_di_dato() {
    for v2 in [false, true] {
        for (tipo_mz, tipo_it) in [(5, 5), (6, 4), (5, 3), (4, 6)] {
            controlla(&Corsa {
                v2,
                tipo_mz,
                tipo_it,
                ..PIATTA
            });
        }
    }
}

#[test]
fn mz_corti_con_scale_factor_sim_e_profilo() {
    controlla(&Corsa {
        tipo_mz: 3,
        scala_mz: Some(0.25),
        ..PIATTA
    });
    controlla(&Corsa {
        sim: true,
        profilo: true,
        v2: true,
        ..PIATTA
    });
}

#[test]
fn la_polarita_negativa_si_legge() {
    let mut l = apri_andi(andi_ms(&Corsa {
        polarita: "Negative Polarity",
        ..PIATTA
    }));
    assert_eq!(l.scansione(0, 0).unwrap().polarity, -1);
}

#[test]
fn tic_e_bpc_vengono_dalle_scansioni() {
    let mut l = apri_andi(andi_ms(&PIATTA));
    let ids: Vec<String> = l
        .cromatogrammi(0)
        .unwrap()
        .into_iter()
        .map(|c| c.id)
        .collect();
    assert_eq!(ids, ["TIC", "BPC"]);
    let tic = l.cromatogramma(0, "TIC").unwrap();
    let bpc = l.cromatogramma(0, "BPC").unwrap();
    assert_eq!((tic.kind, bpc.kind), (ChromKind::Tic, ChromKind::Bpc));
    for k in 0..SCANSIONI {
        let p = punti(k);
        assert!((tic.intensity[k] - p.iter().map(|x| x.1).sum::<f64>()).abs() < 1e-9);
        assert!((bpc.intensity[k] - p.iter().map(|x| x.1).fold(0.0, f64::max)).abs() < 1e-9);
        assert!((tic.time[k] - (60.0 + 1.5 * k as f64) / 60.0).abs() < 1e-9);
    }
}

#[test]
fn l_intestazione_porta_le_variabili_della_scansione() {
    let mut l = apri_andi(andi_ms(&PIATTA));
    let c = l.intestazione(0, 2).unwrap();
    let v = |n: &str| c.iter().find(|x| x.nome == n).map(|x| x.valore.clone());
    assert_eq!(v("scan_acquisition_time").as_deref(), Some("63"));
    assert_eq!(v("mass_range_max").as_deref(), Some("400"));
    assert_eq!(
        v("point_count").as_deref(),
        Some(punti(2).len().to_string().as_str())
    );
    assert!(l.intestazione(0, SCANSIONI).is_err());
}

#[test]
fn il_metodo_porta_strumento_operatore_e_ogni_attributo() {
    let mut l = apri_andi(andi_ms(&PIATTA));
    let m = l.metodo(0).unwrap();
    for t in [
        "\"formato\":\"andi-ms\"",
        "\"file_creato\":\"20190312141102+0100\"",
        "\"produttore\":\"Testmaker\"",
        "\"modello\":\"Model 7\"",
        "\"utente\":\"Some Operator\"",
        "\"tipo\":\"Electron Impact\"",
        "\"chiave\":\"test_scan_function\"",
    ] {
        assert!(m.contains(t), "{t} in {m}");
    }
    // the operator and the serial number are marked as identifying
    let da = m.find("\"chiave\":\"operator_name\"").unwrap();
    assert!(m[da..]
        .split('}')
        .next()
        .unwrap()
        .contains("\"identita\":true"));
    let da = m.find("\"chiave\":\"instrument_serial_no\"").unwrap();
    assert!(m[da..]
        .split('}')
        .next()
        .unwrap()
        .contains("\"identita\":true"));
}

#[test]
fn si_scrive_come_mzml_e_si_rilegge() {
    let mut l = apri_andi(andi_ms(&PIATTA));
    let out = lettori_mzml::scrivi(&mut *l, 0, Vec::new(), &mut |_, _| {}).unwrap();
    let testo = String::from_utf8(out).unwrap();
    assert!(testo.contains("<spectrum "));
    assert!(testo.contains("Model 7"));
}

#[test]
fn un_cromatogramma_andi_e_un_campione_con_una_traccia_analogica() {
    let mut l = apri_andi(andi_cromato(40));
    assert_eq!(l.campioni()[0].scansioni, 0);
    assert!(l.indice(0).unwrap().is_empty());
    let voci = l.cromatogrammi(0).unwrap();
    assert_eq!(voci.len(), 1);
    assert_eq!(
        (voci[0].id.as_str(), voci[0].tipo, voci[0].unita.as_str()),
        ("FID1A", ChromKind::Analog, "Volts")
    );
    let c = l.cromatogramma(0, "FID1A").unwrap();
    assert_eq!(c.time.len(), 40);
    assert!((c.time[2] - (30.0 + 1.0) / 60.0).abs() < 1e-12);
    assert!((c.intensity[3] - (1000.0 + 21.0)).abs() < 1e-9);
    assert!(l.cromatogramma(0, "altro").is_err());
    let out = lettori_mzml::scrivi(&mut *l, 0, Vec::new(), &mut |_, _| {}).unwrap();
    assert!(String::from_utf8(out).unwrap().contains("FID1A"));
}

#[test]
fn riconosce_ms_con_certezza_e_cromatografia_con_meno() {
    use mzlab_core::lettori::Fiducia;
    let mut a = comune::insieme("a.cdf", andi_ms(&PIATTA));
    let mut b = comune::insieme("b.cdf", andi_cromato(5));
    let mut c = comune::insieme("c.cdf", b"CDF\x01\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0".to_vec());
    assert_eq!(
        (mzlab_lettori::andi::ms::VOCE.riconosci)(&mut a),
        Fiducia::Si
    );
    assert_eq!(
        (mzlab_lettori::andi::ms::VOCE.riconosci)(&mut b),
        Fiducia::Forse
    );
    assert_eq!(
        (mzlab_lettori::andi::ms::VOCE.riconosci)(&mut c),
        Fiducia::No
    );
}
