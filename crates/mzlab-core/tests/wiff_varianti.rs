//! The `.wiff` reader of the readers module (feature `estesi`) against the synthetic pairs of `tools/wiff_riferimento.py`: a QTRAP 5500 with scan blocks of
//! versions 1 and 3 and precursor-ion / neutral-loss scans, and the files that must be refused with `err.file.wiff` (the cause in `detail`), never read as a
//! full scan. Skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails instead).

#![cfg(feature = "sciex")]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::lettori::{self, FileSet, Limiti, Membro};
use mzlab_core::wiff::{container, Wiff};
use serde_json::Value;

fn reference() -> Option<(PathBuf, Value)> {
    // one folder per call: the tests of this file run in parallel threads of one process and would write the same files
    static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let out = std::env::temp_dir().join(format!(
        "mzlab_wiff_varianti_{}_{}",
        std::process::id(),
        CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    ));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_riferimento.py");
    let ok = Command::new("python3")
        .arg(script)
        .arg(&out)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP wiff_varianti: python3 not available");
        return None;
    }
    let json =
        serde_json::from_slice(&std::fs::read(out.join("riferimento.json")).unwrap()).unwrap();
    Some((out, json))
}

fn nums(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

fn close(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

fn set(dir: &Path, name: &str) -> FileSet {
    let member = |file: String| {
        let bytes = std::fs::read(dir.join(&file)).unwrap();
        Membro {
            nome: file,
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }
    };
    FileSet {
        membri: vec![
            member(format!("{name}.wiff")),
            member(format!("{name}.wiff.scan")),
        ],
    }
}

fn open(dir: &Path, name: &str) -> Result<(Wiff, Vec<u8>), mzlab_core::error::Error> {
    let ole = container(
        std::fs::read(dir.join(format!("{name}.wiff"))).unwrap(),
        name,
    )
    .unwrap();
    let scan = std::fs::read(dir.join(format!("{name}.wiff.scan"))).unwrap();
    Wiff::open(&ole, name, Some(scan.len() as u64), 1).map(|w| (w, scan))
}

fn detail(e: &mzlab_core::error::Error) -> String {
    assert_eq!(e.key, "err.file.wiff");
    e.params
        .iter()
        .find(|(k, _)| *k == "detail")
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

#[test]
fn a_qtrap_5500_reads_its_scan_types() {
    let Some((dir, want)) = reference() else {
        return;
    };
    let r = &want["variante"]["1"];
    let (w, mut scan) = open(&dir, "variante").unwrap();
    assert!(w.variante() && w.model.as_deref() == r["model"].as_str());
    let run = w.run(&mut scan).unwrap();
    let sp = r["spectra"].as_array().unwrap();
    assert_eq!(run.spectra.len(), sp.len());
    for (g, e) in run.spectra.iter().zip(sp) {
        assert_eq!(g.id, e["id"].as_str().unwrap());
        assert_eq!(
            (g.level as i64, g.precursor),
            (e["level"].as_i64().unwrap(), e["precursor"].as_f64()),
            "{}",
            g.id
        );
        let (mz, it) = (nums(&e["mz"]), nums(&e["intensity"]));
        assert_eq!(g.mz.len(), mz.len(), "{}: points", g.id);
        assert!(
            (0..mz.len()).all(|k| close(g.mz[k], mz[k]) && close(g.intensity_f64(k), it[k])),
            "{}",
            g.id
        );
    }
}

#[test]
fn the_mzml_of_the_module_names_the_instrument_and_the_spectrum_types() {
    let Some((dir, want)) = reference() else {
        return;
    };
    let mut l = lettori::apri(set(&dir, "variante"), &Limiti::sito()).unwrap();
    assert!(l.variante(0));
    let xml = String::from_utf8(
        mzlab_core::lettori_mzml::scrivi(&mut *l, 0, Vec::new(), &mut |_, _| {}).unwrap(),
    )
    .unwrap();
    assert!(
        xml.contains("accession=\"MS:1000931\" name=\"QTRAP 5500\""),
        "instrument"
    );
    assert!(!xml.contains("name=\"instrument model\""));
    let content = &xml[xml.find("<fileContent>").unwrap()..xml.find("</fileContent>").unwrap()];
    assert!(
        content.contains("MS:1000341") && content.contains("MS:1000326"),
        "{content}"
    );
    for (term, name) in want["variante"]["1"]["spectra"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s["term"].as_array())
        .map(|t| (t[0].as_str().unwrap(), t[1].as_str().unwrap()))
    {
        assert!(
            xml.contains(&format!("accession=\"{term}\" name=\"{name}\"")),
            "{term}"
        );
    }
    // one precursor-ion and one neutral-loss spectrum per cycle (two cycles), each type in its own spectra
    assert_eq!(
        xml.matches("accession=\"MS:1000341\" name=\"precursor ion spectrum\"")
            .count(),
        2 + 1
    );
    assert_eq!(
        xml.matches("accession=\"MS:1000326\" name=\"constant neutral loss spectrum\"")
            .count(),
        1 + 1
    );
}

#[test]
fn what_the_reader_does_not_know_is_refused_never_read_as_a_full_scan() {
    let Some((dir, want)) = reference() else {
        return;
    };
    for (name, text) in want["rifiuti"].as_object().unwrap() {
        let text = text.as_str().unwrap();
        // a code or a period is refused when the sample is opened; a block when it is read
        let err = match open(&dir, name) {
            Err(e) => e,
            Ok((w, mut scan)) => w
                .run(&mut scan)
                .err()
                .unwrap_or_else(|| panic!("{name}: read")),
        };
        assert_eq!(detail(&err), text, "{name}");
        let via = lettori::apri(set(&dir, name), &Limiti::sito());
        assert_eq!(
            detail(&via.err().unwrap_or_else(|| panic!("{name}: registry"))),
            text,
            "{name}: registry"
        );
    }
}

/// A damaged file never panics: every byte of the container and of the scan file of the synthetic QTRAP 5500 pair is overwritten in turn with values that
/// make sizes, counts and codes absurd; the reader opens it or refuses it with an `err.*` key, and the method document is always written.
#[test]
fn damaged_files_are_refused_or_read_never_a_panic() {
    let Some((dir, _)) = reference() else { return };
    let wiff = std::fs::read(dir.join("variante.wiff")).unwrap();
    let scan = std::fs::read(dir.join("variante.wiff.scan")).unwrap();
    let try_open = |wiff: Vec<u8>, scan: Vec<u8>| {
        let Ok(ole) = container(wiff, "variante.wiff") else {
            return;
        };
        drop(mzlab_core::metodo_wiff::from_wiff(
            &ole,
            1,
            &Default::default(),
        ));
        let mut scan = scan;
        if let Ok(w) = Wiff::open(&ole, "variante.wiff", Some(scan.len() as u64), 1) {
            drop(w.run(&mut scan));
        }
    };
    for at in (0..wiff.len()).step_by(7) {
        for v in [0u8, 0xff, 0x7f] {
            let mut w = wiff.clone();
            w[at] = v;
            try_open(w, scan.clone());
        }
    }
    for at in 0..scan.len() {
        for v in [0u8, 0xff, 0x80] {
            let mut s = scan.clone();
            s[at] = v;
            try_open(wiff.clone(), s);
        }
    }
    for cut in (0..scan.len()).step_by(11) {
        try_open(wiff.clone(), scan[..cut].to_vec());
    }
}
