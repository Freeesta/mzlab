//! `components::estimate_components` against `ionfamily.estimate_components` (`tools/componenti_riferimento.py`): the number of components and
//! the scans of the evolving factor analysis are identical, the singular values, sigma and edge agree to 1e-9 (relative).
//! Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::Path;
use std::process::Command;

use mzlab_core::components::estimate_components;
use serde_json::Value;

fn nums(v: &Value) -> Vec<f64> {
    v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect()
}

fn near(got: f64, want: f64, scale: f64) -> bool {
    (got - want).abs() <= 1e-9 * scale.max(1e-300)
}

#[test]
fn components_like_python() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = std::env::temp_dir().join(format!("mzlab-components-{}.json", std::process::id()));
    let run = Command::new("python3").current_dir(&root).arg("tools/componenti_riferimento.py").arg(&out).output();
    if !matches!(&run, Ok(o) if o.status.success()) {
        assert!(std::env::var("MZLAB_GOLDEN_STRICT").is_err(), "the reference did not run: {run:?}");
        eprintln!("python3/numpy not available: test skipped");
        return;
    }
    let cases: Vec<Value> = serde_json::from_slice(&std::fs::read(&out).unwrap()).unwrap();
    let mut ks = std::collections::BTreeSet::new();
    for c in &cases {
        let (n, m) = (c["n"].as_u64().unwrap() as usize, c["m"].as_u64().unwrap() as usize);
        let (max_k, min_var) = (c["max_k"].as_u64().unwrap() as usize, c["min_var"].as_f64().unwrap());
        let (got, want) = (estimate_components(&nums(&c["d"]), n, m, max_k, min_var), &c["result"]);
        let tag = format!("{n}x{m} max_k {max_k}");
        assert_eq!(got.k as u64, want["k"].as_u64().unwrap(), "{tag}: k");
        let efa: Vec<usize> = want["efa_start"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as usize).collect();
        assert_eq!(got.efa_start, efa, "{tag}: efa_start");
        let sv = nums(&want["sv"]);
        assert_eq!(got.sv.len(), sv.len(), "{tag}: sv");
        for (g, w) in got.sv.iter().zip(&sv) {
            assert!(near(*g, *w, sv[0]), "{tag}: sv {g} vs {w}");
        }
        match want.get("sigma") {
            Some(s) => {
                assert!(near(got.sigma.unwrap(), s.as_f64().unwrap(), s.as_f64().unwrap()), "{tag}: sigma");
                assert!(near(got.edge.unwrap(), want["edge"].as_f64().unwrap(), want["edge"].as_f64().unwrap()), "{tag}: edge");
            }
            None => assert!(got.sigma.is_none() && got.edge.is_none(), "{tag}: too small"),
        }
        ks.insert((got.k, got.efa_start.len()));
    }
    assert!(ks.len() >= 4, "the cases must cover several answers: {ks:?}");
}
