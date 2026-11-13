//! A `.wiff` method with more than sixteen experiments keeps the collision energy of every one of them, in `Wiff::open` and in the mzML.
//! Checked on `tools/wiff_esperimenti.py` (an independent writer). Skipped without python3 (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::readers::{self, FileSet, Limits, Member};
use mzlab_core::wiff::{container, samples, Wiff};
use serde_json::Value;

fn reference() -> Option<(PathBuf, Value)> {
    let out = std::env::temp_dir().join(format!("mzlab_wiff_many_exp_{}", std::process::id()));
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/wiff_esperimenti.py");
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
        eprintln!("SKIP wiff_many_experiments: python3 not available");
        return None;
    }
    let json =
        serde_json::from_slice(&std::fs::read(out.join("esperimenti.json")).unwrap()).unwrap();
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
        membri: vec![member("esperimenti.wiff"), member("esperimenti.wiff.scan")],
    }
}

#[test]
fn experiments_past_the_sixteenth_keep_their_collision_energy() {
    let Some((dir, want)) = reference() else {
        return;
    };
    let ces: Vec<Option<f64>> = want["ce"]
        .as_array()
        .unwrap()
        .iter()
        .map(Value::as_f64)
        .collect();
    let bytes = std::fs::read(dir.join("esperimenti.wiff")).unwrap();
    let ole = container(bytes, "esperimenti").unwrap();
    let scan_len = std::fs::metadata(dir.join("esperimenti.wiff.scan"))
        .unwrap()
        .len();
    let w = Wiff::open(&ole, "esperimenti", Some(scan_len), samples(&ole)[0]).unwrap();
    let got: Vec<Option<f64>> = w.experiments.iter().map(|e| e.ce).collect();
    assert_eq!(got, ces);

    let mut l = readers::apri(set(&dir), &Limits::sito()).unwrap();
    let mzml = String::from_utf8(
        mzlab_core::readers_mzml::scrivi(&mut *l, 0, Vec::new(), &mut |_, _| {}).unwrap(),
    )
    .unwrap();
    for ce in ["21", "32.5", "44"] {
        let term = format!("accession=\"MS:1000045\" name=\"collision energy\" value=\"{ce}\"");
        assert_eq!(mzml.matches(&term).count(), 2, "CE {ce}");
    }
}
