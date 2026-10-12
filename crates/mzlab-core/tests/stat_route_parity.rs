//! `statroute::stat_route` answers like `api.dispatch` on the POST routes `api/stat/*` (`tools/statistica_riferimento.py`, invented tables):
//! the same JSON (numbers within 1e-9, indices, counts, labels and error keys identical). Skipped without python3 and numpy
//! (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::Path;
use std::process::Command;

use mzlab_core::statroute::stat_route;
use serde_json::Value;

fn same(path: &str, got: &Value, want: &Value, bad: &mut Vec<String>) -> usize {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            if !(a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)) {
                bad.push(format!("{path}: {a} against {b}"));
            }
            1
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => a.iter().zip(b).enumerate().map(|(i, (x, y))| same(&format!("{path}[{i}]"), x, y, bad)).sum(),
        (Value::Object(a), Value::Object(b)) if a.keys().eq(b.keys()) => b.iter().map(|(k, y)| same(&format!("{path}.{k}"), &a[k], y, bad)).sum(),
        (a, b) => {
            if a != b {
                bad.push(format!("{path}: {} against {}", a.to_string().chars().take(80).collect::<String>(), b.to_string().chars().take(80).collect::<String>()));
            }
            1
        }
    }
}

/// With as many components as samples the last one is the null space of the centred data: its loadings are an arbitrary basis (in LAPACK
/// too), so they are not compared.
fn null_component(v: &mut Value) {
    let n = v["n_samples"].as_u64().unwrap_or(0) as usize;
    if let Some(rows) = v["loadings"].as_array_mut() {
        for r in rows.iter_mut().filter_map(|r| r.as_array_mut()).filter(|r| r.len() == n && n > 0) {
            r.pop();
        }
    }
}

#[test]
fn stat_routes_like_python() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ready = std::env::var("MZLAB_STAT_JSON").ok().map(std::path::PathBuf::from);
    let out = ready.clone().unwrap_or_else(|| std::env::temp_dir().join(format!("mzlab-stat-{}.json", std::process::id())));
    let run = match ready {
        Some(_) => Command::new("true").output(),
        None => Command::new("python3").current_dir(&root).arg("tools/statistica_riferimento.py").arg(&out).output(),
    };
    if !matches!(&run, Ok(o) if o.status.success()) {
        assert!(std::env::var("MZLAB_GOLDEN_STRICT").is_err(), "the reference did not run: {run:?}");
        eprintln!("python3/numpy not available: test skipped");
        return;
    }
    let cases: Vec<Value> = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    let (mut bad, mut compared) = (Vec::new(), 0);
    for (n, c) in cases.iter().enumerate() {
        let (path, body) = (c["path"].as_str().unwrap(), c["body"].to_string());
        let got = stat_route(path, &body).unwrap_or_else(|| panic!("{path} is not a route of the Rust engine"));
        let (status, text) = match got {
            Ok(t) => (200, t),
            Err(t) => (400, t),
        };
        let before = bad.len();
        let (mut got, mut want) = (serde_json::from_str::<Value>(&text).unwrap(), c["result"].clone());
        null_component(&mut got);
        null_component(&mut want);
        if status != c["status"].as_i64().unwrap() {
            bad.push(format!("status {status} against {}", c["status"]));
        }
        compared += same("", &got, &want, &mut bad);
        bad[before..].iter_mut().for_each(|b| *b = format!("case {n} {path}: {b}"));
    }
    assert!(compared > 5_000, "only {compared} values compared");
    assert!(bad.is_empty(), "{} differences:\n{}", bad.len(), bad.iter().take(60).cloned().collect::<Vec<_>>().join("\n"));
}
