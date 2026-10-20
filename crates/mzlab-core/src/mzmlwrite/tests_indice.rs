//! The indexed form of the writer (`MzmlWriter::indicizzato`): offsets, checksum, plain file unchanged, one processing method per step.
use super::*;
use crate::mzml;
use sha1::{Digest as _, Sha1};

pub(super) fn indexed_sample(steps: Vec<ProcStep>) -> (Vec<u8>, Vec<u8>) {
    let sp = |i: usize| Spectrum {
        index: i,
        id: format!("scan={}", i + 1),
        level: 1,
        rt: 0.5 + i as f64,
        polarity: 1,
        profile: false,
        mz: vec![100.5 + i as f64, 200.25],
        intensity: vec![10.0, 20.0 + i as f32],
        ..Default::default()
    };
    let ch = Chromatogram {
        index: 0,
        id: "TIC".into(),
        kind: ChromKind::Tic,
        q1: None,
        q3: None,
        unit: "number of detector counts".into(),
        time: vec![0.5, 1.5],
        intensity: vec![30.0, 31.0],
    };
    let h = Header {
        run: "x".into(),
        steps,
        ..Default::default()
    };
    let write = |idx: bool| {
        let mut w = if idx {
            MzmlWriter::indicizzato(Vec::new(), &h, Opzioni::default())
        } else {
            MzmlWriter::new(Vec::new(), &h)
        }
        .unwrap();
        w.open_spectra(2).unwrap();
        (0..2).for_each(|i| w.spectrum(&sp(i)).unwrap());
        w.close_spectra().unwrap();
        w.open_chromatograms(1).unwrap();
        w.chromatogram(&ch).unwrap();
        w.close_chromatograms().unwrap();
        w.finish().unwrap()
    };
    (write(false), write(true))
}

/// (idRef, offset) of the `<index name="NAME">` list.
fn offsets(t: &str, name: &str) -> Vec<(String, u64)> {
    let a = t.find(&format!("<index name=\"{name}\">")).unwrap();
    let body = &t[a..a + t[a..].find("</index>").unwrap()];
    body.split("<offset idRef=\"")
        .skip(1)
        .map(|o| {
            (
                o[..o.find('"').unwrap()].to_string(),
                o[o.find('>').unwrap() + 1..o.find("</offset>").unwrap()]
                    .parse()
                    .unwrap(),
            )
        })
        .collect()
}

#[test]
fn indexed_offsets_checksum_and_read_back() {
    let (plain, idx) = indexed_sample(Vec::new());
    let t = String::from_utf8(idx.clone()).unwrap();
    assert!(
        t.starts_with("<?xml")
            && t.contains("<indexedmzML ")
            && t.trim_end().ends_with("</indexedmzML>")
    );
    assert!(t.contains("version=\"4.2.6\"") && !t.contains("4.1.163"));
    let sp = offsets(&t, "spectrum");
    assert_eq!(
        sp.iter().map(|s| s.0.as_str()).collect::<Vec<_>>(),
        ["scan=1", "scan=2"]
    );
    for (k, (id, off)) in sp.iter().enumerate() {
        assert!(idx[*off as usize..]
            .starts_with(format!("<spectrum index=\"{k}\" id=\"{id}\"").as_bytes()));
    }
    let ch = offsets(&t, "chromatogram");
    assert!(idx[ch[0].1 as usize..].starts_with(b"<chromatogram index=\"0\" id=\"TIC\""));
    let list: usize = t
        .split("<indexListOffset>")
        .nth(1)
        .unwrap()
        .split('<')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(idx[list..].starts_with(b"<indexList count=\"2\">"));
    // the checksum covers every byte up to and including the opening <fileChecksum> tag
    let tag = t.find("<fileChecksum>").unwrap() + "<fileChecksum>".len();
    let hex: String = Sha1::digest(&idx[..tag])
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(&t[tag..tag + 40], hex);
    // the same spectra and chromatograms as the plain file, which keeps the old CV and no index
    let p = String::from_utf8(plain.clone()).unwrap();
    assert!(!p.contains("indexedmzML") && p.contains("version=\"4.1.163\""));
    let body = |s: &str| s[s.find("<spectrumList").unwrap()..s.find("</run>").unwrap()].to_string();
    assert_eq!(body(&p), body(&t));
    let (a, b) = (
        mzml::read(&plain[..]).unwrap(),
        mzml::read(&idx[..]).unwrap(),
    );
    assert_eq!(
        (a.spectra.len(), a.chromatograms.len()),
        (b.spectra.len(), b.chromatograms.len())
    );
    assert!(a
        .spectra
        .iter()
        .zip(&b.spectra)
        .all(|(x, y)| x.id == y.id && x.mz == y.mz && x.intensity == y.intensity));
}

#[test]
fn one_processing_method_per_step() {
    let step = |n: &str, acc: &str, name: &str| ProcStep {
        accession: acc.into(),
        name: name.into(),
        software: n.into(),
        version: "1".into(),
        params: vec![("min_height".into(), "1000".into())],
    };
    let (plain, idx) = indexed_sample(vec![
        step("features.detect@1", "MS:1000035", "peak picking"),
        step("table.filter@1", "MS:1000543", "data processing action"),
    ]);
    for b in [&plain, &idx] {
        let t = String::from_utf8_lossy(b);
        assert_eq!(t.matches("<processingMethod ").count(), 3);
        assert!(
            t.contains("<softwareList count=\"3\">")
                && t.contains("order=\"2\" softwareRef=\"step2\"")
        );
        assert!(t.contains("<userParam name=\"min_height\" value=\"1000\"/>"));
    }
    assert!(mzml::read(&idx[..]).is_ok());
}

#[test]
fn an_empty_run_still_has_a_valid_index() {
    let mut w =
        MzmlWriter::indicizzato(Vec::new(), &Header::default(), Opzioni::default()).unwrap();
    w.open_spectra(0).unwrap();
    w.close_spectra().unwrap();
    let t = String::from_utf8(w.finish().unwrap()).unwrap();
    assert!(t.contains("<indexList count=\"1\"><index name=\"spectrum\"/></indexList>"));
}
