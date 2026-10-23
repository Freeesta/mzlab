//! `ref=` on `api/chrom`, `api/xic` and `api/mrm` in Rust give the JSON of Python (`tools/differenza_riferimento.py`): same keys and
//! lengths, numbers within 1e-9 (relative), the errors of the user's setting with the same key and parameters, a request Python must word
//! itself declined. Without python3 and numpy the test is skipped (`MZLAB_GOLDEN_STRICT=1` fails instead).

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::diffroute::{mrm_diff, plan};
use mzlab_core::idxread::Index;
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

fn transitions(mzml: &str, routes: &Routes<std::fs::File>) -> String {
    let size = std::fs::metadata(mzml).unwrap().len();
    let mut m = mzlab_core::mzmlhead::Mzml::new(std::fs::File::open(mzml).unwrap(), size);
    let end = routes.idx.scans.last().map_or(0, |s| s.end);
    let chroms = m.chromatograms(end).unwrap();
    mzlab_core::fileroutes::mrm_transitions(&mut m, &chroms).unwrap()
}

#[test]
fn reference_difference_matches_python() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = std::env::temp_dir().join(format!("mzlab-diff-{}", std::process::id()));
    let listing = match Command::new("python3").current_dir(&root).arg("tools/differenza_riferimento.py").arg(&out).output() {
        Ok(o) if o.status.success() => String::from_utf8(o.stdout).unwrap(),
        other => {
            if std::env::var("MZLAB_GOLDEN_STRICT").is_ok() {
                panic!("the reference did not run: {other:?}");
            }
            eprintln!("python3/numpy not available: test skipped");
            return;
        }
    };
    let files: Vec<String> = listing.lines().filter(|l| !l.is_empty()).map(String::from).collect();
    let mut routes: Vec<Routes<std::fs::File>> = files
        .iter()
        .map(|f| {
            let p = out.join(format!("{}.mzidx", PathBuf::from(f).file_stem().unwrap().to_string_lossy()));
            Routes::new(Index::open(std::fs::File::open(&p).unwrap(), std::fs::metadata(&p).unwrap().len()).unwrap())
        })
        .collect();
    let rows: Vec<Value> = serde_json::from_slice(&std::fs::read(out.join("diff.json")).unwrap()).unwrap();
    let (mut compared, mut declined, mut failed) = (0, 0, 0);
    for r in &rows {
        let path = r["path"].as_str().unwrap();
        let q = r["query"].as_object().unwrap();
        let qs = q.iter().map(|(k, v)| format!("{k}={}", enc(v.as_str().unwrap()))).collect::<Vec<_>>().join("&");
        let what = format!("{path} {qs}");
        let query = Query::parse(&qs);
        let answer: Option<Result<String, String>> = if path == "/api/mrm" {
            let ks: Vec<usize> = q["k"].as_str().unwrap().split(',').filter_map(|x| x.parse().ok()).collect();
            let parts: Vec<String> = ks.iter().map(|&k| format!("{{\"k\":{k},\"transitions\":{}}}", transitions(&files[k], &routes[k]))).collect();
            let rf: usize = q["ref"].as_str().unwrap().parse().unwrap_or(0);   // a ref that is not a number: the options decline it
            mrm_diff(&format!("[{}]", parts.join(",")), &transitions(&files[rf], &routes[rf]), &query)
        } else {
            match plan(path, &query) {
                None => None,
                Some(Err(e)) => Some(Err(e)),
                Some(Ok(p)) => {
                    let reference = p.fetch(p.o.reference, &mut routes[p.o.reference as usize]).unwrap().unwrap();
                    let fetched: Vec<_> = p.ks.iter().map(|&k| p.fetch(k, &mut routes[k as usize]).unwrap().unwrap()).collect();
                    Some(Ok(p.finish(&reference, &fetched)))
                }
            }
        };
        let status = r["status"].as_u64().unwrap();
        match answer {
            None => {
                assert!(status != 200, "{what}: declined, but Python answers it");
                declined += 1;
            }
            Some(Err(e)) => {
                assert_eq!(status, 400, "{what}");
                same("", &serde_json::from_str(&e).unwrap(), &r["body"], &what);
                failed += 1;
            }
            Some(Ok(j)) => {
                assert_eq!(status, 200, "{what}: Rust answers, Python fails");
                same("", &serde_json::from_str(&j).unwrap(), &r["body"], &what);
                compared += 1;
            }
        }
    }
    assert!(compared >= 140 && declined >= 10 && failed >= 15, "{compared} answers, {declined} declined, {failed} errors");
    let _ = std::fs::remove_dir_all(&out);
}
