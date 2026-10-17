//! The parent-product MS2 pairs against the frozen answers of the former Python engine (`tests/golden/feat/tp_ms2.json`): the three components of
//! the similarity (equal, shifted, common losses) of `legacy::similarity::pair_components`, with their counts and typed pairs, numbers within 1e-9.
//! The network of mass differences on the same file is checked inside the crate (`legacy/network/golden.rs`).

use mzlab_feat::legacy::similarity;
use serde_json::Value;

fn golden() -> Value {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/feat/tp_ms2.json");
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

fn floats(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

fn close(a: f64, b: f64, what: &str) {
    assert!(
        (a - b).abs() <= 1e-9 * b.abs().max(1.0),
        "{what}: {a} vs {b}"
    );
}

#[test]
fn components_match_the_frozen_answers() {
    const TYPES: [&str; 3] = ["equal", "shifted", "loss"];
    let g = golden();
    let cases = g["components"].as_array().unwrap();
    assert!(cases.len() >= 10);
    for (i, c) in cases.iter().enumerate() {
        let (a, b) = (&c["a"], &c["b"]);
        let (pa, pb) = (a[2].as_f64().unwrap(), b[2].as_f64().unwrap());
        let sa = similarity::prepare(&floats(&a[0]), &floats(&a[1]), pa);
        let sb = similarity::prepare(&floats(&b[0]), &floats(&b[1]), pb);
        let r = similarity::pair_components(&sa, pa, &sb, pb);
        close(
            r.delta,
            c["delta"].as_f64().unwrap(),
            &format!("case {i} delta"),
        );
        for k in 0..3 {
            close(
                r.cos[k],
                c["cos"][k].as_f64().unwrap(),
                &format!("case {i} cos {}", TYPES[k]),
            );
            close(
                r.ent[k],
                c["ent"][k].as_f64().unwrap(),
                &format!("case {i} ent {}", TYPES[k]),
            );
            assert_eq!(
                r.n[k] as u64,
                c["n"][k].as_u64().unwrap(),
                "case {i} n {}",
                TYPES[k]
            );
        }
        let want = c["pairs"].as_array().unwrap();
        assert_eq!(r.pairs.len(), want.len(), "case {i} pairs");
        for (p, w) in r.pairs.iter().zip(want) {
            close(p.0, w[0].as_f64().unwrap(), &format!("case {i} pair a"));
            close(p.1, w[1].as_f64().unwrap(), &format!("case {i} pair b"));
            assert_eq!(TYPES[p.2], w[2].as_str().unwrap(), "case {i} pair type");
        }
    }
}
