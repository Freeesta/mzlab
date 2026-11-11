//! Texts read from vendor files that XML 1.0 forbids (U+FFFF, control characters) or that are not CV term names (a model in angle brackets):
//! the written file stays well-formed, the model goes in a userParam and is read back from it.
use super::*;
use crate::mzml_validate;
use crate::mzmlhead::Mzml;

fn written(h: &Header) -> Vec<u8> {
    let sp = Spectrum {
        index: 0,
        id: "scan=1".into(),
        level: 1,
        rt: 1.0,
        polarity: 1,
        mz: vec![100.5],
        intensity: vec![1.0],
        filter: "+ c \u{1} Full ms".into(),
        ..Default::default()
    };
    let mut w = MzmlWriter::indicizzato(Vec::new(), h, Options::default()).unwrap();
    w.open_spectra(1).unwrap();
    w.spectrum(&sp).unwrap();
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

fn vendor_header() -> Header {
    Header {
        run: "x\u{FFFE}".into(),
        instrument: Some(("MS:1000031".into(), "<unknown> & \"model\"".into())),
        serial: Some("SN\u{0}1".into()),
        sample: Some(("s\u{FFFF}".into(), "name\u{1B}".into())),
        run_params: vec![(
            "mzLab metodo/1".into(),
            "method \u{FFFF}\u{FFFE}\u{7} block".into(),
        )],
        ..Default::default()
    }
}

#[test]
fn characters_outside_xml_char_never_reach_the_file() {
    let b = written(&vendor_header());
    let xml = String::from_utf8(b.clone()).unwrap();
    let bad: Vec<char> = xml.chars().filter(|&c| !xml_char(c)).collect();
    assert!(bad.is_empty(), "characters XML 1.0 forbids: {bad:?}");
    assert!(xml.contains("method \u{FFFD}\u{FFFD}\u{FFFD} block"));
    let r = mzml_validate::validate(&b[..]).unwrap();
    assert!(r.is_valid(), "{:?}", r.problems);
}

#[test]
fn a_model_known_only_as_text_is_not_a_cv_term_name() {
    let xml = String::from_utf8(written(&vendor_header())).unwrap();
    assert!(xml.contains("accession=\"MS:1000031\" name=\"instrument model\" value=\"\"/><userParam name=\"instrument model\" value=\"&lt;unknown&gt; &amp; &quot;model&quot;\"/>"), "{xml}");
    let b = xml.into_bytes();
    let n = b.len() as u64;
    let meta = Mzml::new(b, n).metadata().unwrap();
    assert_eq!(meta.instrument, "<unknown> & \"model\"");
}

#[test]
fn a_cv_model_is_written_as_it_is() {
    let h = Header {
        run: "x".into(),
        instrument: Some((
            "MS:1000483".into(),
            "Thermo Fisher Scientific instrument model".into(),
        )),
        ..Default::default()
    };
    let xml = String::from_utf8(written(&h)).unwrap();
    assert!(xml.contains(
        "accession=\"MS:1000483\" name=\"Thermo Fisher Scientific instrument model\" value=\"\"/>"
    ));
    assert!(!xml.contains("userParam name=\"instrument model\""));
}

#[test]
fn a_maker_model_text_keeps_the_maker_term_with_its_cv_name() {
    let h = Header {
        run: "x".into(),
        instrument: Some(("MS:1000490".into(), "Agilent 0000 Model".into())),
        ..Default::default()
    };
    let b = written(&h);
    let xml = String::from_utf8(b.clone()).unwrap();
    assert!(xml.contains("accession=\"MS:1000490\" name=\"Agilent instrument model\" value=\"\"/><userParam name=\"instrument model\" value=\"Agilent 0000 Model\"/>"), "{xml}");
    assert!(mzml_validate::validate(&b[..]).unwrap().is_valid());
    let n = b.len() as u64;
    assert_eq!(
        Mzml::new(b, n).metadata().unwrap().instrument,
        "Agilent 0000 Model"
    );
}
