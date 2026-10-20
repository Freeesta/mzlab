//! Helpers shared by the integration tests: the golden data and the comparison of a record with RDKit's.

use mzlab_mol::Mol;
use serde_json::Value;
use std::collections::BTreeMap;

pub fn golden() -> Value {
    serde_json::from_str(include_str!("../data/golden.json")).expect("golden.json")
}

/// The record as JSON, restricted to what the reader asserts: stereo elements that are `defined`.
pub fn defined(rec: &Value) -> Vec<Value> {
    rec["stereo"]
        .as_array()
        .expect("stereo")
        .iter()
        .filter(|s| s["status"] == "defined")
        .cloned()
        .collect()
}

fn bonds_of(rec: &Value) -> BTreeMap<(u64, u64), u64> {
    rec["bonds"]
        .as_array()
        .expect("bonds")
        .iter()
        .map(|b| {
            (
                (b["a"].as_u64().unwrap_or(0), b["b"].as_u64().unwrap_or(0)),
                b["o"].as_u64().unwrap_or(0),
            )
        })
        .collect()
}

/// Same bond pairs; the same orders, except between the aromatic bonds where more than one Kekulé form is right (then the per-atom sums must agree).
pub fn same_bonds(mine: &Value, truth: &Value, aromatic: &Value, what: &str) {
    let (a, b) = (bonds_of(mine), bonds_of(truth));
    assert_eq!(
        a.keys().collect::<Vec<_>>(),
        b.keys().collect::<Vec<_>>(),
        "{what}: bond pairs"
    );
    let arom: Vec<(u64, u64)> = aromatic
        .as_array()
        .expect("aromatic")
        .iter()
        .map(|p| (p[0].as_u64().unwrap_or(0), p[1].as_u64().unwrap_or(0)))
        .collect();
    for (k, o) in &a {
        assert!(
            *o == b[k] || arom.contains(k),
            "{what}: bond {k:?} order {o} against {}",
            b[k]
        );
    }
    let sums = |m: &BTreeMap<(u64, u64), u64>| {
        let mut s = BTreeMap::new();
        for (&(x, y), &o) in m {
            *s.entry(x).or_insert(0) += o;
            *s.entry(y).or_insert(0) += o;
        }
        s
    };
    assert_eq!(sums(&a), sums(&b), "{what}: bond order sum per atom");
}

pub fn groups_json(m: &Mol) -> Vec<Value> {
    let mut g: Vec<Value> = m
        .groups
        .iter()
        .map(|g| serde_json::to_value(g).expect("group"))
        .collect();
    g.sort_by_key(|v| v["atoms"].to_string());
    g
}
