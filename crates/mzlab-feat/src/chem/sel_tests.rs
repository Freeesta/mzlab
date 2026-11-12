//! Unit tests of `sel/1` (`sel.rs`): Kekulé and aromatic-typed benzene, fused and saturated rings, charge, radical and pieces, request errors.

use super::*;

/// V2000 of atoms `els` with bonds `(a, b, type)` (1-based) and property lines.
fn mf(els: &[&str], bonds: &[(usize, usize, u8)], props: &[&str]) -> String {
    let mut s = format!(
        "\n  t\n\n{:>3}{:>3}  0  0  0  0  0  0  0  0999 V2000\n",
        els.len(),
        bonds.len()
    );
    for (i, e) in els.iter().enumerate() {
        s += &format!(
            "{:>10.4}{:>10.4}{:>10.4} {e:<3} 0  0  0  0  0  0  0  0  0  0  0  0\n",
            i as f64, 0.0, 0.0
        );
    }
    for (a, b, o) in bonds {
        s += &format!("{a:>3}{b:>3}{o:>3}  0  0  0  0\n");
    }
    props.iter().for_each(|p| s += &format!("{p}\n"));
    s + "M  END"
}
fn sel(mf: &str, part: bool) -> serde_json::Value {
    let req = serde_json::json!({ "molfile": mf, "part": part }).to_string();
    serde_json::from_str(&sel_json(&req).unwrap()).unwrap()
}
const BENZENE: [(usize, usize, u8); 6] = [
    (1, 2, 2),
    (2, 3, 1),
    (3, 4, 2),
    (4, 5, 1),
    (5, 6, 2),
    (6, 1, 1),
];

#[test]
fn kekule_benzene_is_one_aromatic_ring() {
    let v = sel(&mf(&["C"; 6], &BENZENE, &[]), false);
    assert_eq!(v["format"], "sel/1");
    assert_eq!(v["label"]["formula"], "C6H6");
    assert!((v["label"]["mass"].as_f64().unwrap() - 78.04695).abs() < 1e-5);
    assert!((v["average"].as_f64().unwrap() - 78.114).abs() < 1e-3);
    assert_eq!(
        (
            v["heavy"].as_i64(),
            v["rings"].as_i64(),
            v["aromaticRings"].as_i64(),
            v["aromaticAtoms"].as_i64()
        ),
        (Some(6), Some(1), Some(1), Some(6))
    );
    assert!(v["label"]["mz"].is_null() && v["part"] == false);
}

#[test]
fn benzene_written_aromatic_counts_the_same() {
    let b: Vec<_> = BENZENE.iter().map(|&(a, c, _)| (a, c, 4)).collect();
    let v = sel(&mf(&["C"; 6], &b, &[]), false);
    assert_eq!(
        (
            v["label"]["formula"].as_str(),
            v["rings"].as_i64(),
            v["aromaticRings"].as_i64(),
            v["aromaticAtoms"].as_i64()
        ),
        (Some("C6H6"), Some(1), Some(1), Some(6)),
        "{v}"
    );
}

#[test]
fn naphthalene_two_rings_cyclohexane_none_aromatic() {
    let mut b = BENZENE.to_vec();
    b[5] = (6, 1, 1);
    b.extend([(1, 7, 1), (7, 8, 2), (8, 9, 1), (9, 10, 2), (10, 6, 1)]);
    b[0] = (1, 2, 2);
    let v = sel(&mf(&["C"; 10], &b, &[]), false);
    assert_eq!(
        (
            v["label"]["formula"].as_str(),
            v["rings"].as_i64(),
            v["aromaticRings"].as_i64()
        ),
        (Some("C10H8"), Some(2), Some(2)),
        "{v}"
    );
    let hex: Vec<_> = (1..=6).map(|i| (i, i % 6 + 1, 1)).collect();
    let v = sel(&mf(&["C"; 6], &hex, &[]), true);
    assert_eq!(
        (
            v["label"]["formula"].as_str(),
            v["rings"].as_i64(),
            v["aromaticRings"].as_i64(),
            v["part"].as_bool()
        ),
        (Some("C6H12"), Some(1), Some(0), Some(true))
    );
}

#[test]
fn charge_radical_and_pieces() {
    // acetate: CC(=O)[O-]
    let v = sel(
        &mf(
            &["C", "C", "O", "O"],
            &[(1, 2, 1), (2, 3, 2), (2, 4, 1)],
            &["M  CHG  1   4  -1"],
        ),
        false,
    );
    assert_eq!(
        (v["label"]["text"].as_str(), v["label"]["charge"].as_i64()),
        (Some("C2H3O2-"), Some(-1))
    );
    assert!(
        (v["label"]["mz"].as_f64().unwrap() - 59.01385).abs() < 1e-4,
        "{v}"
    );
    // methyl radical and a separate methane: two pieces, one unpaired electron
    let v = sel(&mf(&["C", "C"], &[], &["M  RAD  1   1   2"]), true);
    assert_eq!(
        (
            v["label"]["formula"].as_str(),
            v["pieces"].as_i64(),
            v["label"]["unpaired"].as_i64()
        ),
        (Some("C2H7"), Some(2), Some(1)),
        "{v}"
    );
}

#[test]
fn errors_keep_their_key() {
    assert_eq!(sel_json("nope").unwrap_err().key, "error.sel.request");
    assert_eq!(
        sel_json("{\"molfile\":\"x\"}").unwrap_err().key,
        "error.grafo.molfile"
    );
    assert_eq!(sel_json("{}").unwrap_err().key, "error.sel.request");
}
