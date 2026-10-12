//! `/api/spectrum` with a background (`bgk`) from the index(es) answers like `api.dispatch` (`tools/fondo_riferimento.py`: synthetic low-resolution
//! centroid, low-resolution profile and high-resolution sessions): same lines, numbers within 1e-9; a request Python refuses is left to Python.
//! Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead). `MZLAB_FONDO_DIR` = a folder already written by the script.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_core::routes::{Query, Routes};
use mzlab_core::specdiff::spectrum_diff;
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
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => a
            .iter()
            .zip(b)
            .enumerate()
            .map(|(i, (x, y))| same(&format!("{path}[{i}]"), x, y, bad))
            .sum(),
        (Value::Object(a), Value::Object(b)) if a.keys().eq(b.keys()) => b
            .iter()
            .map(|(k, y)| same(&format!("{path}.{k}"), &a[k], y, bad))
            .sum(),
        (a, b) => {
            if a != b {
                bad.push(format!(
                    "{path}: {} against {}",
                    a.to_string().chars().take(70).collect::<String>(),
                    b.to_string().chars().take(70).collect::<String>()
                ));
            }
            1
        }
    }
}

fn open(out: &Path, name: &str) -> Routes<std::fs::File> {
    let p = out.join(Path::new(name).with_extension("mzidx"));
    Routes::new(
        Index::open(
            std::fs::File::open(&p).unwrap(),
            std::fs::metadata(&p).unwrap().len(),
        )
        .unwrap(),
    )
}

#[test]
fn spectrum_with_background_like_python() {
    let ready = std::env::var("MZLAB_FONDO_DIR").ok().map(PathBuf::from);
    let out = ready.clone().unwrap_or_else(|| {
        std::env::temp_dir().join(format!("mzlab-fondo-{}", std::process::id()))
    });
    let run = match ready {
        Some(_) => Command::new("true").output(),
        None => Command::new("python3")
            .current_dir(root())
            .arg("tools/fondo_riferimento.py")
            .arg(&out)
            .output(),
    };
    if !matches!(&run, Ok(o) if o.status.success()) {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "the reference did not run: {run:?}"
        );
        eprintln!("python3/numpy not available: test skipped");
        return;
    }
    let cases: Vec<Value> =
        serde_json::from_slice(&std::fs::read(out.join("fondo.json")).unwrap()).unwrap();
    let mut files: HashMap<(String, usize), Routes<std::fs::File>> = HashMap::new();
    let (mut bad, mut compared, mut answered) = (Vec::new(), 0, 0);
    for c in &cases {
        let sid = c["session"].as_str().unwrap().to_string();
        let names: Vec<&str> = c["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        let (fk, fbk) = (
            c["fk"].as_u64().unwrap() as usize,
            c["fbk"].as_u64().unwrap() as usize,
        );
        if c["bgk"].is_null() {
            continue; // no background asked: the plain spectrum, not this module
        }
        let query = c["query"].as_str().unwrap();
        let label = format!("{sid} k={fk} bg={fbk} ?{query}");
        let q = Query::parse(query);
        let mut main = files
            .remove(&(sid.clone(), fk))
            .unwrap_or_else(|| open(&out, names[fk]));
        let got = match main.diff_request(&q).unwrap() {
            None => None,
            Some(req) if fk == fbk => spectrum_diff(&mut main, None, &req).unwrap(),
            Some(req) => {
                let mut bg = files
                    .remove(&(sid.clone(), fbk))
                    .unwrap_or_else(|| open(&out, names[fbk]));
                let a = spectrum_diff(&mut main, Some(&mut bg), &req).unwrap();
                files.insert((sid.clone(), fbk), bg);
                a
            }
        };
        files.insert((sid.clone(), fk), main);
        if c["status"] != 200 {
            if got.is_some() {
                bad.push(format!("{label}: Python refuses, Rust answers"));
            }
            continue;
        }
        let Some(got) = got else {
            bad.push(format!("{label}: Rust gave it to Python"));
            continue;
        };
        answered += 1;
        let before = bad.len();
        compared += same(
            "",
            &serde_json::from_str(&got).unwrap(),
            &c["result"],
            &mut bad,
        );
        bad[before..]
            .iter_mut()
            .for_each(|b| *b = format!("{label}: {b}"));
    }
    assert!(
        answered > 60 && compared > 3_000,
        "only {answered} answers and {compared} values compared"
    );
    assert!(
        bad.is_empty(),
        "{} differences:\n{}",
        bad.len(),
        bad.iter().take(20).cloned().collect::<Vec<_>>().join("\n")
    );
}
