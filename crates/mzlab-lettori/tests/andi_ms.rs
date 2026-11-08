//! The family `andi` reads ANDI-MS (ASTM E2077) and ANDI-Chromatography (ASTM E1947) files, built here with an encoder in every form of the format (docs/agenti/lettori-andi.md).
#![cfg(feature = "andi")]

mod comune;

use comune::andi::*;
use mzlab_core::lettori_mzml;
use mzlab_core::mzml::ChromKind;

fn check(c: &Run) {
    let mut l = apri_andi(andi_ms(c));
    assert_eq!(l.format().id(), "andi-ms");
    assert_eq!(l.samples()[0].scansioni, SCANSIONI);
    let index = l.index(0).unwrap().to_vec();
    for (k, entry) in index.iter().enumerate() {
        let s = l.scan(0, k).unwrap();
        let p = points(k);
        assert_eq!(s.mz.len(), p.len(), "scan {k}");
        for (i, (mz, it)) in p.iter().enumerate() {
            // short m/z with a scale factor keeps the precision of the factor, the others are exact (floats within f32)
            let tol = if c.tipo_mz == 3 {
                0.5 * c.mz_scale.unwrap_or(1.0)
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
        assert!((entry.tic - p.iter().map(|x| x.1).sum::<f64>()).abs() < 1e-9);
        assert_eq!(l.extra_mzml(&s).sim, c.sim);
        assert_eq!(l.extra_mzml(&s).window, Some((40.0, 400.0)));
    }
}

#[test]
fn cdf1_and_cdf2_with_every_data_type() {
    for v2 in [false, true] {
        for (tipo_mz, tipo_it) in [(5, 5), (6, 4), (5, 3), (4, 6)] {
            check(&Run {
                v2,
                tipo_mz,
                tipo_it,
                ..FLAT_RUN
            });
        }
    }
}

#[test]
fn short_mz_with_scale_factor_sim_and_profile() {
    check(&Run {
        tipo_mz: 3,
        mz_scale: Some(0.25),
        ..FLAT_RUN
    });
    check(&Run {
        sim: true,
        profilo: true,
        v2: true,
        ..FLAT_RUN
    });
}

#[test]
fn negative_polarity_is_read() {
    let mut l = apri_andi(andi_ms(&Run {
        polarita: "Negative Polarity",
        ..FLAT_RUN
    }));
    assert_eq!(l.scan(0, 0).unwrap().polarity, -1);
}

#[test]
fn tic_and_bpc_come_from_the_scans() {
    let mut l = apri_andi(andi_ms(&FLAT_RUN));
    let ids: Vec<String> = l
        .chromatograms(0)
        .unwrap()
        .into_iter()
        .map(|c| c.id)
        .collect();
    assert_eq!(ids, ["TIC", "BPC"]);
    let tic = l.chromatogram(0, "TIC").unwrap();
    let bpc = l.chromatogram(0, "BPC").unwrap();
    assert_eq!((tic.kind, bpc.kind), (ChromKind::Tic, ChromKind::Bpc));
    for k in 0..SCANSIONI {
        let p = points(k);
        assert!((tic.intensity[k] - p.iter().map(|x| x.1).sum::<f64>()).abs() < 1e-9);
        assert!((bpc.intensity[k] - p.iter().map(|x| x.1).fold(0.0, f64::max)).abs() < 1e-9);
        assert!((tic.time[k] - (60.0 + 1.5 * k as f64) / 60.0).abs() < 1e-9);
    }
}

#[test]
fn the_header_carries_the_scan_variables() {
    let mut l = apri_andi(andi_ms(&FLAT_RUN));
    let c = l.scan_header(0, 2).unwrap();
    let v = |n: &str| c.iter().find(|x| x.nome == n).map(|x| x.valore.clone());
    assert_eq!(v("scan_acquisition_time").as_deref(), Some("63"));
    assert_eq!(v("mass_range_max").as_deref(), Some("400"));
    assert_eq!(
        v("point_count").as_deref(),
        Some(points(2).len().to_string().as_str())
    );
    assert!(l.scan_header(0, SCANSIONI).is_err());
}

#[test]
fn the_method_carries_instrument_operator_and_every_attribute() {
    let mut l = apri_andi(andi_ms(&FLAT_RUN));
    let m = l.method(0).unwrap();
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
fn writes_as_mzml_and_reads_back() {
    let mut l = apri_andi(andi_ms(&FLAT_RUN));
    let out = lettori_mzml::scrivi(&mut *l, 0, Vec::new(), &mut |_, _| {}).unwrap();
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("<spectrum "));
    assert!(text.contains("Model 7"));
}

#[test]
fn an_andi_chromatogram_is_a_sample_with_an_analog_trace() {
    let mut l = apri_andi(andi_cromato(40));
    assert_eq!(l.samples()[0].scansioni, 0);
    assert!(l.index(0).unwrap().is_empty());
    let entries = l.chromatograms(0).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        (
            entries[0].id.as_str(),
            entries[0].tipo,
            entries[0].unita.as_str()
        ),
        ("FID1A", ChromKind::Analog, "Volts")
    );
    let c = l.chromatogram(0, "FID1A").unwrap();
    assert_eq!(c.time.len(), 40);
    assert!((c.time[2] - (30.0 + 1.0) / 60.0).abs() < 1e-12);
    assert!((c.intensity[3] - (1000.0 + 21.0)).abs() < 1e-9);
    assert!(l.chromatogram(0, "altro").is_err());
    let out = lettori_mzml::scrivi(&mut *l, 0, Vec::new(), &mut |_, _| {}).unwrap();
    assert!(String::from_utf8(out).unwrap().contains("FID1A"));
}

#[test]
fn recognises_ms_with_certainty_and_chromatography_with_less() {
    use mzlab_core::lettori::Fiducia;
    let mut a = comune::file_set("a.cdf", andi_ms(&FLAT_RUN));
    let mut b = comune::file_set("b.cdf", andi_cromato(5));
    let mut c = comune::file_set("c.cdf", b"CDF\x01\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0".to_vec());
    assert_eq!(
        (mzlab_lettori::andi::ms::ENTRY.riconosci)(&mut a),
        Fiducia::Si
    );
    assert_eq!(
        (mzlab_lettori::andi::ms::ENTRY.riconosci)(&mut b),
        Fiducia::Forse
    );
    assert_eq!(
        (mzlab_lettori::andi::ms::ENTRY.riconosci)(&mut c),
        Fiducia::No
    );
}

#[test]
fn the_header_applies_scale_factor_and_add_offset() {
    let mut l = apri_andi(andi_ms(&Run {
        time_scale: Some((0.5, 10.0)),
        ..FLAT_RUN
    }));
    for k in 0..SCANSIONI {
        let fields = l.scan_header(0, k).unwrap();
        let t = fields
            .iter()
            .find(|c| c.nome == "scan_acquisition_time")
            .unwrap();
        let expected = 60.0 + 1.5 * k as f64;
        assert_eq!(t.valore.parse::<f64>().unwrap(), expected, "scan {k}");
    }
}

#[test]
fn the_index_and_the_time_axis_apply_scale_factor_and_add_offset() {
    let mut l = apri_andi(andi_ms(&Run {
        time_scale: Some((0.5, 10.0)),
        ..FLAT_RUN
    }));
    let rt = |k: usize| (60.0 + 1.5 * k as f64) / 60.0;
    for (k, entry) in l.index(0).unwrap().to_vec().iter().enumerate() {
        assert!(
            (entry.rt - rt(k)).abs() < 1e-9,
            "index scan {k}: {}",
            entry.rt
        );
        assert!((l.scan(0, k).unwrap().rt - rt(k)).abs() < 1e-9);
    }
    for id in ["TIC", "BPC"] {
        let c = l.chromatogram(0, id).unwrap();
        for (k, t) in c.time.iter().enumerate() {
            assert!((t - rt(k)).abs() < 1e-9, "{id} point {k}: {t}");
        }
    }
}

#[test]
fn point_time_and_analog_retention_apply_scale_factor_and_add_offset() {
    let scale = |v: &mut Vb| {
        v.att.push(("scale_factor".into(), At::N(6, vec![0.5])));
        v.att.push(("add_offset".into(), At::N(6, vec![10.0])));
    };
    // ANDI-MS without scan_acquisition_time: the time of a scan is `time_values` at its first point
    let n = 6;
    let dims = [("point_number", 0u32), ("scan_number", n as u32)];
    let mut tv = var(
        "time_values",
        &[0],
        6,
        (0..n)
            .map(|k| (60.0 + 1.5 * k as f64 - 10.0) / 0.5)
            .collect(),
    );
    scale(&mut tv);
    let vars = vec![
        var("scan_index", &[1], 4, (0..n).map(|k| k as f64).collect()),
        var("point_count", &[1], 4, vec![1.0; n]),
        var("mass_values", &[0], 6, vec![100.0; n]),
        var("intensity_values", &[0], 6, vec![5.0; n]),
        tv,
    ];
    let mut l = apri_andi(cdf(false, &dims, &globali_ms(&FLAT_RUN), &vars, n as u32));
    for (k, v) in l.index(0).unwrap().iter().enumerate() {
        assert!(
            (v.rt - (60.0 + 1.5 * k as f64) / 60.0).abs() < 1e-9,
            "time_values scan {k}: {}",
            v.rt
        );
    }
    // ANDI-Chromatography with a scaled raw_data_retention
    let m = 5;
    let dims = [("point_number", 0u32), ("_32_byte_string", 32)];
    let mut r = var(
        "raw_data_retention",
        &[0],
        6,
        (0..m).map(|i| (30.0 + i as f64 - 10.0) / 0.5).collect(),
    );
    scale(&mut r);
    let vars = vec![var("ordinate_values", &[0], 6, vec![1.0; m]), r];
    let g = vec![("detector_name".to_string(), t("FID1A"))];
    let mut l = apri_andi(cdf(false, &dims, &g, &vars, m as u32));
    let c = l.chromatogram(0, "FID1A").unwrap();
    for (i, x) in c.time.iter().enumerate() {
        assert!(
            (x - (30.0 + i as f64) / 60.0).abs() < 1e-9,
            "retention point {i}: {x}"
        );
    }
}
