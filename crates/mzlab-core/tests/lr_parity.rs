//! `lr::file_entry` and `lr::open_items` answer like `App.files()` and `Session(...).info()` (`tools/sessione_riferimento.py`) on the synthetic
//! files and the examples, with the name guessed and with the sample table edited. Skipped without python3 and numpy
//! (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_core::lr::{file_entry, open_items, Sample};
use mzlab_core::mzmlhead::Mzml;
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn close(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}
fn same(path: &str, got: &Value, want: &Value) {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => assert!(
            close(a.as_f64().unwrap(), b.as_f64().unwrap()),
            "{path}: {a} against {b}"
        ),
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

/// Documented exception to the parity: the recognition of an infusion (`infusione.rs`) is Rust only. `explore.py` keeps the rule of
/// the constant RT and has neither `msn` nor the hint "infusion": the new fields are checked against themselves, then the values of
/// Python are put back.
fn infusion_exception(got: &mut Value, want: &Value) {
    let Some(o) = got.as_object_mut() else {
        return;
    };
    if let Some(m) = o.remove("msn") {
        if let Some(inf) = o.get("infusion").and_then(Value::as_bool) {
            assert_eq!(inf, !m.is_null(), "infusion and msn disagree");
            if m["motivo"] != "rt_costante" {
                o.insert("infusion".into(), want["infusion"].clone());
            }
        }
    }
    if o.get("hint").and_then(Value::as_str) == Some("infusion") {
        o.insert("hint".into(), want["hint"].clone());
    }
}

#[test]
fn session_like_python() {
    let out = std::env::temp_dir().join(format!("mzlab-lr-{}", std::process::id()));
    let mut cmd = Command::new("python3");
    cmd.current_dir(root())
        .arg("tools/sessione_riferimento.py")
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
    let (mut files, mut items) = (0, 0);
    for line in listing.lines().filter(|l| !l.is_empty()) {
        let stem = PathBuf::from(line)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let name = PathBuf::from(line)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let size = std::fs::metadata(line).unwrap().len();
        let idx_path = out.join(format!("{stem}.mzidx"));
        let isize = std::fs::metadata(&idx_path).unwrap().len();
        let mut idx = Index::open(std::fs::File::open(&idx_path).unwrap(), isize).unwrap();
        let mut m = Mzml::new(std::fs::File::open(line).unwrap(), size);
        let want: Value =
            serde_json::from_slice(&std::fs::read(out.join(format!("{stem}.lr.json"))).unwrap())
                .unwrap();
        let got =
            file_entry(&mut idx, &mut m, &name, size).unwrap_or_else(|e| panic!("{stem}: {e}"));
        let mut got: Value = serde_json::from_str(&got).unwrap();
        infusion_exception(&mut got, &want["entry"]);
        same(&format!("{stem} entry"), &got, &want["entry"]);
        let edit = Sample {
            label: Some("my sample".into()),
            typ: Some("standard".into()),
            time: Some(12.5),
            conc: Some(3.0),
            cunit: Some("ppm".into()),
            rep: Some("A".into()),
        };
        for (k, s) in [("plain", Sample::default()), ("edited", edit)] {
            let got = open_items(&mut idx, &mut m, &name, &s)
                .unwrap_or_else(|e| panic!("{stem} {k}: {e}"));
            let w = want[k].as_array().unwrap();
            assert_eq!(got.len(), w.len(), "{stem} {k}: items");
            for (i, (g, w)) in got.iter().zip(w).enumerate() {
                let mut g: Value = serde_json::from_str(&g.info).unwrap();
                infusion_exception(&mut g, w);
                same(&format!("{stem} {k}[{i}]"), &g, w);
                items += 1;
            }
        }
        files += 1;
    }
    assert!(files >= 8 && items >= 16, "{files} files, {items} items");
    let _ = std::fs::remove_dir_all(&out);
}
