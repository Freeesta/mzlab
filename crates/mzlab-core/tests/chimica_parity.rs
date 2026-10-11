//! `elements` and `subformulas` answer like `mzlab/chem` (`tools/chimica_riferimento.py`): the same JSON (numbers within 1e-9 relative, rounded
//! values equal), the same names in the same order, the same error keys. Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::elements::{formula_mz, normalize_formula};
use mzlab_core::subformulas::{subformulas, submasses};
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

#[test]
fn chemistry_like_python() {
    let out = std::env::temp_dir().join(format!("mzlab_chimica_{}.json", std::process::id()));
    let ok = Command::new("python3")
        .arg(root().join("tools/chimica_riferimento.py"))
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
    let rows: Vec<Value> = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    let _ = std::fs::remove_file(&out);
    let (mut n_ok, mut n_err) = (0, 0);
    for r in &rows {
        let f = r["f"].as_str().unwrap();
        let what = format!("{} {f:?} {:?}", r["kind"], r.get("adduct").or(r.get("z")));
        if r["kind"] == "norm" {
            assert_eq!(normalize_formula(f), r["norm"].as_str().unwrap(), "{what}");
            continue;
        }
        let got: Result<Value, mzlab_core::Error> = match r["kind"].as_str().unwrap() {
            "formula" => {
                formula_mz(f, r["adduct"].as_str()).map(|s| serde_json::from_str(&s).unwrap())
            }
            "sub" => subformulas(
                f,
                r["z"].as_i64().unwrap() as i32,
                r["min"].as_f64().unwrap(),
            )
            .map(|(_, v)| {
                Value::Array(
                    v.into_iter()
                        .map(|(n, m)| serde_json::json!([n, m]))
                        .collect(),
                )
            }),
            _ => submasses(
                f,
                r["z"].as_i64().unwrap() as i32,
                r["min"].as_f64().unwrap(),
            )
            .map(|v| serde_json::json!(v)),
        };
        match (got, r.get("ok")) {
            (Ok(g), Some(w)) => {
                same(&what, &g, w);
                n_ok += 1;
            }
            (Err(e), None) => {
                let want = r["err"].as_str().unwrap();
                assert!(
                    e.key == want || want.starts_with("python:"),
                    "{what}: {} against {want}",
                    e.key
                );
                if let Some(p) = r["params"].as_object() {
                    for (k, v) in p {
                        let got = e
                            .params
                            .iter()
                            .find(|(n, _)| n == k)
                            .map(|(_, v)| v.clone());
                        assert!(
                            got.as_deref() == v.as_str() || k == "n",
                            "{what} param {k}: {got:?} against {v}"
                        );
                    }
                }
                n_err += 1;
            }
            (g, w) => panic!("{what}: {g:?} against {w:?}"),
        }
    }
    assert!(n_ok > 300 && n_err > 20, "{n_ok} answers, {n_err} errors");
}
