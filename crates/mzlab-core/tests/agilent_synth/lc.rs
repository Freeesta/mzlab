//! Q-TOF/QqQ-style `MSScan.xsd` and 220-byte records for the hostile-folder tests (layout of the real file, no code of the reader).

use super::{put_f64, put_i32};

pub fn xsd(extra_type: &str) -> Vec<u8> {
    let head = [
        ("ScanID", "int"),
        ("ScanMethodID", "int"),
        ("TimeSegmentID", "int"),
        ("ScanTime", "double"),
        ("MSLevel", "int"),
        ("ScanType", "int"),
        ("TIC", "double"),
        ("BasePeakMZ", "double"),
        ("BasePeakValue", "double"),
        ("CalibrationID", "int"),
        ("CycleNumber", "int"),
        ("IonMode", "int"),
        ("IonPolarity", "int"),
        ("Fragmentor", "double"),
        ("CollisionEnergy", "double"),
        ("MzOfInterest", "double"),
        ("AbundanceLimit", "double"),
        ("SamplingPeriod", "double"),
        ("Threshold", "double"),
        ("ChargeState", "int"),
        ("ChromScaleFactor", "float"),
        ("MassCalOffset", "long"),
        ("ActualsOffset", "long"),
        ("NumOfActualsPerScan", "int"),
    ];
    let mut t = String::from("<xs:schema><xs:complexType name=\"ScanRecordType\"><xs:sequence>");
    for (n, ty) in head {
        t += &format!("<xs:element name=\"{n}\" type=\"xs:{ty}\"/>");
    }
    t += &format!("<xs:element name=\"D\" type=\"{extra_type}\"/><xs:element name=\"SpectrumParamValues\" type=\"SpectrumParamsType\"/></xs:sequence></xs:complexType>");
    t += "<xs:complexType name=\"SpectrumParamsType\"><xs:sequence><xs:element name=\"SpectrumFormatID\" type=\"xs:int\"/><xs:element name=\"SpectrumOffset\" type=\"xs:long\"/><xs:element name=\"ByteCount\" type=\"xs:int\"/><xs:element name=\"PointCount\" type=\"xs:int\"/><xs:element name=\"UncompressedByteCount\" type=\"xs:int\"/><xs:element name=\"MinX\" type=\"xs:double\"/><xs:element name=\"MaxX\" type=\"xs:double\"/><xs:element name=\"MinY\" type=\"xs:double\"/><xs:element name=\"MaxY\" type=\"xs:double\"/><xs:element name=\"MeasuredNoise\" type=\"xs:double\"/></xs:sequence></xs:complexType>";
    t += "<xs:complexType name=\"DataDependentScanParamType\"><xs:sequence><xs:element name=\"DDScanID\" type=\"xs:int\"/><xs:element name=\"DDScanParamMz\" type=\"xs:double\"/></xs:sequence></xs:complexType></xs:schema>";
    t.into_bytes()
}

/// One record: (scan type, ms level, format, offset, bytes, points, uncompressed).
pub fn record(
    id: i32,
    kind: i32,
    format: i32,
    offset: i64,
    bytes: i32,
    points: i32,
    unc: i32,
) -> Vec<u8> {
    let mut b = Vec::new();
    for v in [id, 1, 1] {
        put_i32(&mut b, v);
    }
    put_f64(&mut b, id as f64 * 0.01);
    put_i32(&mut b, 1);
    put_i32(&mut b, kind);
    for v in [1.0, 100.0, 10.0] {
        put_f64(&mut b, v);
    }
    for v in [1, id, 1, 0] {
        put_i32(&mut b, v);
    }
    for v in [120.0, 20.0, 300.0, 0.0, 0.0, 0.0] {
        put_f64(&mut b, v);
    }
    put_i32(&mut b, 0);
    b.extend_from_slice(&1.0f32.to_le_bytes());
    b.extend_from_slice(&0i64.to_le_bytes());
    b.extend_from_slice(&0i64.to_le_bytes());
    put_i32(&mut b, 0);
    put_i32(&mut b, 0);
    put_f64(&mut b, 0.0);
    put_i32(&mut b, format);
    b.extend_from_slice(&offset.to_le_bytes());
    for v in [bytes, points, unc] {
        put_i32(&mut b, v);
    }
    for v in [0.0, 0.0, 0.0, 0.0, 0.0] {
        put_f64(&mut b, v);
    }
    assert_eq!(b.len(), 220);
    b
}

pub fn index(records: &[Vec<u8>]) -> Vec<u8> {
    let mut b = vec![0u8; 160];
    b[0] = 1;
    b[1] = 1;
    b[88..92].copy_from_slice(&160i32.to_le_bytes());
    records.iter().for_each(|r| b.extend_from_slice(r));
    b
}
