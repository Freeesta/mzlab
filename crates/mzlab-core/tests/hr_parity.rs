//! `/api/dda` and `/api/spectrum` on HIGH-RESOLUTION centroid files answer from the index like `api.dispatch` (`tools/hr_riferimento.py`, two synthetic
//! Orbitrap DDA files): the same columns and peaks, numbers within 1e-9; a window of one scan is read from the mzML (`spectrum_exact`; the index alone answers `None`).
//! Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead). `MZLAB_HR_DIR` = a folder already written by the script.

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_core::routes::{Query, Routes};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

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
                bad.push(format!("{path}: {} against {}", a.to_string().chars().take(70).collect::<String>(), b.to_string().chars().take(70).collect::<String>()));
            }
            1
        }
    }
}

#[test]
fn hr_routes_like_python() {
    let ready = std::env::var("MZLAB_HR_DIR").ok().map(PathBuf::from);
    let out = ready.clone().unwrap_or_else(|| std::env::temp_dir().join(format!("mzlab-hr-{}", std::process::id())));
    let run = match ready {
        Some(_) => Command::new("true").output(),
        None => Command::new("python3").current_dir(root()).arg("tools/hr_riferimento.py").arg(&out).output(),
    };
    if !matches!(&run, Ok(o) if o.status.success()) {
        assert!(std::env::var("MZLAB_GOLDEN_STRICT").is_err(), "the reference did not run: {run:?}");
        eprintln!("python3/numpy not available: test skipped");
        return;
    }
    let cases: Vec<Value> = serde_json::from_slice(&std::fs::read(out.join("hr.json")).unwrap()).unwrap();
    let mut routes: Vec<Routes<std::fs::File>> = (0..2)
        .map(|i| {
            let p = out.join(format!("h{i}.mzidx"));
            Routes::new(Index::open(std::fs::File::open(&p).unwrap(), std::fs::metadata(&p).unwrap().len()).unwrap())
        })
        .collect();
    let (mut bad, mut compared) = (Vec::new(), 0);
    for c in &cases {
        let (k, path, query) = (c["k"].as_u64().unwrap() as usize, c["path"].as_str().unwrap(), c["query"].as_str().unwrap());
        let t = std::time::Instant::now();
        let got = routes[k].handle(path, &Query::parse(query)).unwrap();
        eprintln!("k={k} {path}?{query}: rust {:.1} ms, python {:.1} ms", t.elapsed().as_secs_f64() * 1e3, c["py_s"].as_f64().unwrap_or(f64::NAN) * 1e3);
        if c["single"] == true {
            if got.is_some() {
                bad.push(format!("k={k} {path}?{query}: a window of one scan needs the mzML"));
            }
            let mut mzml = std::fs::File::open(out.join(format!("h{k}.mzML"))).unwrap();
            let Some(got) = routes[k].spectrum_exact(&Query::parse(query), &mut mzml).unwrap() else {
                bad.push(format!("k={k} {path}?{query}: spectrum_exact gave it to Python"));
                continue;
            };
            let before = bad.len();
            compared += same("", &serde_json::from_str(&got).unwrap(), &c["result"], &mut bad);
            bad[before..].iter_mut().for_each(|b| *b = format!("k={k} {path}?{query}: {b}"));
            continue;
        }
        if c["status"] != 200 {
            if got.is_some() {
                bad.push(format!("k={k} {path}?{query}: Python refuses, Rust answers"));
            }
            continue;
        }
        let Some(got) = got else {
            bad.push(format!("k={k} {path}?{query}: Rust gave it to Python"));
            continue;
        };
        let before = bad.len();
        compared += same("", &serde_json::from_str(&got).unwrap(), &c["result"], &mut bad);
        bad[before..].iter_mut().for_each(|b| *b = format!("k={k} {path}?{query}: {b}"));
    }
    assert!(compared > 20_000, "only {compared} values compared");
    assert!(bad.is_empty(), "{} differences:\n{}", bad.len(), bad.iter().take(20).cloned().collect::<Vec<_>>().join("\n"));
}
