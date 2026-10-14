//! `upload::prepare` gives what `mzidx.builder_inputs` gives (`tools/upload_riferimento.py`): the same scan arrays and rows, the same
//! `source` (name, size, probe, sha256; the modification date is left out) and `run` objects. Skipped without python3 and numpy
//! (`MZLAB_GOLDEN_STRICT=1` fails instead).

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::upload::prepare;
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn same(path: &str, got: &Value, want: &Value) {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            assert!(
                a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0),
                "{path}: {a} against {b}"
            );
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{path}: length");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                same(&format!("{path}[{i}]"), x, y);
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(
                a.keys().collect::<Vec<_>>(),
                b.keys().collect::<Vec<_>>(),
                "{path}: keys"
            );
            for (k, y) in b {
                same(&format!("{path}.{k}"), &a[k], y);
            }
        }
        (a, b) => assert_eq!(a, b, "{path}"),
    }
}

#[test]
fn upload_like_python() {
    let out = std::env::temp_dir().join(format!("mzlab-upl-{}", std::process::id()));
    let mut cmd = Command::new("python3");
    cmd.current_dir(root())
        .arg("tools/upload_riferimento.py")
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
                panic!("the reference did not run: {other:?}");
            }
            eprintln!("python3/numpy not available: test skipped");
            return;
        }
    };
    let mut files = 0;
    for line in listing.lines().filter(|l| !l.is_empty()) {
        let stem = PathBuf::from(line)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let want: Value =
            serde_json::from_slice(&std::fs::read(out.join(format!("{stem}.upl.json"))).unwrap())
                .unwrap();
        let size = std::fs::metadata(line).unwrap().len();
        let name = PathBuf::from(line)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let mut f = std::fs::File::open(line).unwrap();
        let p = prepare(&mut f, size, &name).unwrap_or_else(|e| panic!("{stem}: {e}"));
        let t = &p.table;
        let nums = |k: &str| {
            want[k]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            t.level.iter().map(|v| *v as f64).collect::<Vec<_>>(),
            nums("levels"),
            "{stem}: levels"
        );
        assert_eq!(
            t.polarity.iter().map(|v| *v as f64).collect::<Vec<_>>(),
            nums("polarity"),
            "{stem}: polarity"
        );
        assert_eq!(t.rt, nums("rt"), "{stem}: rt");
        assert_eq!(t.tic, nums("tic"), "{stem}: tic");
        let got_rows: Value = serde_json::from_slice(&t.scans_json).unwrap();
        let want_rows: Value = serde_json::from_str(want["scans"].as_str().unwrap()).unwrap();
        same(&format!("{stem} rows"), &got_rows, &want_rows);
        let src: Value = serde_json::from_str(&t.source_json).unwrap();
        for k in ["name", "size", "probe", "sha256"] {
            assert_eq!(src[k], want["source"][k], "{stem}: source.{k}");
        }
        assert_eq!(
            serde_json::from_str::<Value>(&t.run_json).unwrap(),
            want["run"],
            "{stem}: run"
        );
        assert_eq!(t.hr1, want["hr1"].as_bool().unwrap(), "{stem}: hr1");
        files += 1;
    }
    assert!(files >= 8, "{files} files");
    let _ = std::fs::remove_dir_all(&out);
}
