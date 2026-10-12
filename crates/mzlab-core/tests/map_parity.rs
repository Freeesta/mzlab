//! The map routes of `maproute` give what `api.dispatch` gives (`tools/mappa_riferimento.py`): the same keys, the arrays of cells bit for bit,
//! the other numbers within 1e-9 relative; the answers Python refuses (an error) are Python's here too (`None`).
//! Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use base64::Engine;
use mzlab_core::idxread::Index;
use mzlab_core::maproute::map_route;
use mzlab_core::originroute::ItemMeta;
use mzlab_core::routes::{Query, Routes};
use serde_json::Value;

const ARRAYS: [&str; 4] = ["data", "rt", "mz", "inten"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Compares two answers; arrays of cells (base64) must be identical. Returns the number of values compared.
fn compare(path: &str, got: &Value, want: &Value, bad: &mut Vec<String>) -> usize {
    match (got, want) {
        (Value::Object(g), Value::Object(w)) => {
            let mut n = 0;
            for (k, wv) in w {
                match g.get(k) {
                    Some(gv) if ARRAYS.contains(&k.as_str()) && wv.is_string() => n += same_bytes(&format!("{path}.{k}"), gv, wv, bad),
                    Some(gv) => n += compare(&format!("{path}.{k}"), gv, wv, bad),
                    None => bad.push(format!("{path}.{k}: missing")),
                }
            }
            bad.extend(g.keys().filter(|k| !w.contains_key(*k)).map(|k| format!("{path}.{k}: extra")));
            n
        }
        (Value::Array(g), Value::Array(w)) if g.len() == w.len() => g.iter().zip(w).enumerate().map(|(i, (a, b))| compare(&format!("{path}[{i}]"), a, b, bad)).sum(),
        (Value::Number(g), Value::Number(w)) => {
            let (a, b) = (g.as_f64().unwrap(), w.as_f64().unwrap());
            if !(a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs())) {
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

fn same_bytes(path: &str, got: &Value, want: &Value, bad: &mut Vec<String>) -> usize {
    let dec = |v: &Value| base64::engine::general_purpose::STANDARD.decode(v.as_str().unwrap_or("")).unwrap_or_default();
    let (g, w) = (dec(got), dec(want));
    if g.len() != w.len() {
        bad.push(format!("{path}: {} bytes against {}", g.len(), w.len()));
        return 0;
    }
    let diff = g.chunks(4).zip(w.chunks(4)).filter(|(a, b)| a != b).count();
    if diff > 0 {
        bad.push(format!("{path}: {diff} of {} words differ", g.len() / 4));
    }
    g.len() / 4
}

#[test]
fn rust_maps_match_python() {
    let ready = std::env::var("MZLAB_MAP_DIR").ok().map(PathBuf::from);
    let out = ready.clone().unwrap_or_else(|| std::env::temp_dir().join(format!("mzlab-map-{}", std::process::id())));
    let run = match ready {
        Some(_) => Command::new("true").output(),
        None => Command::new("python3").current_dir(root()).arg("tools/mappa_riferimento.py").arg(&out).output(),
    };
    if !matches!(&run, Ok(o) if o.status.success()) {
        if std::env::var("MZLAB_GOLDEN_STRICT").is_ok() {
            panic!("the reference did not run: {run:?}");
        }
        eprintln!("python3/numpy not available: test skipped");
        return;
    }
    let cases: Vec<Value> = serde_json::from_slice(&std::fs::read(out.join("mappa.json")).unwrap()).unwrap();
    let mut routes: HashMap<u32, Routes<std::fs::File>> = (0..3u32)
        .map(|i| {
            let p = out.join(format!("s{i}.mzidx"));
            (i, Routes::new(Index::open(std::fs::File::open(&p).unwrap(), std::fs::metadata(&p).unwrap().len()).unwrap()))
        })
        .collect();
    let items: Vec<ItemMeta> = (0..3u32).map(|i| ItemMeta { slot: i, label: format!("s{i}"), time: None, full: true }).collect();
    let (mut failures, mut compared) = (Vec::new(), 0);
    for c in &cases {
        let (path, query) = (c["path"].as_str().unwrap(), c["query"].as_str().unwrap());
        let t = std::time::Instant::now();
        let got = map_route(path, &items, &mut routes, &Query::parse(query)).unwrap();
        eprintln!("{path}?{}: rust {:.1} ms", &query[..query.len().min(60)], t.elapsed().as_secs_f64() * 1e3);
        if c["status"] != 200 {
            if got.is_some() {
                failures.push(format!("{path}?{query}: Python refuses, Rust answers"));
            }
            continue;
        }
        let Some(got) = got else {
            failures.push(format!("{path}?{query}: Rust gave it to Python"));
            continue;
        };
        let got: Value = serde_json::from_str(&got).unwrap();
        let mut bad = Vec::new();
        compared += compare("", &got, &c["result"], &mut bad);
        failures.extend(bad.iter().take(8).map(|b| format!("{path}?{query}: {b}")));
    }
    assert!(compared > 100_000, "only {compared} values compared");
    assert!(failures.is_empty(), "{} differences:\n{}", failures.len(), failures.join("\n"));
}
