//! The engine refuses every MS-Numpress array with `err.file.numpress`, the zlib variants (MS:1002746/47/48) too: read as plain floats they gave
//! garbage or an infinity (`err.file.nonFinite`). Spectra (`mzml::read`) and chromatograms (`mzmlhead`) are the same Rust on site and app.

#![allow(clippy::unwrap_used)]

use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::mzmlwrite::{Header, MzmlWriter, Options};
use mzlab_core::numpress::ACCESSIONS;

// two points of 64-bit floats, the bytes of a valid plain array: only the numpress term makes them wrong
const T: &str = "AAAAAAAAAAAAAAAAAADwPw==";
const Y: &str = "AAAAAAAAFEAAAAAAAAAYQA==";

fn array(acc: &str, kind: &str, data: &str) -> String {
    format!(
        r#"<binaryDataArray encodedLength="0"><cvParam cvRef="MS" accession="{acc}" name="numpress" value=""/><cvParam cvRef="MS" accession="{kind}" name="a" value=""/><binary>{data}</binary></binaryDataArray>"#
    )
}

fn spectrum_file(acc: &str) -> String {
    let arrays = [array(acc, "MS:1000514", T), array(acc, "MS:1000515", Y)].join("");
    format!(
        r#"<mzML><run id="r"><spectrumList count="1"><spectrum index="0" id="scan=1" defaultArrayLength="2"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/><binaryDataArrayList count="2">{arrays}</binaryDataArrayList></spectrum></spectrumList></run></mzML>"#
    )
}

fn chromatogram_file(acc: &str) -> String {
    let arrays = [array(acc, "MS:1000595", T), array(acc, "MS:1000515", Y)].join("");
    format!(
        r#"<mzML><run id="r"><chromatogramList count="1"><chromatogram index="0" id="TIC" defaultArrayLength="2"><binaryDataArrayList count="2">{arrays}</binaryDataArrayList></chromatogram></chromatogramList></run></mzML>"#
    )
}

#[test]
fn every_numpress_accession_is_refused_in_spectra_and_chromatograms() {
    for acc in ACCESSIONS {
        let e = mzlab_core::mzml::read(spectrum_file(acc).as_bytes()).unwrap_err();
        assert_eq!(e.key, "err.file.numpress", "spectrum {acc}");
        let e = mzlab_core::mzml::read(chromatogram_file(acc).as_bytes()).unwrap_err();
        assert_eq!(e.key, "err.file.numpress", "chromatogram {acc}");
        let b = chromatogram_file(acc).into_bytes();
        let size = b.len() as u64;
        let mut m = Mzml::new(b, size);
        let c = m.chromatograms(0).unwrap();
        assert_eq!(c.len(), 1, "{acc}");
        assert_eq!(
            m.chromatogram(&c[0]).unwrap_err().key,
            "err.file.numpress",
            "chromatogram by index {acc}"
        );
    }
}

#[test]
fn a_numpress_and_zlib_file_of_the_writer_is_refused_not_read_as_floats() {
    // 5000 points: zlib makes the numpress arrays smaller, so the writer tags them MS:1002746 / MS:1002748
    let o = Options {
        numpress: true,
        ..Options::default()
    };
    let mut w = MzmlWriter::with_options(Vec::new(), &Header::default(), o).unwrap();
    w.open_spectra(2).unwrap();
    for sc in 0..2 {
        w.spectrum(&Spectrum {
            index: sc,
            id: format!("scan={}", sc + 1),
            level: 1,
            rt: sc as f64 * 0.01,
            polarity: 1,
            mz: (0..5000).map(|i| 100.0 + i as f64 * 0.18).collect(),
            intensity: (0..5000).map(|i| 100.0 + ((i * 17) % 977) as f32).collect(),
            ..Default::default()
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let b = w.finish().unwrap();
    assert!(String::from_utf8_lossy(&b).contains("MS:1002746"));
    assert_eq!(
        mzlab_core::mzml::read(&b[..]).unwrap_err().key,
        "err.file.numpress"
    );
}
