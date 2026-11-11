//! An `MSScan.xsd` that lists 20 000 fields: the records of `MSScan.bin` are read in time linear in the fields (every name is looked up through the index
//! of the layout), never one search of the whole field list per field of every record.

use mzlab_core::agilent::scan::{parse, Layout};

const FIELDS: usize = 20_000;
const RECORDS: usize = 50;

fn xsd() -> String {
    let mut t = String::from("<xs:schema><xs:complexType name=\"ScanRecordType\"><xs:sequence>");
    for n in ["ScanID", "ScanTime"] {
        let ty = if n == "ScanTime" { "double" } else { "int" };
        t += &format!("<xs:element name=\"{n}\" type=\"xs:{ty}\"/>");
    }
    for k in 0..FIELDS {
        t += &format!("<xs:element name=\"Extra{k}\" type=\"xs:int\"/>");
    }
    t += "<xs:element name=\"SpectrumParamValues\" type=\"SpectrumParamsType\"/></xs:sequence></xs:complexType>";
    t += "<xs:complexType name=\"SpectrumParamsType\"><xs:sequence>";
    for n in [
        "SpectrumFormatID",
        "SpectrumOffset",
        "ByteCount",
        "PointCount",
    ] {
        t += &format!("<xs:element name=\"{n}\" type=\"xs:int\"/>");
    }
    t + "</xs:sequence></xs:complexType></xs:schema>"
}

#[test]
fn a_layout_of_20000_fields_reads_its_records_in_linear_time() {
    let t = std::time::Instant::now();
    let layout = Layout::from_xsd(&xsd()).expect("layout");
    assert_eq!(layout.extra_names().len(), FIELDS);
    let mut bin = vec![0u8; 160];
    bin[88..92].copy_from_slice(&160i32.to_le_bytes());
    bin.resize(160 + RECORDS * layout.record_len, 0);
    let (recs, _) = parse(&bin, &layout, 0, None).expect("records");
    assert_eq!(recs.len(), RECORDS);
    assert!(recs.iter().all(|r| r.extra.len() == FIELDS));
    assert!(t.elapsed().as_secs() < 5, "{:?}", t.elapsed());
}
