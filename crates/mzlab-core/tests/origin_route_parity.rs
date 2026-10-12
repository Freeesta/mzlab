//! `origin_route` over a whole session gives what `api.dispatch` gives for `api/origin` (`tools/origin_riferimento.py`, `origin-api.json`):
//! unit-resolution centroids, unit-resolution profile and high-resolution centroids, each with a product-ion file; same keys, indices and counts
//! identical, numbers within 1e-8 relative. Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_core::jsonb::write;
use mzlab_core::originroute::{origin_route, Ask, ItemMeta};
use mzlab_core::routes::{Query, Routes};
use serde_json::Value;

/// Relative tolerance of the real numbers (indices and counts are exact) and the absolute floor for quantities that are nearly zero: the
/// baseline and the trapezoids add in another order than numpy's, which moves the last digits (a few 1e-9).
const TOL: f64 = 1e-8;
const FLOOR: f64 = 1e-12;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Compares two reports leaf by leaf; returns the number of numbers compared.
fn compare(path: &str, got: &Value, want: &Value, bad: &mut Vec<String>) -> usize {
    match (got, want) {
        (Value::Object(g), Value::Object(w)) => {
            let mut n = 0;
            for (k, wv) in w {
                match g.get(k) {
                    Some(v) => n += compare(&format!("{path}.{k}"), v, wv, bad),
                    None => bad.push(format!("{path}.{k}: missing")),
                }
            }
            bad.extend(g.keys().filter(|k| !w.contains_key(*k)).map(|k| format!("{path}.{k}: extra")));
            n
        }
        (Value::Array(g), Value::Array(w)) if g.len() == w.len() => g.iter().zip(w).enumerate().map(|(i, (a, b))| compare(&format!("{path}[{i}]"), a, b, bad)).sum(),
        (Value::Array(g), Value::Array(w)) => {
            bad.push(format!("{path}: length {} against {}", g.len(), w.len()));
            0
        }
        (Value::Number(g), Value::Number(w)) => {
            let (a, b) = (g.as_f64().unwrap(), w.as_f64().unwrap());
            let ok = if w.is_i64() { a == b } else { a == b || (a - b).abs() <= TOL * a.abs().max(b.abs()) + FLOOR };
            if !ok {
                bad.push(format!("{path}: {a} against {b}"));
            }
            1
        }
        (g, w) => {
            if g != w {
                bad.push(format!("{path}: {g} against {w}"));
            }
            1
        }
    }
}

/// One index per item of the session (a part of a mixed file opens its file again, as the page does).
fn open_all(out: &Path, items: &[Value]) -> HashMap<u32, Routes<std::fs::File>> {
    items
        .iter()
        .enumerate()
        .map(|(i, it)| {
            let p = out.join(format!("{}.mzidx", Path::new(it["file"].as_str().unwrap()).file_stem().unwrap().to_str().unwrap()));
            let idx = Index::open(std::fs::File::open(&p).unwrap(), std::fs::metadata(&p).unwrap().len()).unwrap();
            (i as u32, Routes::new(idx))
        })
        .collect()
}

#[test]
fn rust_origin_route_matches_python() {
    let ready = std::env::var("MZLAB_ORIGIN_DIR").ok().map(PathBuf::from);
    let out = ready.clone().unwrap_or_else(|| std::env::temp_dir().join(format!("mzlab-origin-api-{}", std::process::id())));
    let run = match ready {
        Some(_) => Command::new("true").output(),
        None => Command::new("python3").current_dir(root()).arg("tools/origin_riferimento.py").arg(&out).output(),
    };
    if !matches!(&run, Ok(o) if o.status.success()) {
        if std::env::var("MZLAB_GOLDEN_STRICT").is_ok() {
            panic!("the reference did not run: {run:?}");
        }
        eprintln!("python3/numpy not available: test skipped");
        return;
    }
    let cases: Vec<Value> = serde_json::from_slice(&std::fs::read(out.join("origin-api.json")).unwrap()).unwrap();
    let (mut failures, mut compared) = (Vec::new(), 0);
    let mut sessions: HashMap<String, HashMap<u32, Routes<std::fs::File>>> = HashMap::new();
    for c in &cases {
        let (set, query) = (c["set"].as_str().unwrap().to_string(), c["query"].as_str().unwrap());
        let metas = c["items"].as_array().unwrap();
        let routes = sessions.entry(set.clone()).or_insert_with(|| open_all(&out, metas));
        let items: Vec<ItemMeta> = metas
            .iter()
            .enumerate()
            .map(|(i, it)| ItemMeta { slot: i as u32, label: it["label"].as_str().unwrap().to_string(), time: it["time"].as_f64(), full: it["full"].as_bool().unwrap() })
            .collect();
        let q = Query::parse(query);
        let k = q.get("k").and_then(|v| v.parse().ok()).unwrap_or(-1);
        let num = |n: &str| q.get(n).and_then(|v| v.parse::<f64>().ok());
        let ask = Ask { k, mz: num("mz").unwrap(), parent: num("parent").unwrap(), rt0: num("rt0"), rt1: num("rt1"), formula: q.get("formula") };
        let t = std::time::Instant::now();
        let rep = origin_route(&items, routes, &ask).unwrap();
        eprintln!("{set}: origin {}: rust {:.1} ms, python {:.1} ms", &query[..query.len().min(50)], t.elapsed().as_secs_f64() * 1e3, c["py_s"].as_f64().unwrap_or(f64::NAN) * 1e3);
        let Some(rep) = rep else {
            failures.push(format!("{set} {query}: Rust gave it to Python"));
            continue;
        };
        let got: Value = serde_json::from_str(&write(&rep)).unwrap();
        let mut got = got;
        got.as_object_mut().unwrap().remove("timing");
        let mut bad = Vec::new();
        compared += compare("", &got, &c["result"], &mut bad);
        failures.extend(bad.iter().take(10).map(|b| format!("{set} {query}: {b}")));
    }
    assert!(compared > 2000, "only {compared} numbers compared");
    assert!(failures.is_empty(), "{} differences:\n{}", failures.len(), failures.join("\n"));
}
