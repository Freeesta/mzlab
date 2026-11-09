//! `conv/1` in the `<run>` of every conversion: the reader, the version, the fields counted and each gap the format declares with its reason; and
//! the traces of a reader that are not mass-spectrometer chromatograms (pressure, device) written with their term.

mod agilent_synth;
mod conv_aiuto;

use agilent_synth::{encode, sample};
use mzlab_core::json;
use mzlab_core::lettori::FileSet;
use mzlab_core::mzml::{self, ChromKind, Chromatogram};
use mzlab_core::mzmlwrite::{Header, MzmlWriter, Options};

/// The `conv/1` document of an mzML as `Conversione` writes it.
pub fn conv_di(xml: &[u8]) -> json::Value {
    let t = String::from_utf8_lossy(xml);
    let key = "name=\"mzLab conv/1\" value=\"";
    let v = &t[t.find(key).expect("conv/1 in the run") + key.len()..];
    let v = &v[..v.find("\"/>").unwrap()];
    let v = v
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    json::parse(v.as_bytes()).unwrap_or_else(|e| panic!("{e:?}: {v}"))
}

/// Every declared gap has a field and a reason; the document names the reader and the version of the program.
pub fn check(doc: &json::Value, reader: &str) {
    assert_eq!(doc.get("schema").and_then(|s| s.str()), Some("conv/1"));
    assert_eq!(doc.get("lettore").and_then(|s| s.str()), Some(reader));
    assert_eq!(
        doc.get("versione").and_then(|s| s.str()),
        Some(env!("CARGO_PKG_VERSION"))
    );
    assert!(doc.get("campi_cv").and_then(|n| n.f64()).is_some());
    assert!(doc.get("campi_user").and_then(|n| n.f64()).is_some());
    for g in doc.get("non_portati").expect("non_portati").arr() {
        let field = g.get("campo").and_then(|s| s.str()).unwrap_or("");
        let reason = g.get("motivo").and_then(|s| s.str()).unwrap_or("");
        assert!(
            !field.is_empty() && !reason.is_empty(),
            "a gap without a reason: {field:?}"
        );
    }
}

#[test]
fn an_agilent_conversion_carries_conv_with_its_gaps_and_reasons() {
    let (idx, peak) = encode(&sample());
    let xml = conv_aiuto::mzml(FileSet {
        membri: vec![
            conv_aiuto::member("x.D/AcqData/MSScan.bin", &idx),
            conv_aiuto::member("x.D/AcqData/MSPeak.bin", &peak),
        ],
    });
    let doc = conv_di(&xml);
    check(&doc, "agilent-d");
    assert!(
        !doc.get("non_portati").unwrap().arr().is_empty(),
        "the format declares what it leaves out"
    );
}

#[test]
fn pressure_and_device_traces_are_written_with_their_term_and_unit() {
    let trace = |id: &str, kind, unit: &str| Chromatogram {
        index: 0,
        id: id.into(),
        kind,
        q1: None,
        q3: None,
        unit: unit.into(),
        time: vec![0.0, 1.0, 2.0],
        intensity: vec![10.0, 20.0, 30.0],
    };
    let mut w =
        MzmlWriter::with_options(Vec::new(), &Header::default(), Options::default()).unwrap();
    w.open_chromatograms(2).unwrap();
    w.chromatogram(&trace("Pump Pressure", ChromKind::Pressure, "pascal"))
        .unwrap();
    w.chromatogram(&trace(
        "Column temperature",
        ChromKind::Analog,
        "degree Celsius",
    ))
    .unwrap();
    w.close_chromatograms().unwrap();
    let xml = w.finish().unwrap();
    let text = String::from_utf8_lossy(&xml);
    assert!(text.contains("MS:1003019") && text.contains("MS:1000626"));
    assert!(text.contains("name=\"unità\" value=\"pascal\""));
    assert!(text.contains("name=\"unità\" value=\"degree Celsius\""));
    let run = mzml::read(&xml[..]).unwrap();
    let kinds: Vec<_> = run
        .chromatograms
        .iter()
        .map(|c| (c.id.as_str(), c.kind))
        .collect();
    assert_eq!(
        kinds,
        [
            ("Pump Pressure", ChromKind::Pressure),
            ("Column temperature", ChromKind::Analog)
        ]
    );
    assert_eq!(run.chromatograms[0].intensity, vec![10.0, 20.0, 30.0]);
}

#[test]
fn a_reader_without_profile_refuses_the_profile_route_with_a_key() {
    use mzlab_core::lettori::{self, Limits};
    use mzlab_core::lettori_mzml::{Conversion, Peaks};
    let (idx, peak) = encode(&sample());
    let mut l = lettori::apri(
        FileSet {
            membri: vec![
                conv_aiuto::member("x.D/AcqData/MSScan.bin", &idx),
                conv_aiuto::member("x.D/AcqData/MSPeak.bin", &peak),
            ],
        },
        &Limits::sito(),
    )
    .unwrap();
    assert!(l.scan_profile(0, 0).unwrap().is_none());
    let conv = Conversion {
        peaks: Peaks::Profilo,
        ..Conversion::default()
    };
    let e = conv
        .write(&mut *l, 0, Vec::new(), &mut |_, _| {})
        .unwrap_err();
    assert_eq!(e.key, "err.conv.noProfile");
}
