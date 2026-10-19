//! HOMO/LUMO share per atom of `reatt/1`: each list sums to 1 and the phenol HOMO sits on the ring.

use mzlab_chem::reactivity_json;
use serde_json::Value;

fn run(smiles: &str) -> Value {
    serde_json::from_str(&reactivity_json(smiles, "{\"conformers\":1}").unwrap()).unwrap()
}

fn shares(v: &Value, k: &str) -> Vec<f64> {
    let atoms = v["atoms"].as_array().unwrap();
    atoms.iter().map(|a| a[k].as_f64().unwrap()).collect()
}

#[test]
fn shares_sum_to_one() {
    for smiles in ["Oc1ccccc1", "CC(=O)[O-]"] {
        let v = run(smiles);
        for k in ["homo", "lumo"] {
            let s: f64 = shares(&v, k).iter().sum();
            assert!((s - 1.0).abs() < 1e-3, "{smiles} {k} sums to {s}");
        }
    }
}

#[test]
fn phenol_homo_is_on_the_ring() {
    let v = run("Oc1ccccc1");
    let h = shares(&v, "homo");
    let ring: f64 = h[1..7].iter().sum();
    assert!(ring > 0.5, "ring carbons hold {ring} of the HOMO");
}
