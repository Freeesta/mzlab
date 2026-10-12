//! The routes that Rust answers from the `.mzidx` give the JSON of the Python engine (`tools/rotte_riferimento.py`): same keys, same
//! lengths, identical indices and counts, numbers within 1e-9 (relative). Without python3 and numpy the test is skipped
//! (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_core::routes::{Query, Routes};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn close(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}

fn same(path: &str, got: &Value, want: &Value, what: &str) {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => {
            assert!(
                close(a.as_f64().unwrap(), b.as_f64().unwrap()),
                "{what} {path}: {a} against {b}"
            )
        }
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{what} {path}: length");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                same(&format!("{path}[{i}]"), x, y, what);
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(
                a.keys().collect::<Vec<_>>().len(),
                b.keys().count(),
                "{what} {path}: keys {a:?} against {b:?}"
            );
            for (k, y) in b {
                same(
                    &format!("{path}.{k}"),
                    a.get(k).unwrap_or_else(|| panic!("{what} {path}: no {k}")),
                    y,
                    what,
                );
            }
        }
        (a, b) => assert_eq!(a, b, "{what} {path}"),
    }
}

/// Percent-encoding as `URLSearchParams` does it (a `+` in a filter must come back as a `+`).
fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

#[test]
fn rust_routes_match_python() {
    let out = std::env::temp_dir().join(format!("mzlab-rot-{}", std::process::id()));
    let mut cmd = Command::new("python3");
    cmd.current_dir(root())
        .arg("tools/rotte_riferimento.py")
        .arg(&out);
    if let Ok(rd) = std::fs::read_dir(root().join("mzlab/web/esempi")) {
        let mut v: Vec<_> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "mzML"))
            .collect();
        v.sort();
        cmd.args(v);
    }
    let listing = match cmd.output() {
        Ok(o) if o.status.success() => String::from_utf8(o.stdout).unwrap(),
        other => {
            if std::env::var("MZLAB_GOLDEN_STRICT").is_ok() {
                panic!("the reference did not run: {other:?}");
            }
            eprintln!("python3/numpy not available: test skipped");
            return;
        }
    };
    let (mut files, mut compared) = (0, 0);
    for line in listing.lines().filter(|l| !l.is_empty()) {
        let stem = PathBuf::from(line)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let idx_path = out.join(format!("{stem}.mzidx"));
        let size = std::fs::metadata(&idx_path).unwrap().len();
        let idx = Index::open(std::fs::File::open(&idx_path).unwrap(), size)
            .unwrap_or_else(|e| panic!("{stem}: {e}"));
        let mut routes = Routes::new(idx);
        let mut mzml = std::fs::File::open(line).unwrap();
        let rows: Vec<Value> =
            serde_json::from_slice(&std::fs::read(out.join(format!("{stem}.rotte.json"))).unwrap())
                .unwrap();
        for r in &rows {
            let path = r["path"].as_str().unwrap();
            let q = r["query"].as_object().unwrap();
            let what = format!("{stem} {path} {q:?}");
            let qs = q
                .iter()
                .map(|(k, v)| format!("{k}={}", enc(v.as_str().unwrap())))
                .collect::<Vec<_>>()
                .join("&");
            let got = if path == "/api/fileinfo" || path == "/api/mrm" {
                let size = std::fs::metadata(line).unwrap().len();
                let mut mz = mzlab_core::mzmlhead::Mzml::new(std::fs::File::open(line).unwrap(), size);
                if path == "/api/mrm" {
                    let end = routes.idx.scans.last().map_or(0, |s| s.end);
                    let chroms = mz.chromatograms(end).unwrap();
                    Some(mzlab_core::fileroutes::mrm_files(&[(0, mzlab_core::fileroutes::mrm_transitions(&mut mz, &chroms).unwrap())]))
                } else {
                    let head = mz.info_head().unwrap();
                    let name = PathBuf::from(line).file_name().unwrap().to_string_lossy().to_string();
                    Some(mzlab_core::fileroutes::fileinfo(&mut routes, &mut mz, &name, &head).unwrap_or_else(|e| panic!("{what}: {e}")))
                }
            } else if matches!(path, "/api/method" | "/api/delay" | "/api/analog") || q.get("kind").is_some_and(|k| k == "pda") {
                if r["body"].get("scan_params").is_some() {
                    continue; // high resolution: Python's
                }
                item_answer(&mut routes, line, path, q, r, &what)
            } else if path == "/api/scaninfo" {
                let sid = q["sid"].as_str().unwrap().parse().unwrap();
                Some(routes.scaninfo(sid, &mut mzml).unwrap_or_else(|e| panic!("{what}: {e}")))
            } else if path == "/api/xic" {
                let f = |k: &str| q[k].as_str().unwrap().parse::<f64>().unwrap();
                Some(routes.xic(f("mz"), f("tol"), f("level") as u8).unwrap())
            } else {
                routes
                    .handle(path, &Query::parse(&qs))
                    .unwrap_or_else(|e| panic!("{what}: {e}"))
            };
            let want = if path == "/api/xic" {
                r["body"]["traces"][0].clone()
            } else {
                r["body"].clone()
            };
            if r["status"] != 200 {
                assert!(got.is_none(), "{what}: Python fails, Rust must decline");
                continue;
            }
            let got = got.unwrap_or_else(|| panic!("{what}: declined"));
            let got: Value = serde_json::from_str(&got).unwrap();
            let want = if path == "/api/xic" {
                serde_json::json!({"rt": want["rt"], "y": want["y"]})
            } else {
                want
            };
            same("", &got, &want, &what);
            compared += 1;
        }
        files += 1;
    }
    assert!(
        files >= 8 && compared > 200,
        "{files} files, {compared} answers"
    );
    let _ = std::fs::remove_dir_all(&out);
}

/// The routes of one item (`method`, `delay`, `analog`, the PDA trace): the answer of Rust, `None` when it declines (as Python fails).
fn item_answer(routes: &mut Routes<std::fs::File>, line: &str, path: &str, q: &serde_json::Map<String, Value>, r: &Value, what: &str) -> Option<String> {
    let size = std::fs::metadata(line).unwrap().len();
    let mut mz = mzlab_core::mzmlhead::Mzml::new(std::fs::File::open(line).unwrap(), size);
    let chroms = mz.chromatograms(routes.idx.scans.last().map_or(0, |s| s.end)).unwrap();
    let name = r["file"].as_str().map_or_else(|| PathBuf::from(line).file_name().unwrap().to_string_lossy().to_string(), String::from);
    let num = |k: &str| q[k].as_str().unwrap().parse::<i64>().unwrap();
    let res = match path {
        "/api/method" => mzlab_core::itemroutes::item_method(&routes.idx, &mut mz, &name).and_then(|d| mzlab_core::labmethods::method_json(&d, &[], &name)),
        "/api/delay" => routes.delay(&mut mz, &chroms),
        "/api/analog" => mzlab_core::itemroutes::analog(&mut mz, &chroms, num("i"), q.get("n").map_or(2000, |_| num("n"))),
        _ => mzlab_core::itemroutes::pda_trace(&mut mz, &chroms),
    };
    match res {
        Ok(t) => Some(t),
        Err(e) => {
            assert!(r["status"] != 200, "{what}: {e}");
            None
        }
    }
}
