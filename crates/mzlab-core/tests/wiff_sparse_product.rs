//! Sparse product-ion scans of a 3200 QTRAP `.wiff` (`rado` of `tools/wiff_prodotto_rado.py`, an independent writer: scan code 6, 0 to 5 points a scan,
//! streams padded to 4 bytes, an index value at +26 that is not the point count) are read whole: every scan with its points through the reader the site
//! uses (index, scans, losses, TIC) and through `Wiff::run` (the app and the CLI), none judged damaged, the sample never refused. Skipped without python3.

#![cfg(feature = "sciex")]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::readers::{self, FileSet, Limits, Member};
use mzlab_core::wiff::{container, Wiff};
use serde_json::Value;

fn reference() -> Option<(PathBuf, Value)> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_rado_{}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_prodotto_rado.py");
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
        eprintln!("SKIP wiff_sparse_product: python3 not available");
        return None;
    }
    let json = serde_json::from_slice(&std::fs::read(out.join("rado.json")).unwrap()).unwrap();
    Some((out, json))
}

fn set(dir: &Path) -> FileSet {
    let member = |file: &str| {
        let bytes = std::fs::read(dir.join(file)).unwrap();
        Member {
            nome: file.into(),
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }
    };
    FileSet {
        membri: vec![member("rado.wiff"), member("rado.wiff.scan")],
    }
}

/// `got` as `want` within `rel` (the intensities of a spectrum are f32).
fn same(got: &[f64], want: &Value, rel: f64, what: &str) {
    let want: Vec<f64> = want
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert_eq!(got.len(), want.len(), "{what}: points");
    for (g, w) in got.iter().zip(&want) {
        assert!(
            (g - w).abs() <= rel * w.abs().max(1.0),
            "{what}: {g} != {w}"
        );
    }
}

#[test]
fn sparse_product_ion_scans_are_read_whole() {
    let Some((dir, want)) = reference() else {
        return;
    };
    let want = want["spectra"].as_array().unwrap();
    let ole = container(std::fs::read(dir.join("rado.wiff")).unwrap(), "rado.wiff").unwrap();
    let mut scan = std::fs::read(dir.join("rado.wiff.scan")).unwrap();
    let run = Wiff::open(&ole, "rado.wiff", Some(scan.len() as u64), 1)
        .unwrap()
        .run(&mut scan)
        .unwrap();
    let mut l = readers::apri(set(&dir), &Limits::sito()).unwrap();
    let n = l.index(0).unwrap().len();
    let lost = l.losses(0).unwrap();
    assert!(!lost.partial(), "no block is damaged ({lost:?})");
    assert_eq!(
        (run.spectra.len(), n),
        (want.len(), want.len()),
        "spectra (app, site)"
    );
    for (k, (g, e)) in run.spectra.iter().zip(want).enumerate() {
        let s = l.scan(0, k).unwrap();
        for (x, how) in [(g, "app"), (&s, "site")] {
            let id = e["id"].as_str().unwrap();
            assert_eq!(x.id, id, "{how}");
            same(&x.mz, &e["mz"], 1e-9, &format!("{how} {id} m/z"));
            let inten: Vec<f64> = x.intensity.iter().map(|&v| f64::from(v)).collect();
            same(
                &inten,
                &e["intensity"],
                1e-6,
                &format!("{how} {id} intensity"),
            );
        }
    }
    // the site draws the TIC of a product-ion experiment from the index: one value a scan, the one its index row gives
    let tic: Vec<f64> = l.index(0).unwrap().iter().map(|e| e.tic).collect();
    assert_eq!(tic.len(), want.len(), "a TIC value per scan");
    assert!(
        tic.iter().any(|&v| v != tic[0]),
        "the TIC is not flat: {tic:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
