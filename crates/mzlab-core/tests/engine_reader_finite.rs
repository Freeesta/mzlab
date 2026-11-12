//! The engine's own vendor readers (`readers::apri`, the registry the site's engine opens with) answer through the guard of the readers module and
//! the desktop app (`readers_finite::Finished`): a NaN in the scan header of a synthetic `.wiff` pair (start, step or scale of the m/z axis) is
//! `err.file.nonFinite`, never a number passed on to the index or the charts. A finite pair and a finite `.D` read as before.
//! The pairs need python3 (`tools/wiff_riferimento.py`) and skip when it is missing.

mod agilent_synth;

use std::path::Path;
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

/// `one.wiff` and its `.scan` with the first f64 `value` of the scan file replaced by `with` (no replacement when `value` is `None`).
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

/// The synthetic pairs, or `None` (skip) without python3.
fn pairs() -> Option<std::path::PathBuf> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_finite_{}", std::process::id()));
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
        eprintln!("SKIP engine_reader_finite: python3 not available");
        return None;
    }
    Some(out)
}

/// The start, step and scale of the first block of `one` in `tools/wiff_riferimento.py` (`HEAD`).
const HEADER: [f64; 3] = [100.02, 0.06, 16666.667];

#[test]
fn a_non_finite_wiff_header_is_an_error_of_the_engine() {
    let Some(dir) = pairs() else { return };
    assert_eq!(
        key(readers::apri(pair(&dir, None, 0.0), &Limits::sito())),
        "ok"
    );
    for v in HEADER {
        for with in [f64::NAN, f64::INFINITY] {
            let fs = pair(&dir, Some(v), with);
            assert_eq!(
                key(readers::apri(fs, &Limits::sito())),
                "err.file.nonFinite",
                "{v} -> {with}"
            );
        }
    }
}

#[test]
fn a_finite_d_reads_through_the_guard() {
    let (scan, peak) = agilent_synth::encode(&agilent_synth::sample());
    let fs = FileSet {
        membri: vec![
            member("x.D/AcqData/MSScan.bin", scan),
            member("x.D/AcqData/MSPeak.bin", peak),
        ],
    };
    assert_eq!(key(readers::apri(fs, &Limits::sito())), "ok");
}
