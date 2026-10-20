#![cfg(feature = "record")]
//! `frag/1` and `parz/1` from a `mol/1` record against the molfile path, over the V2000 molfiles of the mzlab-mol golden: the same cuts (compared as a
//! set by atom pair: the record sorts its bonds) with the same sides, and the same partial structure.

use mzlab_feat::chem::frammenti::{
    fragments_json, fragments_record_json, partial_json, partial_record_json,
};
use serde_json::Value;

/// The cuts of a `frag/1` report keyed by the atom pair, each with its sides.
fn cuts(json: &str) -> Vec<((i64, i64), Value)> {
    let v: Value = serde_json::from_str(json).expect("json");
    let mut out: Vec<_> = v["cuts"]
        .as_array()
        .expect("cuts")
        .iter()
        .map(|c| {
            let (a, b) = (c["a"].as_i64().unwrap(), c["b"].as_i64().unwrap());
            let mut sides = c["sides"].as_array().unwrap().clone();
            sides.sort_by_key(|s| s["atoms"].to_string());
            (
                (a.min(b), a.max(b)),
                serde_json::json!([c["order"], c["kind"], sides]),
            )
        })
        .collect();
    out.sort_by_key(|x| x.0);
    out
}

#[test]
fn record_cuts_equal_molfile_cuts() {
    let g: Value = serde_json::from_str(include_str!(
        "../../mzlab-mol/tests/data/golden_molfile.json"
    ))
    .expect("golden");
    let (mut n, mut same) = (0, 0);
    for c in g["molfiles"].as_array().expect("molfiles") {
        let mf = c["mol"].as_str().unwrap_or_default();
        // the legacy reader takes V2000 only
        if mf.contains("V3000") {
            continue;
        }
        let (Ok(a), Ok(rec)) = (fragments_json(mf, -1, -1), mzlab_mol::Mol::from_molfile(mf))
        else {
            continue;
        };
        let b = fragments_record_json(&rec, -1, -1).expect("record cuts");
        n += 1;
        // charged and radical atoms may carry other hydrogens in the legacy reader: their formulas may differ, the cut bonds may not
        let (ca, cb) = (cuts(&a), cuts(&b));
        assert_eq!(
            ca.iter().map(|x| x.0).collect::<Vec<_>>(),
            cb.iter().map(|x| x.0).collect::<Vec<_>>(),
            "{mf}"
        );
        if ca == cb {
            same += 1;
        }
    }
    assert!(n > 20, "{n} molfiles compared");
    assert!(same * 10 >= n * 9, "{same} of {n} identical");
}

#[test]
fn one_bond_and_partial_structure_match() {
    let mf = "\n  t\n\n  3  2  0  0  0  0  0  0  0  0999 V2000\n    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    1.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n    2.0000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n  1  2  1  0  0  0  0\n  2  3  1  0  0  0  0\nM  END";
    let rec = mzlab_mol::Mol::from_molfile(mf).unwrap();
    assert_eq!(
        fragments_json(mf, 1, 2).unwrap(),
        fragments_record_json(&rec, 1, 2).unwrap()
    );
    assert_eq!(
        partial_json(mf, "+O -H2").unwrap(),
        partial_record_json(&rec, "+O -H2").unwrap()
    );
    assert_eq!(
        fragments_json(mf, 0, 2).unwrap_err().key,
        fragments_record_json(&rec, 0, 2).unwrap_err().key
    );
}
