//! `origin::origin_report` gives what `ionfamily.origin_report` gives (`tools/origin_riferimento.py`): the same keys, the same indices
//! and counts, numbers within 1e-6 relative. Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_core::jsonb::write;
use mzlab_core::origin::{origin_report, Ms2, Request, Sample};
use mzlab_core::originroute::{origin_route, Ask, ItemMeta};
use mzlab_core::routes::Routes;
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Compares two reports leaf by leaf; returns the number of numbers compared.
fn compare(path: &str, got: &Value, want: &Value, bad: &mut Vec<String>) -> usize {
    let mut n = 0;
    match (got, want) {
        (Value::Object(g), Value::Object(w)) => {
            for k in w.keys() {
                match g.get(k) {
                    Some(v) => n += compare(&format!("{path}.{k}"), v, &w[k], bad),
                    None => bad.push(format!("{path}.{k}: missing")),
                }
            }
            for k in g.keys().filter(|k| !w.contains_key(*k)) {
                bad.push(format!("{path}.{k}: extra"));
            }
        }
        (Value::Array(g), Value::Array(w)) => {
            if g.len() != w.len() {
                bad.push(format!("{path}: length {} against {}", g.len(), w.len()));
            } else {
                for (i, (a, b)) in g.iter().zip(w).enumerate() {
                    n += compare(&format!("{path}[{i}]"), a, b, bad);
                }
            }
        }
        (Value::Number(g), Value::Number(w)) => {
            n += 1;
            let (a, b) = (g.as_f64().unwrap(), w.as_f64().unwrap());
            let ok = if w.is_i64() {
                a == b
            } else {
                a == b || (a - b).abs() <= 1e-6 * a.abs().max(b.abs()).max(1e-9)
            };
            if !ok {
                bad.push(format!("{path}: {a} against {b}"));
            }
        }
        (g, w) => {
            if g != w {
                bad.push(format!("{path}: {g} against {w}"));
            }
        }
    }
    n
}

fn spec(v: &Value) -> (Vec<f64>, Vec<f64>) {
    let f = |k: &str| {
        v[k].as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect()
    };
    (f("mz"), f("int"))
}

/// The same cases through the route of the session (`origin_route`: the files of the session by slot, the report plus `hr` and `pol`).
fn via_route(cases: &[Value], out: &Path, n_files: usize, failures: &mut Vec<String>) {
    let mut routes: std::collections::HashMap<u32, Routes<std::fs::File>> = (0..n_files)
        .map(|i| {
            let p = out.join(format!("s{i}.mzidx"));
            let idx = Index::open(
                std::fs::File::open(&p).unwrap(),
                std::fs::metadata(&p).unwrap().len(),
            )
            .unwrap();
            (i as u32, Routes::new(idx))
        })
        .collect();
    let mut via_route = 0;
    for c in cases.iter().filter(|c| c["params"]["ms2"].is_null()) {
        let pa = &c["params"];
        let items: Vec<ItemMeta> = (0..n_files)
            .map(|i| ItemMeta {
                slot: i as u32,
                label: pa["labels"][i].as_str().unwrap().to_string(),
                time: pa["times"][i].as_f64(),
                full: true,
            })
            .collect();
        let ask = Ask {
            k: pa["ref"].as_i64().unwrap_or(-1),
            mz: pa["mz"].as_f64().unwrap(),
            parent: pa["parent"].as_f64().unwrap(),
            rt0: pa["rt0"].as_f64(),
            rt1: pa["rt1"].as_f64(),
            formula: pa["formula"].as_str(),
        };
        let rep = origin_route(&items, &mut routes, &ask)
            .unwrap()
            .expect("unit-resolution centroids are answered by Rust");
        let mut got: Value = serde_json::from_str(&write(&rep)).unwrap();
        let g = got.as_object_mut().unwrap();
        assert_eq!(g.remove("hr"), Some(Value::Bool(false)));
        assert!(g.remove("pol").is_some());
        g.remove("timing");
        let mut want = c["result"].clone();
        want.as_object_mut().unwrap().remove("timing");
        let mut bad = Vec::new();
        compare("", &got, &want, &mut bad);
        failures.extend(
            bad.iter()
                .take(10)
                .map(|b| format!("route mz {}: {b}", pa["mz"])),
        );
        via_route += 1;
    }
    assert!(
        via_route > 0,
        "no case without MS2 to run through the route"
    );
}

