//! `fit`: a document that fits is the same text; a long one, as an Orbitrap method of some 17 000 XML items and a 400 KB XML text makes, is cut
//! to the room and still parses, with its sections, the head of `campi`, the texts of the file and a `meta.troncato` warning for each cut.

use super::*;
use crate::json;
use crate::mzmlwrite::{Header, MzmlWriter};

/// A document like the one of a `.raw` with a big instrument method (synthetic: `n` fields, an XML text of `xml` bytes).
fn big(n: usize, xml: usize) -> String {
    let mut o = Out(String::new());
    o.raw("{\"schema\":\"metodo/1\",\"origine\":{\"formato\":\"thermo-raw\"},\"campi\":[");
    for i in 0..n {
        if i > 0 {
            o.raw(",");
        }
        o.raw("{\"origine\":\"thermo-raw:InstrumentMethod/XML\",\"chiave\":");
        o.str(&format!("CmData/Item{i}/Name@value"));
        o.raw(",\"valore\":");
        o.str(&format!("\"value\" <{i}> & more"));
        o.raw("}");
    }
    o.raw("],\"grezzo\":[{\"sezione\":\"Method LC\",\"testo\":\"Pump.Flow = 0.3\"},{\"sezione\":\"Method XML\",\"testo\":");
    o.str(&"<Item Name=\"x\" Value=\"1\"/>".repeat(xml / 26 + 1));
    o.raw("}],\"avvisi\":[]}");
    o.0
}

fn esc_len(s: &str) -> usize {
    crate::mzmlwrite::esc(s).len()
}

#[test]
fn a_document_that_fits_is_the_same_text() {
    let d = big(10, 1000);
    assert_eq!(fit(&d, DOC_MAX, &esc_len), d);
}

#[test]
fn a_long_document_is_cut_to_the_room_and_still_parses() {
    let d = big(17_000, 300_000);
    assert!(esc_len(&d) > 3 * DOC_MAX);
    let cut = fit(&d, DOC_MAX, &esc_len);
    assert!(esc_len(&cut) <= DOC_MAX, "{}", esc_len(&cut));
    let v = json::parse(cut.as_bytes()).unwrap();
    assert_eq!(v.get("schema").and_then(json::Value::str), Some("metodo/1"));
    // the fields go before the texts of the file: here the texts stay whole, and the head of `campi` is kept
    let texts: Vec<&str> = v
        .get("grezzo")
        .unwrap()
        .arr()
        .iter()
        .filter_map(|t| t.get("testo").and_then(json::Value::str))
        .collect();
    assert_eq!(texts[0], "Pump.Flow = 0.3");
    assert!(texts[1].len() >= 300_000, "{}", texts[1].len());
    let fields = v.get("campi").unwrap().arr();
    assert!(!fields.is_empty() && fields.len() < 17_000);
    assert_eq!(
        fields[0].get("chiave").and_then(json::Value::str),
        Some("CmData/Item0/Name@value")
    );
    let warnings = format!("{:?}", v.get("avvisi").unwrap());
    assert!(
        warnings.contains("meta.troncato")
            && warnings.contains("campi")
            && !warnings.contains("grezzo"),
        "{warnings}"
    );
}

/// Whatever a reader puts in it, the document of a written mzML ends within `DOC_MAX` bytes of `<run`, where every reader of it looks.
#[test]
fn every_written_document_is_read_back_by_the_readers() {
    let h = Header {
        run: "x".into(),
        sources: vec!["x.raw".into()],
        software: ("mzLab".into(), "0".into()),
        run_params: vec![
            ("opentfraw.computer_name".into(), "pc".into()),
            (crate::method::USERPARAM.into(), big(17_000, 400_000)),
        ],
        ..Default::default()
    };
    let mut w = MzmlWriter::new(Vec::new(), &h).unwrap();
    w.open_spectra(0).unwrap();
    w.close_spectra().unwrap();
    let out = w.finish().unwrap();
    let text = String::from_utf8_lossy(&out).into_owned();
    let run = text.find("<run ").unwrap();
    let head = &text[..(run + DOC_MAX).min(text.len())];
    let doc = crate::method::from_userparam(head)
        .expect("the document is within the head the readers read");
    assert!(json::parse(doc.as_bytes()).is_ok());
}
