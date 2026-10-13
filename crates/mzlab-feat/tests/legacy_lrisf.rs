//! The in-source-fragment verdict of the low-resolution rows (`legacy/lrisf/`) against the Python answers (`tools/finder_lrisf_riferimento.py --golden`):
//! the inputs are the reports of `origin_report`, numbers within 1e-9, labels, `doubtful` and texts exactly equal.

use mzlab_core::json::{parse, Value as V};
use mzlab_feat::legacy::lrisf::{classify_report, Ramp};
use serde_json::Value;

fn golden() -> Value {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/finder_lrisf.json");
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

fn same(a: &Value, b: &Value, what: &str) {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            assert!(
                (x - y).abs() <= 1e-9 * y.abs().max(1.0),
                "{what}: {x} vs {y}"
            );
        }
        (Value::Object(x), Value::Object(y)) => {
            let (mut kx, mut ky): (Vec<_>, Vec<_>) = (x.keys().collect(), y.keys().collect());
            kx.sort();
            ky.sort();
            assert_eq!(kx, ky, "{what}: keys");
            x.iter()
                .for_each(|(k, v)| same(v, &y[k], &format!("{what}.{k}")));
        }
        (Value::Array(x), Value::Array(y)) => {
            assert_eq!(x.len(), y.len(), "{what}: length");
            x.iter()
                .zip(y)
                .enumerate()
                .for_each(|(i, (p, q))| same(p, q, &format!("{what}[{i}]")));
        }
        _ => assert_eq!(a, b, "{what}"),
    }
}

fn floats(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

#[test]
fn verdict_matches_python() {
    let g = golden();
    let cases = g["cases"].as_array().unwrap();
    assert!(cases.len() >= 40);
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let rep = parse(c["rep"].to_string().as_bytes()).unwrap();
        let formula = c["formula_p"].as_str();
        let ramp = c["ramp"]
            .as_object()
            .map(|r| (floats(&r["dp"]), floats(&r["f"]), floats(&r["p"])));
        let ramp = ramp.as_ref().map(|(dp, f, p)| Ramp { dp, f, p });
        let got = classify_report(&rep, formula, ramp.as_ref());
        match (&got, &c["expect"]) {
            (None, Value::Null) => {}
            (Some(v), e) if !e.is_null() => {
                let j: Value = serde_json::from_str(&mzlab_core::jsonb::write(v)).unwrap();
                same(&j, e, name);
            }
            (g, e) => panic!("{name}: got {:?}, expected {e}", g.is_some()),
        }
    }
}

#[test]
fn damaged_reports_do_not_panic() {
    for s in [
        "{}",
        "null",
        "{\"mz\":1}",
        "{\"mz\":1,\"parent_mz\":\"x\"}",
        "[]",
    ] {
        let rep: V = parse(s.as_bytes()).unwrap();
        assert!(classify_report(&rep, None, None).is_none());
    }
}
