//! Sparse product-ion scans of a 3200 QTRAP `.wiff` (`rado` of `tools/wiff_prodotto_rado.py`, an independent writer: scan code 6, 0 to 5 points a scan,
//! streams padded to 4 bytes, an index value at +26 that is not the point count) are read whole: every scan with its points through the reader the site
//! uses (index, scans, losses, TIC) and through `Wiff::run` (the app and the CLI), none judged damaged, the sample never refused. `storto` (`--odd`) holds
//! a few blocks their bytes contradict: those are left out and counted, the others read whole, the sample still never refused. Skipped without python3.

#![cfg(feature = "sciex")]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::readers::{self, FileSet, Limits, Member};
use mzlab_core::wiff::{container, Wiff};
use serde_json::Value;

fn reference(name: &str) -> Option<(PathBuf, Value)> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_{name}_{}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_prodotto_rado.py");
    let ok = Command::new("python3")
        .arg(script)
        .arg(&out)
        .args((name == "storto").then_some("--odd"))
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
    let json =
        serde_json::from_slice(&std::fs::read(out.join(format!("{name}.json"))).unwrap()).unwrap();
    Some((out, json))
}

fn set(dir: &Path, name: &str) -> FileSet {
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
        membri: vec![
            member(&format!("{name}.wiff")),
            member(&format!("{name}.wiff.scan")),
        ],
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

/// Every spectrum of `name` as its JSON lists it, through `Wiff::run` and the site's reader; returns the reader's losses and index TIC.
fn read_as_listed(name: &str) -> Option<(readers::Losses, Value, Vec<f64>)> {
    let (dir, json) = reference(name)?;
    let want = json["spectra"].as_array().unwrap();
    let wiff = format!("{name}.wiff");
    let ole = container(std::fs::read(dir.join(&wiff)).unwrap(), &wiff).unwrap();
    let mut scan = std::fs::read(dir.join(format!("{wiff}.scan"))).unwrap();
    let run = Wiff::open(&ole, &wiff, Some(scan.len() as u64), 1)
        .unwrap()
        .run(&mut scan)
        .unwrap();
    let mut l = readers::apri(set(&dir, name), &Limits::sito()).unwrap();
    let n = l.index(0).unwrap().len();
    assert_eq!(
        (run.spectra.len(), n),
        (want.len(), want.len()),
        "{name}: spectra (app, site)"
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
    let lost = l.losses(0).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    Some((lost, json, tic))
}

#[test]
fn sparse_product_ion_scans_are_read_whole() {
    let Some((lost, json, tic)) = read_as_listed("rado") else {
        return;
    };
    assert!(!lost.partial(), "no block is damaged ({lost:?})");
    assert_eq!(
        tic.len(),
        json["spectra"].as_array().unwrap().len(),
        "a TIC value per scan"
    );
    assert!(
        tic.iter().any(|&v| v != tic[0]),
        "the TIC is not flat: {tic:?}"
    );
}

/// Blocks with 5 to 8 bytes after their points and a head that declares more points than its stream holds are left out and counted
/// (`warn.wiff.damaged`, «x of y read»): the sample opens with every other scan, never `err.file.wiff`.
#[test]
fn blocks_their_bytes_contradict_are_counted_not_a_refusal() {
    let Some((lost, json, _)) = read_as_listed("storto") else {
        return;
    };
    let bad = json["damaged"].as_u64().unwrap() as usize;
    assert!(bad >= 2, "the fixture holds odd blocks");
    assert_eq!(
        (lost.cause, lost.lost()),
        (readers::DAMAGED, bad),
        "{lost:?}"
    );
}
