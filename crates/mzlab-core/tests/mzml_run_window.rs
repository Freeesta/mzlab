//! A truncated mzML whose run userParams (mzLab's own `metodo/1`) push `<spectrumList count>` far past the `<run ` tag still tells how many
//! spectra it declares, so the scan table reports the missing ones as for any other cut mzML (H6-3).

use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};

fn written(method: usize, n: usize) -> Vec<u8> {
    let h = Header {
        run: "r".into(),
        run_params: vec![("mzLab metodo/1".into(), "m".repeat(method))],
        ..Default::default()
    };
    let mut w = MzmlWriter::new(Vec::new(), &h).unwrap();
    w.open_spectra(n).unwrap();
    for i in 0..n {
        w.spectrum(&Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 1,
            rt: i as f64 * 0.1,
            polarity: 1,
            mz: vec![100.0, 200.0],
            intensity: vec![1.0, 2.0],
            ..Default::default()
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

/// The file cut right after the end of spectrum `keep`.
fn cut_after(doc: &[u8], keep: usize) -> Vec<u8> {
    let end = b"</spectrum>";
    let mut at = 0;
    for _ in 0..keep {
        at += doc[at..].windows(end.len()).position(|w| w == end).unwrap() + end.len();
    }
    doc[..at].to_vec()
}

#[test]
fn a_cut_file_with_a_long_method_still_declares_its_spectra() {
    for method in [0, 1500, 4000, 200_000] {
        let doc = cut_after(&written(method, 6), 3);
        let mut d = doc.clone();
        let t = mzlab_core::scantable::read(&mut d, doc.len() as u64).unwrap();
        assert_eq!(
            (t.rows.len(), t.declared),
            (3, 6),
            "method text of {method} bytes"
        );
    }
}
