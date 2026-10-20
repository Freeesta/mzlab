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
        sha1: None,
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

/// The `convert.mzml` form: same spectra as `scrivi`, `indexedmzML` with working offsets, CV 4.2.6, one `processingMethod` per step.
#[test]
fn indexed_form_has_the_same_scans_and_a_working_index() {
    use mzlab_core::mzmlwrite::{Opzioni, ProcStep};
    let (idx, peak) = encode(&sample());
    let member = |nome: &str, b: &[u8]| Membro {
        nome: nome.into(),
        dim: b.len() as u64,
        dati: Box::new(b.to_vec()),
        sha1: None,
    };
    let fs = FileSet {
        membri: vec![
            member("x.D/AcqData/MSScan.bin", &idx),
            member("x.D/AcqData/MSPeak.bin", &peak),
        ],
    };
    let mut l = lettori::apri(fs, &Limiti::sito()).unwrap();
    let plain = lettori_mzml::scrivi(&mut *l, 0, Vec::new(), &mut |_, _| {}).unwrap();
    let step = ProcStep {
        accession: "MS:1000035".into(),
        name: "peak picking".into(),
        software: "features.detect@1".into(),
        version: "1".into(),
        params: vec![("min_height".into(), "1000".into())],
    };
    let run = |l: &mut dyn lettori::Lettore| {
        lettori_mzml::scrivi_indicizzato(
            l,
            0,
            Vec::new(),
            &mut |_, _| {},
            Opzioni::default(),
            std::slice::from_ref(&step),
        )
        .unwrap()
    };
    let (a, b) = (run(&mut *l), run(&mut *l));
    assert_eq!(a, b, "two runs, same bytes");
    let (p, t) = (
        String::from_utf8_lossy(&plain).into_owned(),
        String::from_utf8_lossy(&a).into_owned(),
    );
    assert!(t.contains("<indexedmzML ") && t.contains("version=\"4.2.6\""));
    assert_eq!(
        t.matches("<processingMethod ").count(),
        p.matches("<processingMethod ").count() + 1
    );
    let body = |s: &str| s[s.find("<spectrumList").unwrap()..s.find("</run>").unwrap()].to_string();
    assert_eq!(body(&p), body(&t));
    let n = t.matches("<spectrum index=").count();
    assert!(n > 0 && t.matches("<offset idRef=").count() >= n);
    for o in t.split("<offset idRef=\"").skip(1) {
        let off: usize = o[o.find('>').unwrap() + 1..o.find("</offset>").unwrap()]
            .parse()
            .unwrap();
        assert!(a[off..].starts_with(b"<spectrum ") || a[off..].starts_with(b"<chromatogram "));
    }
    let back = mzlab_core::mzml::read(&a[..]).unwrap();
    assert_eq!(back.spectra.len(), l.indice(0).unwrap().len());
}
