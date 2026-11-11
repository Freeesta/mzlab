use super::super::lccal::{Calibration, Miss, MASS_CAL_HEAD};
use super::super::lcprofile::{lzf, run_length};

#[test]
fn run_length_skips_and_changes_width() {
    // header 4 bytes, −first = −2 (points 0 and 1 skipped), then width 4: 7; −(1<<2|1)−1: skip 1, width 2: 300
    let mut s = vec![0u8; 4];
    s.extend((-2i32).to_le_bytes());
    s.extend(7i32.to_le_bytes());
    s.extend((!(1i32 << 2 | 1)).to_le_bytes());
    s.extend(300i16.to_le_bytes());
    let y = run_length(&s, 6).unwrap();
    assert_eq!(y, vec![0, 0, 7, 0, 300, 0]);
}

#[test]
fn lzf_literal_and_back_reference() {
    // literal run of 3 bytes, then a back reference of length 4 at distance 3
    let s = [2u8, 1, 2, 3, 2 << 5, 2];
    assert_eq!(lzf(&s, 7).unwrap(), vec![1, 2, 3, 1, 2, 3, 1]);
    assert!(lzf(&s, 9).is_err());
}

#[test]
fn calibration_uses_the_scan_coefficients_and_never_others() {
    let xml = "<DefaultCalibration DefaultCalibrationID=\"1\"><Step Number=\"1\"><CalibrationFormula>Traditional</CalibrationFormula><ValueUseFlags>0</ValueUseFlags><Values><Value Number=\"1\">0.5</Value><Value Number=\"2\">2</Value></Values></Step><Step Number=\"2\"><CalibrationFormula>Polynomial</CalibrationFormula><ValueUseFlags>1</ValueUseFlags><Values><Value Number=\"1\">0</Value><Value Number=\"2\">100</Value><Value Number=\"3\">3</Value></Values></Step></DefaultCalibration>";
    // a scan without its own coefficients (MassCalOffset 0): the default, (0.5·(10−2))² − 3·10⁰ = 16 − 3
    let c = Calibration::parse(None, Some(xml.as_bytes()));
    assert_eq!(c.mz(Some(1), None, false, 10.0), Ok(13.0));
    assert_eq!(c.mz(None, None, false, 10.0), Ok(13.0));
    // one that has them and no MSMassCal.bin, or names a calibration that is not there: no m/z
    assert_eq!(c.mz(Some(1), Some(0), true, 10.0), Err(Miss::NoRecord));
    assert_eq!(c.mz(Some(2), None, false, 10.0), Err(Miss::NoDefault));
    let mut cal = vec![0u8; MASS_CAL_HEAD];
    cal.extend(10u32.to_le_bytes());
    for v in [1.0f64, 0.0, 0.0, 100.0, 1.0] {
        cal.extend(v.to_le_bytes());
    }
    cal.extend([0u8; 40]);
    // an emptied record (all zeros)
    cal.extend([0u8; 84]);
    let c = Calibration::parse(Some(&cal), Some(xml.as_bytes()));
    assert_eq!(c.mz(Some(1), Some(0), true, 10.0), Ok(99.0));
    assert_eq!(c.mz(Some(1), Some(1), true, 10.0), Err(Miss::NoRecord));
    assert_eq!(c.mz(Some(1), Some(5), true, 10.0), Err(Miss::NoRecord));
}
