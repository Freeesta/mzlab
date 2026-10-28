//! The tags and attributes `mzmlhead` looks for may come with any XML white space and in any attribute order.

use base64::Engine;
use mzlab_core::mzmlhead::Mzml;

fn b64(v: &[f64]) -> String {
    base64::engine::general_purpose::STANDARD
        .encode(v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>())
}

fn file(sep: &str, time_cv: &str) -> Vec<u8> {
    let t = b64(&[60.0, 120.0, 180.0]);
    let y = b64(&[1.0, 2.0, 3.0]);
    format!(
        "<mzML><header/><run{sep}id=\"r\"><chromatogramList count=\"1\"><chromatogram{sep}index=\"0\"{sep}id=\"TIC\"{sep}defaultArrayLength=\"3\">\
<binaryDataArrayList count=\"2\"><binaryDataArray{sep}encodedLength=\"0\">\
<cvParam accession=\"MS:1000523\" name=\"64-bit float\"/>{time_cv}<binary>{t}</binary></binaryDataArray>\
<binaryDataArray><cvParam accession=\"MS:1000523\" name=\"64-bit float\"/><cvParam accession=\"MS:1000515\" name=\"intensity array\"/><binary>{y}</binary></binaryDataArray>\
</binaryDataArrayList></chromatogram></chromatogramList></run></mzML>"
    )
    .into_bytes()
}

fn read(bytes: Vec<u8>) -> (Vec<f64>, Vec<f64>) {
    let size = bytes.len() as u64;
    let mut m = Mzml::new(bytes, size);
    let chroms = m.chromatograms(0).unwrap();
    assert_eq!(chroms.len(), 1);
    m.chromatogram(&chroms[0]).unwrap()
}

#[test]
fn time_in_seconds_whatever_the_attribute_order_and_value() {
    let variants = [
        r#"<cvParam cvRef="MS" accession="MS:1000595" name="time array" value="" unitCvRef="UO" unitAccession="UO:0000010" unitName="second"/>"#,
        r#"<cvParam cvRef="MS" accession="MS:1000595" name="time array" unitCvRef="UO" unitAccession="UO:0000010"/>"#,
        r#"<cvParam unitAccession="UO:0000010" unitCvRef="UO" value="" name="time array" accession="MS:1000595" cvRef="MS"/>"#,
    ];
    for cv in variants {
        let (t, _) = read(file(" ", cv));
        assert_eq!(t, vec![1.0, 2.0, 3.0], "{cv}");
    }
    let minutes = r#"<cvParam accession="MS:1000595" name="time array" unitAccession="UO:0000031" unitName="minute"/>"#;
    assert_eq!(read(file(" ", minutes)).0, vec![60.0, 120.0, 180.0]);
}

#[test]
fn tags_accept_any_white_space_after_the_name() {
    let cv = r#"<cvParam accession="MS:1000595" name="time array"/>"#;
    for sep in [" ", "\n", "\r\n  ", "\t"] {
        let (t, y) = read(file(sep, cv));
        assert_eq!(
            (t, y),
            (vec![60.0, 120.0, 180.0], vec![1.0, 2.0, 3.0]),
            "{sep:?}"
        );
    }
}

#[test]
fn run_head_stops_at_a_run_tag_closed_at_once() {
    let bytes = b"<mzML><header>H</header><run>R</run></mzML>".to_vec();
    let size = bytes.len() as u64;
    let mut m = Mzml::new(bytes, size);
    assert_eq!(m.run_head(0).unwrap(), "<mzML><header>H</header>");
}
