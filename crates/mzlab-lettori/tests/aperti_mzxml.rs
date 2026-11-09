//! The module `aperti` reads an mzXML (docs/agenti/lettori-aperti.md): files built here in every form (32 and 64 bit, zlib or not, with and without `<index>`, flat and nested), SRM scans,
//! the `metodo/1` document and the header of the mzML we write. The real-data parity is in `aperti_mzxml_hostile.rs` (it needs `MZLAB_DATI`).
#![cfg(feature = "aperti")]

mod common;

use common::mzxml::*;
use common::{file_set, written_mzml};
use mzlab_core::lettori::Fiducia;
use mzlab_core::mzml::ChromKind;

fn options() -> Vec<Opz> {
    let mut v = Vec::new();
    for a64 in [false, true] {
        for zlib in [false, true] {
            for index in [false, true] {
                for nested in [false, true] {
                    v.push(Opz {
                        a64,
                        zlib,
                        index,
                        nested,
                        attributes: true,
                    });
                }
            }
        }
    }
    v
}

#[test]
fn every_shape_opens_and_returns_as_written() {
    let scan = run();
    for o in options() {
        let source = if o.nested { nest(&scan) } else { scan.clone() };
        let mut l = open_mzxml(mzxml(&source, o));
        assert_eq!(l.samples()[0].scansioni, scan.len());
        for (k, x) in scan.iter().enumerate() {
            let s = l.scan(0, k).unwrap();
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
            assert_eq!(s.level, x.level);
            assert_eq!(s.polarity, 1);
            assert_eq!(s.precursor, x.prec);
            assert!((s.rt - x.rt_s / 60.0).abs() < 1e-12);
            assert_eq!(s.id, format!("scan={}", x.num));
            assert!(!s.profile);
        }
    }
}

#[test]
fn the_index_gives_level_precursor_and_tic_without_reading_peaks() {
    let scan = run();
    let mut l = open_mzxml(mzxml(&scan, FLAT));
    let ind = l.index(0).unwrap();
    assert_eq!(ind.len(), 5);
    assert_eq!(ind[1].livello, 2);
    assert_eq!(ind[1].precursore, Some(412.25));
    assert!((ind[3].tic - scan[3].it.iter().sum::<f64>()).abs() < 1e-9);
    assert!(ind[1].position.is_some());
}

#[test]
fn tic_and_bpc_are_computed_from_peaks_when_the_file_omits_them() {
    let scan = run();
    let mut l = open_mzxml(mzxml(
        &scan,
        Opz {
            attributes: false,
            ..FLAT
        },
    ));
    let entries = l.chromatograms(0).unwrap();
    assert!(entries
        .iter()
        .any(|v| v.id == "TIC" && v.tipo == ChromKind::Tic));
    let tic = l.chromatogram(0, "TIC").unwrap();
    let one: Vec<&Sc> = scan.iter().filter(|x| x.level == 1).collect();
    assert_eq!(tic.time.len(), one.len());
    for (i, x) in one.iter().enumerate() {
        assert!((tic.intensity[i] - x.it.iter().sum::<f64>()).abs() < 1e-9);
        assert!((tic.time[i] - x.rt_s / 60.0).abs() < 1e-12);
    }
    let bpc = l.chromatogram(0, "BPC").unwrap();
    assert!((bpc.intensity[0] - one[0].it.iter().copied().fold(0.0, f64::max)).abs() < 1e-9);
    assert!(l.chromatogram(0, "XIC").is_err());
}

#[test]
fn srm_scans_become_chromatograms() {
    let mut scan = Vec::new();
    for (k, rt) in [6.0, 7.0, 8.0].iter().enumerate() {
        for (j, (q1, q3)) in [(300.2, 150.1), (300.2, 200.3), (410.4, 120.0)]
            .iter()
            .enumerate()
        {
            let mut x = sc((k * 3 + j + 1) as u32, 2, *rt, 1, Some(*q1));
            x.mz = vec![*q3];
            x.it = vec![100.0 * (j + 1) as f64 + k as f64];
            x.kind = "SRM";
            scan.push(x);
        }
    }
    let mut l = open_mzxml(mzxml(&scan, FLAT));
    let srm: Vec<_> = l
        .chromatograms(0)
        .unwrap()
        .into_iter()
        .filter(|v| v.tipo == ChromKind::Srm)
        .collect();
    assert_eq!(srm.len(), 3);
    let c = l.chromatogram(0, &srm[0].id).unwrap();
    assert_eq!(c.time.len(), 3);
    assert_eq!(c.q1, Some(300.2));
    assert_eq!(c.q3, Some(150.1));
}

#[test]
fn recognises_by_bytes_not_by_name() {
    let mut good = file_set("x.mzXML", mzxml(&run(), FLAT));
    assert!(mzlab_lettori::riconosci(&mut good).is_some_and(|v| v.id == "mzxml"));
    let mut other = file_set("x.mzXML", b"<?xml version=\"1.0\"?><altro/>".to_vec());
    assert!(mzlab_lettori::riconosci(&mut other).is_none());
    let v = (mzlab_lettori::aperti::mzxml::ENTRY.riconosci)(&mut good);
    assert_eq!(v, Fiducia::Si);
}

#[test]
fn the_method_and_the_header_carry_instrument_file_and_software() {
    let mut l = open_mzxml(mzxml(&run(), FLAT));
    let m = l.method(0).unwrap();
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
    let h = l.mzml_header(0).unwrap().unwrap();
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
    let mzml = String::from_utf8(written_mzml(&mut l)).unwrap();
    assert!(mzml.contains("<spectrumList count=\"5\""));
    assert!(mzml.contains("Q Exactive"));
}

#[test]
fn the_header_carries_every_scan_attribute() {
    let mut l = open_mzxml(mzxml(&run(), FLAT));
    let fields = l.scan_header(0, 1).unwrap();
    let value = |n: &str| {
        fields
            .iter()
            .find(|c| c.nome == n)
            .map(|c| c.valore.clone())
    };
    assert_eq!(value("msLevel").as_deref(), Some("2"));
    assert_eq!(value("scanType").as_deref(), Some("Full"));
    assert_eq!(value("precursorMz.precursorCharge").as_deref(), Some("2"));
    assert_eq!(value("peaks.precision").as_deref(), Some("64"));
    assert_eq!(value("ion injection time").as_deref(), Some("12.5"));
}
