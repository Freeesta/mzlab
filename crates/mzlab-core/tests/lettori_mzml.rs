//! The single mzML writer on top of `Lettore` gives the file the Agilent converter wrote before it (synthetic folder, no laboratory data).

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

mod agilent_synth;

use agilent_synth::*;
use mzlab_core::agilent::{AgilentD, Files};
use mzlab_core::lettori::{self, FileSet, Limiti, Membro};
use mzlab_core::lettori_mzml;

#[test]
fn agilent_through_lettore_equals_the_direct_converter() {
    let (idx, peak) = encode(&sample());
    let direct = {
        let d = AgilentD::open(
            "x.D",
            &Files {
                ms_scan: &idx,
                ms_peak_len: peak.len() as u64,
                ..Default::default()
            },
        )
        .unwrap();
        let mut out = Vec::new();
        d.to_mzml(&mut peak.clone(), peak.len() as u64, &mut out)
            .unwrap();
        out
    };
    let member = |nome: &str, b: &[u8]| Membro {
        nome: nome.into(),
        dim: b.len() as u64,
        dati: Box::new(b.to_vec()),
    };
    let fs = FileSet {
        membri: vec![
            member("x.D/AcqData/MSScan.bin", &idx),
            member("x.D/AcqData/MSPeak.bin", &peak),
        ],
    };
    let mut l = lettori::apri(fs, &Limiti::sito()).unwrap();
    let mut passi = 0;
    let via = lettori_mzml::scrivi(&mut *l, 0, Vec::new(), &mut |_, _| passi += 1).unwrap();
    assert!(passi >= 2);
    assert_eq!(
        String::from_utf8_lossy(&via),
        String::from_utf8_lossy(&direct)
    );
    assert!(lettori_mzml::scrivi(&mut *l, 1, Vec::new(), &mut |_, _| {}).is_err());
}
