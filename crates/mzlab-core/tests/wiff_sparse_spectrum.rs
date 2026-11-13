//! The spectrum route on the index of sparse product-ion scans of a 3200 QTRAP `.wiff` (`tools/wiff_prodotto_rado.py`: `rado` with a few dense
//! scans, `spaziato` with no two neighbouring bins in any scan, `singolo` with one bin a scan), built as the site builds it (the reader of the pair,
//! `mzidx::build_reader`): the spacing is the bin (0.06) in all three, so no two bins of a scan merge, and a scan of one bin is drawn with a zero on
//! each side. Skipped without python3.

#![cfg(feature = "sciex")]
#![allow(clippy::indexing_slicing)]

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_core::readers::{self, FileSet, Limits, Member};
use mzlab_core::routes::{Query, Routes};
use mzlab_core::{mzidx, upload};
use serde_json::Value;

fn reference(name: &str, flag: Option<&str>) -> Option<(PathBuf, Value)> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_spec_{name}_{}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_prodotto_rado.py");
    let ok = Command::new("python3")
        .arg(script)
        .arg(&out)
        .args(flag)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP wiff_sparse_spectrum: python3 not available");
        return None;
    }
    let json =
        serde_json::from_slice(&std::fs::read(out.join(format!("{name}.json"))).unwrap()).unwrap();
    Some((out, json))
}

/// The routes on the `.mzidx` of sample 0 of the pair `name` in `dir`.
fn routes(dir: &Path, name: &str) -> Routes<Vec<u8>> {
    let member = |file: String| {
        let bytes = std::fs::read(dir.join(&file)).unwrap();
        Member {
            nome: file,
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }
    };
    let set = FileSet {
        membri: vec![
            member(format!("{name}.wiff")),
            member(format!("{name}.wiff.scan")),
        ],
    };
    let mut l = readers::apri(set, &Limits::sito()).unwrap();
    let p = upload::prepare_reader(&mut *l, 0, &[], &format!("{name}.wiff")).unwrap();
    let mut out = Cursor::new(Vec::new());
    mzidx::build_reader(&p.table, &mut *l, 0, &mut out, 64 << 20).unwrap();
    let idx = out.into_inner();
    let len = idx.len() as u64;
    Routes::new(Index::open(idx, len).unwrap())
}

fn nums(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

/// Every scan alone in its window: the line keeps each bin at its m/z (none merged), and a single bin has a zero 0.06 below and above it.
fn spectra_keep_their_bins(name: &str, flag: Option<&str>) {
    let Some((dir, json)) = reference(name, flag) else {
        return;
    };
    let mut r = routes(&dir, name);
    let step = json["step"].as_f64().unwrap();
    assert_eq!(
        r.profile_level(2).unwrap().map(|(_, s)| s),
        Some(step),
        "{name}: spacing"
    );
    for e in json["spectra"].as_array().unwrap() {
        let id = e["id"].as_str().unwrap();
        let cycle: f64 = id
            .split("cycle=")
            .nth(1)
            .and_then(|t| t.split(' ').next())
            .unwrap()
            .parse()
            .unwrap();
        let t = 0.025 * cycle;
        let q = format!(
            "?k=0&rt0={:.4}&rt1={:.4}&level=2&precursor=",
            t - 0.01,
            t + 0.01
        );
        let s = r
            .handle("/api/spectrum", &Query::parse(&q))
            .unwrap()
            .unwrap_or_default();
        let v: Value = serde_json::from_str(&s).unwrap_or_else(|_| panic!("{name} {id}: {s:.200}"));
        let (pmz, py, mz) = (nums(&v["pmz"]), nums(&v["py"]), nums(&e["mz"]));
        let kept: Vec<f64> = pmz
            .iter()
            .zip(&py)
            .filter(|(_, y)| **y > 0.0)
            .map(|(m, _)| *m)
            .collect();
        assert_eq!(kept.len(), mz.len(), "{name} {id}: bins {kept:?} != {mz:?}");
        for (g, w) in kept.iter().zip(&mz) {
            assert!((g - w).abs() < 2e-3, "{name} {id}: {g} != {w}");
        }
        if let [m] = mz[..] {
            let at: Vec<f64> = pmz.iter().map(|p| ((p - m) / step).round()).collect();
            assert_eq!(
                (at, py.first(), py.get(2)),
                (vec![-1.0, 0.0, 1.0], Some(&0.0), Some(&0.0)),
                "{name} {id}: {pmz:?} {py:?}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sparse_scans_with_a_few_dense_ones() {
    spectra_keep_their_bins("rado", None);
}

#[test]
fn scans_with_no_two_neighbouring_bins() {
    spectra_keep_their_bins("spaziato", Some("--spaced"));
}

#[test]
fn scans_of_one_bin_each() {
    spectra_keep_their_bins("singolo", Some("--single"));
}
