//! The in-source-fragment verdict of the low-resolution rows (`legacy/lrisf/`) against the Python answers (`tools/finder_lrisf_riferimento.py --golden`):
//! the inputs are the reports of `origin_report`, numbers within 1e-9, labels, `doubtful` and texts exactly equal.

use mzlab_core::json::{parse, Value as V};
use mzlab_feat::legacy::lrisf::{
    classify_report, compare_lr, lr_tree, resolve_smiles, ParentMs2, Ramp,
};
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

#[test]
fn supposed_tree_matches_python() {
    let g = golden();
    let cases = g["tree"].as_array().unwrap();
    assert!(cases.len() >= 20);
    let mut with_smiles = 0;
    for c in cases {
        let (name, i) = (c["name"].as_str().unwrap(), &c["in"]);
        let (mz, rel) = (floats(&i["mz"]), floats(&i["rel"]));
        let ms2 = ParentMs2 {
            mz: &mz,
            rel: &rel,
            scans: i["scans"].as_u64().unwrap() as usize,
            precursor: i["precursor"].as_f64(),
        };
        let smiles = resolve_smiles(i["smiles_in"].as_str(), i["neutral_text"].as_str());
        assert_eq!(smiles.as_deref(), i["smiles"].as_str(), "{name}: smiles");
        let got = lr_tree(
            Some(&ms2),
            i["mz_x"].as_f64().unwrap(),
            i["ion"].as_str(),
            smiles.as_deref(),
        );
        let got: Value = serde_json::from_str(&got).unwrap();
        with_smiles += got["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| !n["smiles"].as_array().unwrap().is_empty())
            .count();
        same(&got, &c["out"], name);
    }
    assert!(with_smiles > 0);
}

#[test]
fn tree_without_ms2_is_the_note() {
    let none: Value =
        serde_json::from_str(&lr_tree(None, 300.3, Some("C10H12N2O3S"), None)).unwrap();
    assert_eq!(none["ok"], false);
    assert_eq!(none["note"], "no MS2 of the parent in the files");
    let empty = ParentMs2 {
        mz: &[],
        rel: &[],
        scans: 3,
        precursor: None,
    };
    assert_eq!(
        lr_tree(Some(&empty), 300.3, None, None),
        lr_tree(None, 300.3, None, None)
    );
}

#[test]
fn comparison_matches_python() {
    let g = golden();
    let cases = g["compare"].as_array().unwrap();
    assert!(cases.len() >= 20);
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let tree = parse(c["tree"].to_string().as_bytes()).unwrap();
        let got = compare_lr(&tree, &floats(&c["tp_mz"]), c["dm"].as_f64());
        let got: Value = serde_json::from_str(&got).unwrap();
        same(&got, &c["out"], name);
    }
}
