//! Parity of the non-finite guard on a synthetic SCIEX pair: the site's engine (`readers::apri`, and `readers_finite::wiff_sample` of its index
//! worker), the readers module and the desktop app / CLI (`mzlab_lettori::apri`) give the same key for the same file: `"ok"` for a finite pair,
//! `err.file.nonFinite` when the start, step or scale of the m/z axis is NaN or infinite. Needs python3 (`tools/wiff_riferimento.py`), skips without.
#![cfg(feature = "sciex")]

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::readers::{self, FileSet, Limits, Member, Reader};

fn member(name: &str, b: Vec<u8>) -> Member {
    Member {
        nome: name.into(),
        dim: b.len() as u64,
        dati: Box::new(b),
        sha1: None,
    }
}

/// The key of the first error met reading the index and every scan of sample 0; `"ok"` when none.
fn key(l: mzlab_core::Result<Box<dyn Reader>>) -> String {
    let mut l = match l {
        Ok(l) => l,
        Err(e) => return e.key.to_string(),
    };
    let n = match l.index(0) {
        Ok(ix) => ix.len(),
        Err(e) => return e.key.to_string(),
    };
    for k in 0..n {
        if let Err(e) = l.scan(0, k) {
            return e.key.to_string();
        }
    }
    "ok".into()
}

/// `one.wiff` and its `.scan` with the first f64 `value` of the scan file replaced by `with` (none replaced when `value` is `None`).
fn pair(dir: &Path, value: Option<f64>, with: f64) -> FileSet {
    let wiff = std::fs::read(dir.join("one.wiff")).unwrap();
    let mut scan = std::fs::read(dir.join("one.wiff.scan")).unwrap();
    if let Some(v) = value {
        let i = scan
            .windows(8)
            .position(|w| w == v.to_le_bytes())
            .expect("the value is in the scan file");
        scan.splice(i..i + 8, with.to_le_bytes());
    }
    FileSet {
        membri: vec![member("one.wiff", wiff), member("one.wiff.scan", scan)],
    }
}

fn pairs() -> Option<PathBuf> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_parity_{}", std::process::id()));
    let ok = Command::new("python3")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_riferimento.py"))
        .arg(&out)
        .output()
        .ok()
        .filter(|o| o.status.success());
    if ok.is_none() {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 not available"
        );
        eprintln!("SKIP engine_finite_parity: python3 not available");
        return None;
    }
    Some(out)
}

#[test]
fn site_and_app_give_the_same_key_for_a_non_finite_wiff() {
    let Some(dir) = pairs() else { return };
    let cases = [None, Some(100.02), Some(0.06), Some(16666.667)];
    for v in cases {
        for with in [f64::NAN, f64::INFINITY] {
            let app = key(mzlab_lettori::apri(pair(&dir, v, with), &Limits::sito()));
            let engine = key(readers::apri(pair(&dir, v, with), &Limits::sito()));
            let worker = key(mzlab_lettori::finite::wiff_sample(
                pair(&dir, v, with),
                &Limits::sito(),
                0,
            ));
            let want = if v.is_some() {
                "err.file.nonFinite"
            } else {
                "ok"
            };
            assert_eq!(
                (app.as_str(), engine.as_str(), worker.as_str()),
                (want, want, want),
                "{v:?} -> {with}"
            );
        }
    }
}
