//! The rows of mzML spectra written in the forms XML and mzML allow, against the truth the fixture was written with: attributes in any order and
//! quote, levels and polarity from a `referenceableParamGroup`, every retention time unit, the first of two scans, a product window that is not the
//! precursor, an MS3 that lists its immediate precursor first.

use super::*;

const HEAD: &str = r#"<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.hupo.org/ms/mzml" version="1.1.0"><referenceableParamGroupList count="1">
<referenceableParamGroup id="ms2neg"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="2"/><cvParam cvRef="MS" accession="MS:1000129" name="negative scan"/>
<cvParam cvRef="MS" accession="MS:1000128" name="profile spectrum"/></referenceableParamGroup></referenceableParamGroupList>"#;

fn rt(v: &str, unit: &str) -> String {
    format!(
        r#"<scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{v}" {unit}/></scan></scanList>"#
    )
}

fn sel(mz: f64, refer: &str) -> String {
    format!(
        r#"<precursor spectrumRef="{refer}"><selectedIonList count="1"><selectedIon><cvParam cvRef="MS" accession="MS:1000744" name="selected ion m/z" value="{mz}"/></selectedIon></selectedIonList></precursor>"#
    )
}

/// A non-indexed mzML with these spectrum bodies (what goes between the start tag and the end tag).
fn table(bodies: &[String]) -> Table {
    let mut s = format!(
        "{HEAD}<run id=\"r\"><spectrumList count=\"{}\">\n",
        bodies.len()
    );
    for (i, b) in bodies.iter().enumerate() {
        s.push_str(&format!(
            "<spectrum index=\"{i}\" id=\"scan={}\" defaultArrayLength=\"0\">{b}</spectrum>\n",
            i + 1
        ));
    }
    s.push_str("</spectrumList></run></mzML>\n");
    let mut data = s.into_bytes();
    let n = data.len() as u64;
    read(&mut data, n).unwrap()
}

const MIN: &str = r#"unitCvRef="UO" unitAccession="UO:0000031" unitName="minute""#;

#[test]
fn single_quotes_and_value_before_accession_are_read() {
    let t = table(&[format!(
        "<cvParam value='2' name='ms level' accession='MS:1000511' cvRef='MS'/><cvParam accession='MS:1000130' name='positive scan'/>{}",
        rt("5.5", "unitName='minute' unitAccession='UO:0000031'").replace('"', "'")
    )]);
    let r = &t.rows[0];
    assert_eq!((r.level, r.rt, r.polarity), (2, 5.5, 1));
}

#[test]
fn spectrum_terms_inherited_from_a_group_are_read() {
    let t = table(&[format!(
        "<referenceableParamGroupRef ref=\"ms2neg\"/>{}",
        rt("1", MIN)
    )]);
    let r = &t.rows[0];
    assert_eq!((r.level, r.polarity, r.profile), (2, -1, true));
}

#[test]
fn every_retention_time_unit_becomes_minutes() {
    let t = table(&[
        rt("0.5", r#"unitAccession="UO:0000032" unitName="hour""#),
        rt(
            "120000",
            r#"unitAccession="UO:0000028" unitName="millisecond""#,
        ),
        rt("90", r#"unitAccession="UO:0000010" unitName="second""#),
        rt("4", MIN),
    ]);
    let got: Vec<f64> = t.rows.iter().map(|r| r.rt).collect();
    assert_eq!(got, [30.0, 2.0, 1.5, 4.0]);
}

#[test]
fn the_first_of_two_scans_gives_the_time() {
    let two = format!(
        r#"<scanList count="2"><scan><cvParam accession="MS:1000016" value="1" {MIN}/></scan><scan><cvParam accession="MS:1000016" value="9" {MIN}/></scan></scanList>"#
    );
    assert_eq!(table(&[two]).rows[0].rt, 1.0);
}

#[test]
fn a_product_window_is_not_the_precursor() {
    let body = format!(
        r#"<cvParam accession="MS:1000511" value="2"/>{}<precursorList count="1">{}</precursorList><productList count="1"><product><isolationWindow><cvParam accession="MS:1000827" value="600"/></isolationWindow></product></productList>"#,
        rt("2", MIN),
        sel(400.0, "")
    );
    assert_eq!(table(&[body]).rows[0].precursor(), Some(400.0));
    // with no precursor at all, the product target is still not one
    let only_product = format!(
        r#"<cvParam accession="MS:1000511" value="2"/>{}<productList count="1"><product><isolationWindow><cvParam accession="MS:1000827" value="600"/></isolationWindow></product></productList>"#,
        rt("2", MIN)
    );
    assert_eq!(table(&[only_product]).rows[0].precursor(), None);
}

#[test]
fn the_immediate_precursor_is_the_stage_just_before() {
    let ms1 = format!(
        "<cvParam accession=\"MS:1000511\" value=\"1\"/>{}",
        rt("1", MIN)
    );
    let ms2 = format!(
        "<cvParam accession=\"MS:1000511\" value=\"2\"/>{}<precursorList count=\"1\">{}</precursorList>",
        rt("1.1", MIN),
        sel(500.3, "scan=1")
    );
    // listed with the immediate precursor first, then the MS1 stage
    let ms3_first = format!(
        "<cvParam accession=\"MS:1000511\" value=\"3\"/>{}<precursorList count=\"2\">{}{}</precursorList>",
        rt("1.2", MIN),
        sel(250.0, "scan=2"),
        sel(500.3, "scan=1")
    );
    // listed from the first stage to the last, without references
    let ms3_plain = format!(
        "<cvParam accession=\"MS:1000511\" value=\"3\"/>{}<precursorList count=\"2\">{}{}</precursorList>",
        rt("1.3", MIN),
        sel(500.3, ""),
        sel(250.0, "")
    );
    let t = table(&[ms1, ms2, ms3_first, ms3_plain]);
    assert_eq!(t.rows[2].precursor(), Some(250.0));
    assert_eq!(t.rows[3].precursor(), Some(250.0));
}
