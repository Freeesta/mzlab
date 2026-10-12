//! `comproute::composition_route` reads the query of `/api/composition` and answers like `api.dispatch` (`tools/composizione_rotta_riferimento.py`):
//! the same JSON for the requests Python accepts (numbers within 1e-9), `None` (Python's) for the ones it rejects.
//! Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::comproute::composition_route;
use mzlab_core::routes::Query;
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn same(path: &str, got: &Value, want: &Value, bad: &mut Vec<String>) {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            if !(a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)) {
                bad.push(format!("{path}: {a} against {b}"));
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => a.iter().zip(b).enumerate().for_each(|(i, (x, y))| same(&format!("{path}[{i}]"), x, y, bad)),
        (Value::Object(a), Value::Object(b)) if a.keys().eq(b.keys()) => b.iter().for_each(|(k, y)| same(&format!("{path}.{k}"), &a[k], y, bad)),
        (a, b) => {
            if a != b {
                bad.push(format!("{path}: {a} against {b}"));
            }
        }
    }
}

#[test]
fn composition_route_like_python() {
    let out = std::env::temp_dir().join(format!("mzlab-comp-route-{}.json", std::process::id()));
    let ok = Command::new("python3").current_dir(root()).arg("tools/composizione_rotta_riferimento.py").arg(&out).output().map(|o| o.status.success()).unwrap_or(false);
    if !ok {
        assert!(std::env::var("MZLAB_GOLDEN_STRICT").is_err(), "python3 or numpy missing");
        eprintln!("skipped: python3/numpy not available");
        return;
    }
    let rows: Vec<Value> = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    let _ = std::fs::remove_file(&out);
    let (mut bad, mut answered) = (Vec::new(), 0);
    for r in &rows {
        let query = r["query"].as_str().unwrap();
        let t = std::time::Instant::now();
        let got = composition_route(&Query::parse(query));
        eprintln!("composition {}: rust {:.1} ms, python {:.1} ms", &query[..query.len().min(70)], t.elapsed().as_secs_f64() * 1e3, r["py_s"].as_f64().unwrap_or(f64::NAN) * 1e3);
        if r["status"] != 200 {
            if got.is_some() {
                bad.push(format!("{query}: Python refuses, Rust answers"));
            }
            continue;
        }
        let Some(got) = got else {
            bad.push(format!("{query}: Rust gave it to Python"));
            continue;
        };
        answered += 1;
        let before = bad.len();
        same("", &serde_json::from_str(&got).unwrap(), &r["result"], &mut bad);
        bad[before..].iter_mut().for_each(|b| *b = format!("{query}: {b}"));
    }
    assert!(answered >= 40, "only {answered} requests answered");
    assert!(bad.is_empty(), "{} differences:\n{}", bad.len(), bad.iter().take(15).cloned().collect::<Vec<_>>().join("\n"));
}