#[test]
fn rust_origin_matches_python() {
    // MZLAB_ORIGIN_DIR: a folder already written by the reference script (e.g. from real files), to compare without regenerating
    let ready = std::env::var("MZLAB_ORIGIN_DIR").ok().map(PathBuf::from);
    let out = ready.clone().unwrap_or_else(|| {
        std::env::temp_dir().join(format!("mzlab-origin-{}", std::process::id()))
    });
    let run = match ready {
        Some(_) => Command::new("true").output(),
        None => Command::new("python3")
            .current_dir(root())
            .arg("tools/origin_riferimento.py")
            .arg(&out)
            .output(),
    };
    if !matches!(&run, Ok(o) if o.status.success()) {
        if std::env::var("MZLAB_GOLDEN_STRICT").is_ok() {
            panic!("the reference did not run: {run:?}");
        }
        eprintln!("python3/numpy not available: test skipped");
        return;
    }
    let cases: Vec<Value> =
        serde_json::from_slice(&std::fs::read(out.join("origin.json")).unwrap()).unwrap();
    let n_files = cases[0]["params"]["labels"].as_array().unwrap().len();
    let mut idxs: Vec<Index<std::fs::File>> = (0..n_files)
        .map(|i| {
            let p = out.join(format!("s{i}.mzidx"));
            Index::open(
                std::fs::File::open(&p).unwrap(),
                std::fs::metadata(&p).unwrap().len(),
            )
            .unwrap()
        })
        .collect();
    let mut failures = Vec::new();
    let mut compared = 0;
    for c in &cases {
        let pa = &c["params"];
        let si: Vec<usize> = idxs
            .iter()
            .map(|i| i.section_of(1, None).unwrap())
            .collect();
        let labels = pa["labels"].as_array().unwrap();
        let times = pa["times"].as_array().unwrap();
        let mut samples: Vec<Sample<std::fs::File>> = idxs
            .iter_mut()
            .enumerate()
            .map(|(k, idx)| Sample {
                label: labels[k].as_str().unwrap().to_string(),
                time: times[k].as_f64(),
                idx,
                si: si[k],
            })
            .collect();
        let ms2 = match pa["ms2"].as_str() {
            Some(kind) => Some(Ms2 {
                x: (kind == "xp").then(|| spec(&c["ms2_x"])),
                p: spec(&c["ms2_p"]),
            }),
            None => None,
        };
        let formula = pa["formula"].as_str();
        let req = Request {
            mz: pa["mz"].as_f64().unwrap(),
            parent_mz: pa["parent"].as_f64().unwrap(),
            rt0: pa["rt0"].as_f64(),
            rt1: pa["rt1"].as_f64(),
            reference: pa["ref"].as_u64().map(|v| v as usize),
            formula_p: formula,
            top: 25,
        };
        let t = std::time::Instant::now();
        let got = origin_report(&mut samples, ms2.as_ref(), &req).unwrap();
        eprintln!(
            "origin mz {}: rust {:.1} ms, python {:.1} ms",
            pa["mz"],
            t.elapsed().as_secs_f64() * 1e3,
            c["py_s"].as_f64().unwrap_or(f64::NAN) * 1e3
        );
        let got: Value = serde_json::from_str(&write(&got)).unwrap();
        let mut bad = Vec::new();
        let mut want = c["result"].clone();
        want.as_object_mut().unwrap().remove("timing");
        let mut got = got;
        got.as_object_mut().unwrap().remove("timing");
        compared += compare("", &got, &want, &mut bad);
        for b in bad.iter().take(25) {
            failures.push(format!("mz {} ref {}: {b}", pa["mz"], pa["ref"]));
        }
    }
    via_route(&cases, &out, n_files, &mut failures);
    assert!(compared > 1000, "only {compared} numbers compared");
    assert!(
        failures.is_empty(),
        "{} differences:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
