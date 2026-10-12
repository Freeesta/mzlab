//! `api/qaqc` in Rust gives the JSON of Python (`tools/qaqc_riferimento.py`): same keys, numbers within 1e-9 (relative), the user's error
//! with the same key, a request Python must word itself declined. Without python3 and numpy the test is skipped (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_core::qaqc::plan;
use mzlab_core::routes::{Query, Routes};
use serde_json::Value;

fn close(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

fn same(path: &str, got: &Value, want: &Value, what: &str) {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => assert!(close(a.as_f64().unwrap(), b.as_f64().unwrap()), "{what} {path}: {a} against {b}"),
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{what} {path}: length");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                same(&format!("{path}[{i}]"), x, y, what);
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>(), "{what} {path}: keys");
            for (k, y) in b {
                same(&format!("{path}.{k}"), &a[k], y, what);
            }
        }
        (a, b) => assert_eq!(a, b, "{what} {path}"),
    }
}

fn enc(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

#[test]
fn qaqc_matches_python() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = std::env::temp_dir().join(format!("mzlab-qaqc-{}", std::process::id()));
    let listing = match Command::new("python3").current_dir(&root).arg("tools/qaqc_riferimento.py").arg(&out).output() {
        Ok(o) if o.status.success() => String::from_utf8(o.stdout).unwrap(),
        other => {
            if std::env::var("MZLAB_GOLDEN_STRICT").is_ok() {
                panic!("the reference did not run: {other:?}");
            }
            eprintln!("python3/numpy not available: test skipped");
            return;
        }
    };
    let mut routes: Vec<Routes<std::fs::File>> = listing
        .lines()
        .filter(|l| !l.is_empty())
        .map(|f| {
            let p = out.join(format!("{}.mzidx", PathBuf::from(f).file_stem().unwrap().to_string_lossy()));
            Routes::new(Index::open(std::fs::File::open(&p).unwrap(), std::fs::metadata(&p).unwrap().len()).unwrap())
        })
        .collect();
    let rows: Vec<Value> = serde_json::from_slice(&std::fs::read(out.join("qaqc.json")).unwrap()).unwrap();
    let (mut compared, mut declined, mut failed) = (0, 0, 0);
    for r in &rows {
        let qs = r["query"].as_object().unwrap().iter().map(|(k, v)| format!("{k}={}", enc(v.as_str().unwrap()))).collect::<Vec<_>>().join("&");
        let status = r["status"].as_u64().unwrap();
        let answer = match plan(&Query::parse(&qs)) {
            None => None,
            Some(Err(e)) => Some(Err(e)),
            Some(Ok(p)) => {
                let measured: Vec<_> = p.ks.iter().map(|&k| p.measure(k, &mut routes[k as usize]).unwrap().unwrap()).collect();
                Some(Ok(p.finish(&measured)))
            }
        };
        match answer {
            None => {
                assert!(status != 200, "{qs}: declined, but Python answers it");
                declined += 1;
            }
            Some(Err(e)) => {
                assert_eq!(status, 400, "{qs}");
                same("", &serde_json::from_str(&e).unwrap(), &r["body"], &qs);
                failed += 1;
            }
            Some(Ok(j)) => {
                assert_eq!(status, 200, "{qs}: Rust answers, Python fails");
                same("", &serde_json::from_str(&j).unwrap(), &r["body"], &qs);
                compared += 1;
            }
        }
    }
    assert!(compared >= 290 && declined >= 4 && failed >= 2, "{compared} answers, {declined} declined, {failed} errors");
    let _ = std::fs::remove_dir_all(&out);
}
