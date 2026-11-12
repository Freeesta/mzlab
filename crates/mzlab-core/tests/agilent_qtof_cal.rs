//! A synthetic Q-TOF `.D` (invented numbers, no lab data) with centroids stored as flight times: the m/z come from the scan's own record of
//! `MSMassCal.bin`; a scan whose `MassCalOffset` says it has one and finds none (file missing, empty, cut, record emptied), or whose `CalibrationID`
//! names no default, is `err.file.agilentCalibration`, never computed with other coefficients.
//! Every fragmentation spectrum carries the collision energy of its record in the activation of its precursor.
#![cfg(feature = "estesi")]

mod agilent_synth;

use agilent_synth::lc;
use mzlab_core::readers::{self, FileSet, Limits, Member};

const DEFAULT: &[u8] = b"<DefaultMassCal><DefaultCalibration DefaultCalibrationID=\"1\"><Step StepNumber=\"1\"><CalibrationFormula>Traditional</CalibrationFormula><Value Number=\"1\">0.2</Value><Value Number=\"2\">0</Value></Step></DefaultCalibration></DefaultMassCal>";

/// MSMassCal.bin: the 72-byte header, then per scan `u32` 10 and A, T0, tmin, tmax, c0..c5 (`None`: an emptied record).
fn mass_cal(a: &[Option<f64>]) -> Vec<u8> {
    let mut b = vec![0u8; 72];
    for x in a {
        match x {
            Some(a) => {
                b.extend(10u32.to_le_bytes());
                for v in [*a, 0.0, 0.0, 1e9, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] {
                    b.extend(v.to_le_bytes());
                }
            }
            None => b.extend([0u8; 84]),
        }
    }
    b
}

/// Two scans: a product-ion MS2 (level 2, CE 20 eV, precursor 300) and a full scan, each with two centroids at flight times 2000 and 3000.
fn folder(cal: Option<Vec<u8>>) -> FileSet {
    folder_id(cal, 1)
}

/// `folder` whose records name the default calibration `cal_id` (`DefaultMassCal.xml` has only 1).
fn folder_id(cal: Option<Vec<u8>>, cal_id: i32) -> FileSet {
    let mut peak = Vec::new();
    let mut recs = Vec::new();
    for (id, kind, level) in [(1, 512, 2i32), (2, 1, 1)] {
        let mut r = lc::record(id, kind, 2, peak.len() as i64, 24, 2, 24);
        r[20..24].copy_from_slice(&level.to_le_bytes());
        r[52..56].copy_from_slice(&cal_id.to_le_bytes());
        // MassCalOffset: the scan has its own record in MSMassCal.bin
        r[124..132].copy_from_slice(&(72 + 84 * (i64::from(id) - 1)).to_le_bytes());
        recs.push(r);
        for t in [2000.0f64, 3000.0] {
            peak.extend(t.to_le_bytes());
        }
        for y in [50.0f32, 80.0] {
            peak.extend(y.to_le_bytes());
        }
    }
    let m = |n: &str, b: Vec<u8>| Member {
        nome: format!("x.D/AcqData/{n}"),
        dim: b.len() as u64,
        dati: Box::new(b),
        sha1: None,
    };
    let mut membri = vec![
        m("MSScan.bin", lc::index(&recs)),
        m("MSPeak.bin", peak),
        m("MSScan.xsd", lc::xsd("DataDependentScanParamType")),
        m("DefaultMassCal.xml", DEFAULT.to_vec()),
    ];
    membri.extend(cal.map(|c| m("MSMassCal.bin", c)));
    FileSet { membri }
}

fn mzml(fs: FileSet) -> mzlab_core::error::Result<String> {
    let mut l = readers::apri(fs, &Limits::desktop())?;
    let mut out = Vec::new();
    mzlab_core::readers_mzml::scrivi(&mut *l, 0, &mut out, &mut |_, _| {})?;
    Ok(String::from_utf8(out).unwrap())
}

#[test]
fn the_masses_come_from_the_record_of_the_scan() {
    let mut l = readers::apri(
        folder(Some(mass_cal(&[Some(0.1), Some(0.2)]))),
        &Limits::desktop(),
    )
    .unwrap();
    // (A·t)²: scan 1 with its own A = 0.1, scan 2 with 0.2 (the default A is 0.2: scan 1 would be 4× off with it)
    assert_eq!(l.scan(0, 0).unwrap().mz, vec![40_000.0, 90_000.0]);
    assert_eq!(l.scan(0, 1).unwrap().mz, vec![160_000.0, 360_000.0]);
}

#[test]
fn a_run_with_no_usable_calibration_record_is_a_named_error() {
    for (what, cal, id) in [
        ("no MSMassCal.bin", None, 1),
        ("an empty MSMassCal.bin", Some(Vec::new()), 1),
        (
            "a CalibrationID with no default",
            Some(mass_cal(&[Some(0.1), Some(0.2)])),
            9,
        ),
    ] {
        let e = mzml(folder_id(cal, id))
            .err()
            .unwrap_or_else(|| panic!("{what}: no error"));
        assert_eq!(e.key, "err.file.agilentCalibration", "{what}");
    }
}

#[test]
fn a_scan_with_a_missing_cut_or_emptied_record_is_left_out_and_counted() {
    let whole = mass_cal(&[Some(0.1), Some(0.2)]);
    for (what, cal) in [
        ("a record short", mass_cal(&[Some(0.1)])),
        (
            "cut inside the last record",
            whole[..whole.len() - 30].to_vec(),
        ),
        ("an emptied record", mass_cal(&[Some(0.1), None])),
    ] {
        let mut l = readers::apri(folder(Some(cal)), &Limits::desktop()).unwrap();
        let mut out = Vec::new();
        mzlab_core::readers_mzml::scrivi(&mut *l, 0, &mut out, &mut |_, _| {})
            .unwrap_or_else(|e| panic!("{what}: {e}"));
        let x = String::from_utf8(out).unwrap();
        // scan 1 keeps its masses, scan 2 is in the list with no points
        assert_eq!(x.matches("<spectrum ").count(), 2, "{what}");
        assert!(x.contains("defaultArrayLength=\"0\""), "{what}");
        assert_eq!(
            l.losses(0).unwrap(),
            readers::Losses::of(2, 1, readers::SPECTRUM_DAMAGED),
            "{what}"
        );
    }
}

#[test]
fn every_fragmentation_spectrum_has_its_collision_energy() {
    let x = mzml(folder(Some(mass_cal(&[Some(0.1), Some(0.2)])))).unwrap();
    let pre: Vec<&str> = x.split("<precursorList").skip(1).collect();
    assert_eq!(pre.len(), 1, "one MS2 spectrum");
    let act = &pre[0][pre[0].find("<activation>").unwrap()..pre[0].find("</activation>").unwrap()];
    assert!(
        act.contains("accession=\"MS:1000045\" name=\"collision energy\" value=\"20\""),
        "{act}"
    );
    // the energy is in the activation, not repeated as a userParam
    assert!(
        !x.contains("name=\"collision energy\" value=\"20\" type"),
        "duplicate userParam"
    );
}
