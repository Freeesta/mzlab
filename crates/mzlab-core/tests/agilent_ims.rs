//! An Agilent ion-mobility `.D` (the `MSScan.xsd` of IMS frames split into drift bins, no `SpectrumParamValues`) gets its own keyed notice on both builds;
//! synthetic xsd with the field names of that layout only, no laboratory data.

use mzlab_core::agilent::{AgilentD, Files};

const CAMPI: [(&str, &str); 22] = [
    ("ScanID", "int"),
    ("FrameID", "short"),
    ("BaseAbund", "int"),
    ("BaseMsBin", "int"),
    ("DetectorGain", "short"),
    ("DriftBin", "short"),
    ("FirstNonzeroMsBin", "int"),
    ("LastNonzeroMsBin", "int"),
    ("TIC", "double"),
    ("TfsBasePeakAbund", "double"),
    ("TfsBasePeakMz", "double"),
    ("MsProfSpecFmtId", "short"),
    ("MsProfByteCount", "int"),
    ("MsProfOffset", "long"),
    ("MsProfPointCount", "int"),
    ("MsProfFullByteCount", "int"),
    ("MsPeakSpecFmtId", "short"),
    ("MsPeakByteCount", "int"),
    ("MsPeakOffset", "long"),
    ("MsPeakPointCount", "int"),
    ("MsPeakMaxX", "double"),
    ("MsPeakMinX", "double"),
];

fn xsd() -> String {
    let els: String = CAMPI
        .iter()
        .map(|(n, t)| format!("<xs:element name=\"{n}\" type=\"xs:{t}\"/>"))
        .collect();
    format!(
        "<?xml version=\"1.0\"?><xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\"><xs:element name=\"ScanRecord\" type=\"ScanRecordType\"/>\
         <xs:complexType name=\"ScanRecordType\"><xs:sequence>{els}</xs:sequence></xs:complexType></xs:schema>"
    )
}

#[test]
fn ims_layout_is_named() {
    let mut scan = vec![0u8; 296 + 3 * 106];
    scan[88..92].copy_from_slice(&296i32.to_le_bytes());
    let x = xsd();
    let e = AgilentD::open(
        "x.D",
        &Files {
            ms_scan: &scan,
            ms_scan_xsd: Some(x.as_bytes()),
            ..Default::default()
        },
    )
    .err()
    .expect("refused");
    assert_eq!(e.key, "err.file.agilentIms", "{:?}", e.params);
    // Without the drift-bin field the layout is just incomplete.
    let x = x.replace("DriftBin", "OtherBin");
    let e = AgilentD::open(
        "x.D",
        &Files {
            ms_scan: &scan,
            ms_scan_xsd: Some(x.as_bytes()),
            ..Default::default()
        },
    )
    .err()
    .expect("refused");
    assert_eq!(e.key, "err.file.agilent");
}
