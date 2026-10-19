//! Hirshfeld populations (`"charges":"hirshfeld"`) and the HOMO/LUMO share of each atom in `reatt/1`.

use mzlab_chem::reactivity_json;
use serde_json::Value;

fn run(smiles: &str, opts: &str) -> Value {
    serde_json::from_str(&reactivity_json(smiles, opts).unwrap()).unwrap()
}

fn sum(v: &Value, k: &str) -> f64 {
    v["atoms"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a[k].as_f64().unwrap())
        .sum()
}

#[test]
fn hirshfeld_charges_conserve_charge_and_fukui_sum() {
    for (smiles, total) in [
        ("Oc1ccccc1", 0.0),
        ("CS(=O)(=O)N", 0.0),
        ("CC(=O)[O-]", -1.0),
    ] {
        let v = run(smiles, "{\"charges\":\"hirshfeld\",\"conformers\":1}");
        assert_eq!(v["charges"], "hirshfeld", "{smiles}");
        assert!(
            (sum(&v, "q") - total).abs() < 5e-3,
            "{smiles}: {}",
            sum(&v, "q")
        );
        assert!((sum(&v, "f_minus") - 1.0).abs() < 5e-3, "{smiles}");
        assert!((sum(&v, "f_plus") - 1.0).abs() < 5e-3, "{smiles}");
    }
}

#[test]
fn hirshfeld_signs_of_phenol() {
    let v = run("Oc1ccccc1", "{\"charges\":\"hirshfeld\",\"conformers\":1}");
    let q = |i: usize| v["atoms"][i]["q"].as_f64().unwrap();
    assert!(q(0) < -0.1, "O {}", q(0));
    assert!(q(7) > 0.1, "OH hydrogen {}", q(7));
}

#[test]
fn frontier_shares_sum_to_one_and_default_is_mulliken() {
    let v = run("Oc1ccccc1", "{\"conformers\":1}");
    assert_eq!(v["charges"], "mulliken");
    assert!((sum(&v, "homo") - 1.0).abs() < 1e-3);
    assert!((sum(&v, "lumo") - 1.0).abs() < 1e-3);
    assert!(v["atoms"][0]["homo"].as_f64().unwrap() > 0.0);
}

#[test]
fn unknown_charge_scheme_is_an_option_error() {
    let e = reactivity_json("CCO", "{\"charges\":\"cm5\"}").unwrap_err();
    assert_eq!(e.key, "error.chem.opts");
}
