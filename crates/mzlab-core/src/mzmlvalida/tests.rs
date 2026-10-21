//! The validator on the files the writer and `tools/mzml_msconvert_sintetico.py` produce: valid, and the checks that must not be silent.
use super::*;
use crate::mzml::{ChromKind, Chromatogram, Spectrum};
use crate::mzmlwrite::{Header, MzmlWriter, Opzioni};

pub(super) fn spectrum(i: usize, level: u8, mz: Vec<f64>) -> Spectrum {
    let n = mz.len();
    Spectrum {
        index: i,
        id: format!("scan={}", i + 1),
        level,
        rt: 0.5 + i as f64,
        polarity: 1,
        profile: false,
        mz,
        intensity: (0..n).map(|k| 10.0 + k as f32).collect(),
        filter: "+ c ESI Full ms [100.00-900.00]".into(),
        precursor: (level > 1).then_some(445.12),
        ..Default::default()
    }
}

/// Three spectra (the second one MS2) and two chromatograms, written by `MzmlWriter`; `sorted: false` writes the m/z of the first spectrum backwards.
pub(super) fn written(indexed: bool, opz: Opzioni, sorted: bool) -> Vec<u8> {
    let h = Header {
        run: "x".into(),
        sources: vec!["x.raw".into()],
        components: [
            ("source", "MS:1000073", "electrospray ionization"),
            ("analyzer", "MS:1000081", "quadrupole"),
            ("detector", "MS:1000253", "electron multiplier"),
        ]
        .map(|(k, a, n)| (k.into(), a.into(), n.into()))
        .to_vec(),
        ..Default::default()
    };
    let mut w = if indexed {
        MzmlWriter::indicizzato(Vec::new(), &h, opz)
    } else {
        MzmlWriter::con_opzioni(Vec::new(), &h, opz)
    }
    .unwrap();
    w.open_spectra(3).unwrap();
    let first = if sorted {
        vec![100.5, 200.25, 300.125]
    } else {
        vec![300.125, 200.25, 100.5]
    };
    w.spectrum(&spectrum(0, 1, first)).unwrap();
    w.spectrum(&spectrum(1, 2, vec![110.5, 120.25, 130.0, 140.5]))
        .unwrap();
    w.spectrum(&spectrum(2, 1, vec![105.0, 205.0])).unwrap();
    w.close_spectra().unwrap();
    w.open_chromatograms(2).unwrap();
    let ch = |index: usize, id: &str, kind| Chromatogram {
        index,
        id: id.into(),
        kind,
        q1: None,
        q3: None,
        unit: "number of detector counts".into(),
        time: vec![0.5, 1.5],
        intensity: vec![30.0, 31.0],
    };
    w.chromatogram(&ch(0, "TIC", ChromKind::Tic)).unwrap();
    w.chromatogram(&Chromatogram {
        q1: Some(445.1),
        q3: Some(150.2),
        ..ch(1, "SRM", ChromKind::Srm)
    })
    .unwrap();
    w.close_chromatograms().unwrap();
    w.finish().unwrap()
}

pub(super) fn report(b: &[u8]) -> Report {
    valida(b).unwrap()
}

pub(super) fn errors(r: &Report) -> Vec<&'static str> {
    r.problems
        .iter()
        .filter(|p| p.severity == Severity::Error)
        .map(|p| p.code)
        .collect()
}

pub(super) fn show_all(r: &Report) -> String {
    r.problems
        .iter()
        .map(|p| {
            format!(
                "{} {} {} {}",
                p.severity.as_str(),
                p.path,
                p.code,
                p.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_vocabulary_table_is_loaded() {
    assert!(termini() > 5000);
}

#[test]
fn the_indexed_file_of_the_writer_is_valid() {
    let r = report(&written(true, Opzioni::default(), true));
    assert!(
        r.is_valid() && r.indexed && (r.spectra, r.chromatograms) == (3, 2),
        "{}",
        show_all(&r)
    );
    assert!(r.problems.is_empty(), "{}", show_all(&r));
}

#[test]
fn the_plain_file_and_the_numpress_file_are_valid() {
    for opz in [
        Opzioni::default(),
        Opzioni {
            numpress: true,
            ..Opzioni::default()
        },
    ] {
        let r = report(&written(false, opz, true));
        assert!(r.is_valid() && !r.indexed, "{}", show_all(&r));
        let r = report(&written(true, opz, true));
        assert!(r.is_valid(), "{}", show_all(&r));
    }
}

#[test]
fn the_pass_hands_over_what_the_file_holds() {
    let mut got = Vec::new();
    let r = analizza(&written(true, Opzioni::default(), true)[..], &mut |i| {
        got.push(i)
    })
    .unwrap();
    assert!(r.is_valid());
    let specs: Vec<&SpecInfo> = got
        .iter()
        .filter_map(|i| {
            if let Item::Spectrum(s) = i {
                Some(s)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(specs.len(), 3);
    assert_eq!(
        (specs[1].level, specs[1].polarity, specs[1].rt),
        (2, 1, Some(1.5))
    );
    assert_eq!(specs[1].precursors, vec![(Some(445.12), None)]);
    assert_eq!(specs[1].mz, vec![110.5, 120.25, 130.0, 140.5]);
    assert_eq!(specs[1].intensity, vec![10.0, 11.0, 12.0, 13.0]);
    let srm = got
        .iter()
        .find_map(|i| {
            if let Item::Chromatogram(c) = i {
                Some(c)
            } else {
                None
            }
            .filter(|c| c.id == "SRM")
        })
        .unwrap();
    assert_eq!(
        (srm.kind.as_str(), srm.q1, srm.q3),
        ("MS:1001473", Some(445.1), Some(150.2))
    );
}

#[test]
fn a_file_that_is_not_xml_or_not_mzml_says_so() {
    assert_eq!(errors(&report(b"just text")), ["required"]);
    assert_eq!(errors(&report(b"<html><body/></html>")), ["root"]);
    assert_eq!(errors(&report(b"")), ["required"]);
}

/// The file `tools/mzml_msconvert_sintetico.py` writes (`variant`), or `None` when this machine has no `python3` with numpy.
fn synthetic(variant: &str) -> Option<Vec<u8>> {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/mzml_msconvert_sintetico.py");
    let out = std::env::temp_dir().join(format!(
        "mzlab-valida-{}-{variant}.mzML",
        std::process::id()
    ));
    let ok = std::process::Command::new("python3")
        .arg(script)
        .arg(&out)
        .args(["--scans", "6", "--variant", variant])
        .status()
        .ok()?
        .success();
    let bytes = if ok { std::fs::read(&out).ok() } else { None };
    std::fs::remove_file(&out).ok();
    if bytes.is_none() {
        eprintln!("skipped: no python3 with numpy");
    }
    bytes
}

#[test]
fn the_msconvert_style_synthetic_file_is_valid() {
    let Some(b) = synthetic("base") else { return };
    let r = report(&b);
    assert!(
        r.is_valid() && r.indexed && r.spectra == 6,
        "{}",
        show_all(&r)
    );
}

#[test]
fn the_synthetic_variants_that_are_broken_are_caught() {
    for (variant, code) in [
        ("indice_falso", "index.offset"),
        ("accessione_ignota", "accession"),
    ] {
        let Some(b) = synthetic(variant) else { return };
        assert!(
            errors(&report(&b)).contains(&code),
            "{variant}: {}",
            show_all(&report(&b))
        );
    }
}
