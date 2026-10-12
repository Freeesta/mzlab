//! `/api/msntree` from the index and the mzML (`Routes::mzml_route`) against `dispatch` of Python (`tools/msntree_rotta_riferimento.py`): the same
//! tree (numbers within 1e-9) and the same error keys. `MZLAB_MSN_DIR` names a folder already written by the reference (a real file); without it the
//! reference runs on a synthetic infusion. Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_core::routes::{Query, Routes};
use serde_json::Value;

fn close(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

fn same(path: &str, got: &Value, want: &Value) -> usize {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => {
            assert!(close(a.as_f64().unwrap(), b.as_f64().unwrap()), "{path}: {a} against {b}");
            1
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{path}: length");
            a.iter().zip(b).enumerate().map(|(i, (x, y))| same(&format!("{path}[{i}]"), x, y)).sum()
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>(), "{path}: keys");
            b.iter().map(|(k, y)| same(&format!("{path}.{k}"), &a[k], y)).sum()
        }
        (a, b) => {
            assert_eq!(a, b, "{path}");
            1
        }
    }
}

#[test]
fn msntree_route_like_python() {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/ref/msntree_rotta");           // the answers of Python for the synthetic infusion are committed: no python3 needed
    let out = match std::env::var("MZLAB_MSN_DIR") {
        Ok(d) => PathBuf::from(d),
        Err(_) if golden.exists() => golden,
        Err(_) => {
            let out = std::env::temp_dir().join(format!("mzlab-msntree-route-{}", std::process::id()));
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            let run = Command::new("python3").current_dir(&root).arg("tools/msntree_rotta_riferimento.py").arg(&out).output();
            if !matches!(&run, Ok(o) if o.status.success()) {
                assert!(std::env::var("MZLAB_GOLDEN_STRICT").is_err(), "the reference did not run: {run:?}");
                eprintln!("python3/numpy not available: test skipped");
                return;
            }
            out
        }
    };
    let cases: Vec<Value> = serde_json::from_slice(&std::fs::read(out.join("msntree.json")).unwrap()).unwrap();
    let p = out.join("msn.mzidx");
    let mut routes = Routes::new(Index::open(std::fs::File::open(&p).unwrap(), std::fs::metadata(&p).unwrap().len()).unwrap());
    let mut mzml = std::fs::File::open(out.join("msn.mzML")).unwrap();
    let (mut trees, mut errors, mut compared) = (0, 0, 0);
    for c in &cases {
        let query = c["query"].as_str().unwrap();
        let got = routes.mzml_route("/api/msntree", &Query::parse(query), &mut mzml);
        if c["status"] == 200 {
            let text = got.unwrap().unwrap_or_else(|| panic!("{query}: Python's answer expected from Rust"));
            compared += same(query, &serde_json::from_str(&text).unwrap(), &c["result"]);
            trees += 1;
        } else {
            match got {
                Err(e) => assert_eq!(Some(e.key), c["result"]["error_key"].as_str(), "{query}"),
                Ok(None) => assert!(c["result"]["error_key"].is_null(), "{query}: left to Python without a reason"),
                Ok(Some(t)) => panic!("{query}: a tree where Python refuses: {t}"),
            }
            errors += 1;
        }
    }
    assert!(trees >= 4 && errors >= 3 && compared > 100, "{trees} trees, {errors} errors, {compared} values");
}
