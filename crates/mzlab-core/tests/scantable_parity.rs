//! The scan table read by Rust from the headers of an mzML is the one the Python reader wrote in its `.mzidx` (`tools/mzidx_riferimento.py`):
//! every row equal (numbers exactly), and the `run` object (`nce`, `hr1`, `n_chromatograms`). Skipped without python3 and numpy
//! (`MZLAB_GOLDEN_STRICT=1` fails instead).

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::scantable;
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn u64_at(b: &[u8], at: usize) -> usize {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap()) as usize
}

#[test]
fn rust_scan_table_is_the_python_one() {
    let out = std::env::temp_dir().join(format!("mzlab-st-{}", std::process::id()));
    let mut cmd = Command::new("python3");
    cmd.current_dir(root())
        .arg("tools/mzidx_riferimento.py")
        .arg(&out);
    if let Ok(rd) = std::fs::read_dir(root().join("mzlab/web/esempi")) {
        let mut v: Vec<_> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "mzML"))
            .collect();
        v.sort();
        cmd.args(v);
    }
    let listing = match cmd.output() {
        Ok(o) if o.status.success() => String::from_utf8(o.stdout).unwrap(),
        other => {
            if std::env::var("MZLAB_GOLDEN_STRICT").is_ok() {
                panic!("the reference builder did not run: {other:?}");
            }
            eprintln!("python3/numpy not available: test skipped");
            return;
        }
    };
    let mut n = 0;
    for line in listing.lines().filter(|l| !l.is_empty()) {
        let mzml = PathBuf::from(line);
        let stem = mzml.file_stem().unwrap().to_string_lossy().to_string();
        let idx = std::fs::read(out.join(format!("{stem}.mzidx"))).unwrap();
        let (mo, ml, so, sl) = (
            u64_at(&idx, 16),
            u64_at(&idx, 24),
            u64_at(&idx, 32),
            u64_at(&idx, 40),
        );
        let meta: Value = serde_json::from_slice(&idx[mo..mo + ml]).unwrap();
        let want: Value = serde_json::from_slice(&idx[so..so + sl]).unwrap();
        let mut f = std::fs::File::open(&mzml).unwrap();
        let size = f.metadata().unwrap().len();
        let t = scantable::read(&mut f, size).unwrap_or_else(|e| panic!("{stem}: {e}"));
        let got: Value = serde_json::from_str(&t.rows_json()).unwrap();
        let (g, w) = (got.as_array().unwrap(), want.as_array().unwrap());
        assert_eq!(g.len(), w.len(), "{stem}: scans");
        for (i, (a, b)) in g.iter().zip(w).enumerate() {
            assert_eq!(a, b, "{stem}: row {i}");
        }
        assert_eq!(meta["run"]["nce"].as_bool(), Some(t.nce), "{stem}: nce");
        assert_eq!(meta["run"]["hr1"].as_bool(), Some(t.hr1), "{stem}: hr1");
        assert_eq!(
            meta["run"]["n_chromatograms"].as_u64(),
            Some(t.n_chromatograms as u64),
            "{stem}: chromatograms"
        );
        n += 1;
    }
    assert!(n >= 8);
    let _ = std::fs::remove_dir_all(&out);
}
