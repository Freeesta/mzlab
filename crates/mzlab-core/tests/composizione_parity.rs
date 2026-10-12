//! `composition::compose` and `parse_elements` answer like `mzlab/chem/composition.py` (`tools/composizione_riferimento.py`), without observed
//! isotope peaks or MS2 peaks: the same candidates in the same order, numbers within 1e-9, same error keys. Skipped without python3 and numpy
//! (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::composition::{compose, parse_elements, Opts};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn close(a: f64, b: f64) -> bool {
    a == b || (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
}
fn same(path: &str, got: &Value, want: &Value) {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => assert!(
            close(a.as_f64().unwrap(), b.as_f64().unwrap()),
            "{path}: {a} against {b}"
        ),
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len(), "{path}: length");
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                same(&format!("{path}[{i}]"), x, y);
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            assert_eq!(
                a.keys().collect::<Vec<_>>(),
                b.keys().collect::<Vec<_>>(),
                "{path}: keys"
            );
            for (k, y) in b {
                same(&format!("{path}.{k}"), &a[k], y);
            }
        }
        (a, b) => assert_eq!(a, b, "{path}"),
    }
}
fn err_matches(e: &mzlab_core::Error, r: &Value, what: &str) {
    let want = r["err"].as_str().unwrap();
    assert!(
        e.key == want || want.starts_with("python:"),
        "{what}: {} against {want}",
        e.key
    );
}

#[test]
fn composition_like_python() {
    let golden = root().join("tests/golden/ref/composizione.json");
    let committed = golden.exists();           // the answers of Python for the synthetic data are committed: no python3 needed
    let out = if committed { golden } else { std::env::temp_dir().join(format!("mzlab_comp_{}.json", std::process::id())) };
    let ok = committed || Command::new("python3")
        .arg(root().join("tools/composizione_riferimento.py"))
        .arg(&out)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        eprintln!("skipped: python3/numpy not available");
        return;
    }
    let rows: Vec<Value> = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    if !committed { let _ = std::fs::remove_file(&out); }
    let (mut n_ok, mut n_err, mut n_res) = (0, 0, 0);
    for (i, r) in rows.iter().enumerate() {
        if r["kind"] == "parse" {
            let what = format!("parse {:?}", r["text"]);
            match (parse_elements(r["text"].as_str().unwrap()), r.get("ok")) {
                (Ok(g), Some(w)) => {
                    let g: Value = g
                        .into_iter()
                        .map(|(s, (a, b))| (s, serde_json::json!([a, b])))
                        .collect::<serde_json::Map<_, _>>()
                        .into();
                    same(&what, &g, w);
                    // the order of the elements matters (the grid runs in it)
                    assert_eq!(
                        g.as_object().unwrap().len(),
                        w.as_object().unwrap().len(),
                        "{what}"
                    );
                }
                (Err(e), None) => err_matches(&e, r, &what),
                (g, w) => panic!("{what}: {g:?} against {w:?}"),
            }
            continue;
        }
        let a = r["args"].as_object().unwrap();
        let mut o = Opts::default();
        if let Some(e) = a.get("elements") {
            o.elements = Some(
                e.as_object()
                    .unwrap()
                    .iter()
                    .map(|(s, v)| (s.clone(), (v[0].as_i64().unwrap(), v[1].as_i64().unwrap())))
                    .collect(),
            );
        }
        if let Some(v) = a.get("ion") {
            o.ion = v.as_str().unwrap();
        }
        if let Some(v) = a.get("z") {
            o.z = Some(v.as_i64().unwrap() as i32);
        }
        if let Some(v) = a.get("tol") {
            o.tol = v.as_f64().unwrap();
        }
        if let Some(v) = a.get("unit") {
            o.unit = v.as_str().unwrap();
        }
        if let Some(v) = a.get("rdb") {
            o.rdb = (v[0].as_f64().unwrap(), v[1].as_f64().unwrap());
        }
        if let Some(v) = a.get("nitrogen") {
            o.nitrogen = v.as_str().unwrap();
        }
        if let Some(v) = a.get("rules") {
            o.rules = v
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().unwrap().to_string())
                .collect();
        }
        if let Some(v) = a.get("max_results") {
            o.max_results = v.as_u64().unwrap() as usize;
        }
        if let Some(v) = a.get("parent") {
            o.parent = v.as_str();
        }
        let pairs = |k: &str| -> Vec<(f64, f64)> {
            a.get(k).map_or(vec![], |v| {
                v.as_array()
                    .unwrap()
                    .iter()
                    .map(|p| (p[0].as_f64().unwrap(), p[1].as_f64().unwrap()))
                    .collect()
            })
        };
        o.obs = pairs("obs");
        o.frags = pairs("frags");
        if let Some(v) = a.get("r200") {
            o.r200 = v.as_f64().unwrap();
        }
        if let Some(v) = a.get("sigma_ppm") {
            o.sigma_ppm = v.as_f64().unwrap();
        }
        if let Some(v) = a.get("sigma_int") {
            o.sigma_int = v.as_f64().unwrap();
        }
        if let Some(v) = a.get("iso_min_rel") {
            o.iso_min_rel = v.as_f64().unwrap();
        }
        if let Some(v) = a.get("iso_noise_rel") {
            o.iso_noise_rel = v.as_f64().unwrap();
        }
        if let Some(v) = a.get("frag_ppm") {
            o.frag_ppm = v.as_f64();
        }
        if let Some(v) = a.get("pool") {
            o.pool = v.as_u64().unwrap() as usize;
        }
        if let Some(v) = a.get("norm") {
            o.norm = v.as_str().unwrap();
        }
        if let Some(v) = a.get("weights") {
            o.weights = (
                v[0].as_f64().unwrap(),
                v[1].as_f64().unwrap(),
                v[2].as_f64().unwrap(),
            );
        }
        o.m1 = a.get("m1").and_then(Value::as_f64);
        o.m2 = a.get("m2").and_then(Value::as_f64);
        let mz = r["mz"].as_f64().unwrap();
        let what = format!("#{i} compose {mz} {a:?}");
        match (compose(mz, &o), r.get("ok")) {
            (Ok(g), Some(w)) => {
                let g: Value = serde_json::from_str(&g).unwrap();
                same(&what, &g, w);
                n_ok += 1;
                n_res += w["results"].as_array().unwrap().len();
            }
            (Err(e), None) => {
                err_matches(&e, r, &what);
                n_err += 1;
            }
            (g, w) => panic!("{what}: {g:?} against {w:?}"),
        }
    }
    assert!(
        n_ok > 100 && n_err >= 4 && n_res > 500,
        "{n_ok} answers, {n_err} errors, {n_res} candidates"
    );
}
