//! An mzML the writer made with a DAD (UV spectra in the same `<spectrumList>`, counted by its `count`) is whole: the index says 0 missing scans. The same
//! file cut before its last scans still says what is lacking. Synthetic data shaped like the reported files (MS2 scans, 5000 optical spectra).

use std::io::Cursor;

use mzlab_core::mzidx::build_mzml;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::upload::prepare;

const SCANS: usize = 40;
const UV: usize = 5000;

fn file() -> Vec<u8> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "dad".into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(SCANS + UV).unwrap();
    for i in 0..SCANS {
        let sp = Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 2,
            rt: i as f64 * 0.01,
            polarity: 1,
            mz: vec![100.0, 200.0],
            intensity: vec![10.0, 20.0],
            ..Default::default()
        };
        w.spectrum(&sp).unwrap();
    }
    for i in 0..UV {
        w.uv_spectrum(i, i as f64 * 0.001, &[210.0, 254.0], &[1.0, 2.0])
            .unwrap();
    }
    w.close_spectra().unwrap();
    w.finish().unwrap()
}

fn missing(mut data: Vec<u8>) -> (usize, usize) {
    let size = data.len() as u64;
    let table = prepare(&mut data, size, "dad.mzML").unwrap().table;
    let mut out = Cursor::new(Vec::new());
    let st = build_mzml(&table, Cursor::new(&data), &mut out, 256 << 20).unwrap();
    (st.scans, st.missing)
}

#[test]
fn optical_spectra_are_not_lacking_scans_but_a_cut_file_still_is() {
    let whole = file();
    assert_eq!(missing(whole.clone()), (SCANS, 0));
    // cut inside the scans: the list still declares them all
    let at = whole
        .windows(19)
        .position(|w| w == b"<spectrum index=\"3\"")
        .unwrap();
    let (scans, lacking) = missing(whole[..at].to_vec());
    assert!(
        scans < SCANS && lacking >= SCANS - scans,
        "{scans} {lacking}"
    );
}
