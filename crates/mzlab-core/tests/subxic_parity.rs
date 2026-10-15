//! `subxic::subxic` answers like `api.dispatch` on `/api/subxic` (`tools/subxic_riferimento.py`, two synthetic high-resolution DDA files): the same
//! rows in the same order, numbers within 1e-9; the requests Python refuses are Python's here too (`None`). Skipped without python3 and numpy
//! (`MZLAB_GOLDEN_STRICT=1` fails instead). `MZLAB_SUBXIC_DIR` = a folder already written by the script (e.g. from real files).

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_core::routes::{Query, Routes};
use mzlab_core::subxic::subxic;
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
                bad.push(format!("{path}: {} against {}", a.to_string().chars().take(60).collect::<String>(), b.to_string().chars().take(60).collect::<String>()));
            }
            1
        }
    }
}

#[test]
fn subxic_like_python() {
    let ready = std::env::var("MZLAB_SUBXIC_DIR").ok().map(PathBuf::from);
    let out = ready.clone().unwrap_or_else(|| std::env::temp_dir().join(format!("mzlab-subxic-{}", std::process::id())));
    let run = match ready {
        Some(_) => Command::new("true").output(),
        None => Command::new("python3").current_dir(root()).arg("tools/subxic_riferimento.py").arg(&out).output(),
    };
    if !matches!(&run, Ok(o) if o.status.success()) {
        assert!(std::env::var("MZLAB_GOLDEN_STRICT").is_err(), "the reference did not run: {run:?}");
        eprintln!("python3/numpy not available: test skipped");
        return;
    }
    let cases: Vec<Value> = serde_json::from_slice(&std::fs::read(out.join("subxic.json")).unwrap()).unwrap();
    let mut routes: Vec<Routes<std::fs::File>> = (0..2)
        .map(|i| {
            let p = out.join(format!("s{i}.mzidx"));
            Routes::new(Index::open(std::fs::File::open(&p).unwrap(), std::fs::metadata(&p).unwrap().len()).unwrap())
        })
        .collect();
    let (mut bad, mut compared) = (Vec::new(), 0);
    for c in &cases {
        let (k, query) = (c["k"].as_i64().unwrap(), c["query"].as_str().unwrap());
        let t = std::time::Instant::now();
        let got = subxic(&mut routes[k as usize / 2], k, &Query::parse(query));
        eprintln!("k={k} {query}: rust {:.0} ms, python {:.0} ms", t.elapsed().as_secs_f64() * 1e3, c["py_s"].as_f64().unwrap_or(f64::NAN) * 1e3);
        if c["status"] != 200 {
            if matches!(got, Ok(Some(_))) {
                bad.push(format!("k={k} {query}: Python refuses, Rust answers"));
            }
            continue;
        }
        let Some(got) = got.unwrap() else {
            bad.push(format!("k={k} {query}: Rust gave it to Python"));
            continue;
        };
        let before = bad.len();
        compared += same("", &serde_json::from_str(&got).unwrap(), &c["result"], &mut bad);
        bad[before..].iter_mut().for_each(|b| *b = format!("k={k} {query}: {b}"));
    }
    assert!(compared > 20_000, "only {compared} values compared");
    assert!(bad.is_empty(), "{} differences:\n{}", bad.len(), bad.iter().take(2000).cloned().collect::<Vec<_>>().join("\n"));
}
