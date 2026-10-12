//! `msntree::msn_tree` answers like `mzlab/chem/msntree.py` on synthetic fragmentation cascades (`tools/msntree_riferimento.py`): the same nodes,
//! parents, precursors, formulas and peaks (numbers within 1e-9), the same error keys. Skipped without python3 and numpy
//! (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::msntree::{msn_tree, TreeScan};
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
fn nums(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

#[test]
fn tree_like_python() {
    let golden = root().join("tests/golden/ref/msntree.json");
    let committed = golden.exists();           // the answers of Python for the synthetic data are committed: no python3 needed
    let out = if committed { golden } else { std::env::temp_dir().join(format!("mzlab_msntree_{}.json", std::process::id())) };
    let ok = committed || Command::new("python3")
        .arg(root().join("tools/msntree_riferimento.py"))
        .arg(&out)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        eprintln!("skipped: python3/numpy not available");
        return;
    }
    let cases: Vec<Value> = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    if !committed { let _ = std::fs::remove_file(&out); }
    let (mut trees, mut errors, mut formulas) = (0, 0, 0);
    for (i, c) in cases.iter().enumerate() {
        let scans: Vec<TreeScan> = c["scans"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| TreeScan {
                index: s["index"].as_u64().unwrap() as usize,
                level: s["level"].as_u64().unwrap() as u8,
                polarity: s["polarity"].as_i64().unwrap() as i8,
                path: s["path"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| {
                        (
                            p[0].as_f64().unwrap(),
                            p[1].as_str().unwrap().to_string(),
                            p[2].as_f64().unwrap(),
                        )
                    })
                    .collect(),
            })
            .collect();
        let spectra = c["spectra"].as_object().unwrap();
        let read = |k: usize| {
            let s = &spectra[&k.to_string()];
            Ok((nums(&s[0]), nums(&s[1])))
        };
        let got = msn_tree(
            &scans,
            read,
            c["formula"].as_str(),
            c["ppm"].as_f64().unwrap(),
        );
        match (got, c.get("ok")) {
            (Ok(g), Some(w)) => {
                let g: Value = serde_json::from_str(&g).unwrap();
                same(&format!("case {i}"), &g, w);
                trees += 1;
                formulas += w["nodes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|n| !n["formula"].is_null())
                    .count();
            }
            (Err(e), None) => {
                assert_eq!(e.key, c["err"].as_str().unwrap(), "case {i}");
                errors += 1;
            }
            (g, w) => panic!("case {i}: {g:?} against {w:?}"),
        }
    }
    assert!(
        trees >= 5 && errors >= 3 && formulas >= 10,
        "{trees} trees, {errors} errors, {formulas} node formulas"
    );
}
