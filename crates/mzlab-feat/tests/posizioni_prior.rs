#![cfg(feature = "chem")]
//! The `prior` of `posiz/1`: the f(−) of the parent (a `reatt/1`, here built from the tblite 0.7.0 ALPB-water values of `mzlab-chem/tests/data/cc_ref.json`)
//! on every site, ranked, and nothing else of the report changes (classes, statuses, proposal: the same JSON once `prior` is removed).

use mzlab_feat::chem::posizioni::{positions_json, positions_prior_json, HALOGEN_CAVEAT};
use serde_json::{json, Value};

fn reference(name: &str) -> Value {
    let p = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../mzlab-chem/tests/data/cc_ref.json"
    );
    let d: Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    d["molecules"][name].clone()
}

/// A `reatt/1` with the f(−) of the reference (heavy atoms first in `z`, then the H, as `Mol3d` numbers them).
fn reatt_of(name: &str) -> String {
    let m = reference(name);
    let z = m["z"].as_array().unwrap();
    let f = m["alpb"]["f_minus"].as_array().unwrap();
    let atoms: Vec<Value> = z
        .iter()
        .enumerate()
        .map(|(i, el)| {
            let heavy = el.as_u64() != Some(1);
            json!({"i": i, "heavy_index": if heavy { json!(i) } else { Value::Null }, "f_minus": f[i]})
        })
        .collect();
    json!({"format": "reatt", "version": 1, "method": "GFN2-xTB", "solvent": "water", "smiles": m["smiles"], "atoms": atoms})
        .to_string()
}

fn parse(s: &str) -> Value {
    serde_json::from_str(s).unwrap()
}

const PEAKS: &str = "[[95.0491,100],[67.0542,30]]";

fn with_and_without(smiles: &str, tr: &str, reatt: &str) -> (Value, Value) {
    let plain = positions_json(smiles, tr, PEAKS, 0.5, false, true).unwrap();
    let full = positions_prior_json(smiles, tr, PEAKS, 0.5, false, true, reatt).unwrap();
    (parse(&plain), parse(&full))
}

fn rank_of(v: &Value, atom: u64) -> u64 {
    let p = v["prior"].as_array().unwrap();
    let e = p.iter().find(|e| e["atom"].as_u64() == Some(atom));
    e.unwrap_or_else(|| panic!("atom {atom} in {p:?}"))["rank"]
        .as_u64()
        .unwrap()
}

#[test]
fn phenol_hydroxylation_ranks_para_ortho_meta() {
    // Oc1ccccc1: O0, ipso C1, ortho C2 and C6, meta C3 and C5, para C4
    let (plain, mut full) = with_and_without("Oc1ccccc1", "+O", &reatt_of("phenol"));
    assert_eq!(rank_of(&full, 4), 1);
    assert_eq!((rank_of(&full, 2), rank_of(&full, 6)), (2, 2));
    assert_eq!((rank_of(&full, 3), rank_of(&full, 5)), (3, 3));
    for e in full["prior"].as_array().unwrap() {
        assert_eq!(e["quantity"], "f_minus");
        assert_eq!(e["note"], "Prev.");
        assert_eq!(e["method"], "GFN2-xTB, implicit water");
        assert!(e.get("caveat").is_none());
    }
    let para = full["prior"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["atom"] == 4)
        .unwrap()["value"]
        .as_f64()
        .unwrap();
    assert!((para - 0.0814).abs() < 1e-4, "{para}");
    full.as_object_mut().unwrap().remove("prior");
    assert_eq!(plain, full, "the prior changes nothing else");
    assert!(plain.get("prior").is_none());
}

#[test]
fn classes_and_proposal_unchanged_on_a_larger_parent() {
    for (name, tr) in [
        ("paracetamol", "+O"),
        ("diclofenac", "+O"),
        ("anisole", "-CH2"),
    ] {
        let m = reference(name);
        let (plain, mut full) =
            with_and_without(m["smiles"].as_str().unwrap(), tr, &reatt_of(name));
        let prior = full.as_object_mut().unwrap().remove("prior").unwrap();
        assert_eq!(plain, full, "{name}");
        if tr == "-CH2" {
            assert_eq!(prior, json!([]), "no prior for a demethylation");
        } else {
            let sites: Vec<u64> = plain["sites"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s["atom"].as_u64().unwrap())
                .collect();
            let got: Vec<u64> = prior
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p["atom"].as_u64().unwrap())
                .collect();
            assert_eq!(
                sites, got,
                "{name}: one prior per site, in the order of the sites"
            );
        }
    }
}

#[test]
fn halogenation_carries_the_caveat() {
    let (_, full) = with_and_without("Oc1ccccc1", "+Cl-H", &reatt_of("phenol"));
    let p = full["prior"].as_array().unwrap();
    assert_eq!(p.len(), 5);
    assert!(p.iter().all(|e| e["caveat"] == HALOGEN_CAVEAT));
    assert_eq!(rank_of(&full, 4), 1);
}

#[test]
fn close_values_share_a_rank() {
    // toluene: methyl C0, ipso C1, ortho C2/C6, meta C3/C5, para C4; values written by hand
    let f = [0.01, 0.05, 0.100, 0.090, 0.0955, 0.090, 0.100];
    let atoms: Vec<Value> = f
        .iter()
        .enumerate()
        .map(|(i, v)| json!({"heavy_index": i, "f_minus": v}))
        .collect();
    let r = json!({"format": "reatt", "method": "GFN2-xTB", "solvent": "none", "atoms": atoms})
        .to_string();
    let full =
        parse(&positions_prior_json("Cc1ccccc1", "+O", PEAKS, 0.5, false, true, &r).unwrap());
    // ortho 0.100 leads; para 0.0955 is within 0.005 of it; meta 0.090 is not; the methyl (aliphatic) is last
    assert_eq!(
        (
            rank_of(&full, 2),
            rank_of(&full, 4),
            rank_of(&full, 3),
            rank_of(&full, 0)
        ),
        (1, 1, 2, 3)
    );
    assert_eq!(full["prior"][0]["method"], "GFN2-xTB, gas phase");
}

#[test]
fn a_reatt_that_does_not_fit_is_an_error() {
    let one =
        |r: &str| positions_prior_json("Oc1ccccc1", "+O", PEAKS, 0.5, false, true, r).unwrap_err();
    let e = one("{\"format\":\"reatt\",\"atoms\":[{\"heavy_index\":0,\"f_minus\":0.1}]}");
    assert_eq!(
        (e.key, e.params[0].1.as_str()),
        ("error.posiz.prior", "atoms")
    );
    assert_eq!(one("{\"format\":\"posiz\"}").params[0].1, "format");
    assert_eq!(one("not json").params[0].1, "json");
    let big = reatt_of("anisole"); // 8 heavy atoms for a parent of 7
    assert_eq!(one(&big).params[0].1, "atoms");
}
