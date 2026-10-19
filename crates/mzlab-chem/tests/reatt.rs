//! The contract `reatt/1` of `reactivity_json`: fields, hydrogens included, hydrogen bonds, errors and the EHT fallback.

use mzlab_chem::{reactivity_json, ENGINE};
use serde_json::Value;

fn run(smiles: &str, opts: &str) -> Value {
    serde_json::from_str(&reactivity_json(smiles, opts).unwrap()).unwrap()
}

#[test]
fn salicylic_acid_contract() {
    let v = run("O=C(O)c1ccccc1O", "");
    assert_eq!(
        (
            v["format"].as_str(),
            v["version"].as_u64(),
            v["engine"].as_str()
        ),
        (Some("reatt"), Some(1), Some(ENGINE))
    );
    assert_eq!(
        (v["method"].as_str(), v["solvent"].as_str()),
        (Some("GFN2-xTB"), Some("water"))
    );
    let atoms = v["atoms"].as_array().unwrap();
    assert_eq!(atoms.len(), 16);
    assert_eq!(
        atoms.iter().filter(|a| a["heavy_index"].is_null()).count(),
        6
    );
    let q: f64 = atoms.iter().map(|a| a["q"].as_f64().unwrap()).sum();
    let fm: f64 = atoms.iter().map(|a| a["f_minus"].as_f64().unwrap()).sum();
    assert!(
        q.abs() < 2e-3 && (fm - 1.0).abs() < 2e-3,
        "sum q {q}, sum f(−) {fm}"
    );
    assert!(v["hbonds"].is_array());
    assert!(v["global"]["gap_ev"].as_f64().unwrap() > 0.0 && v["global"]["omega_ev"].is_null());
    assert_eq!(v["scc"]["converged"], serde_json::json!([true, true, true]));
    let sites = v["ch_sites"].as_array().unwrap();
    assert_eq!(sites.len(), 4);
    assert!(sites
        .windows(2)
        .all(|w| w[0]["f_minus"].as_f64() >= w[1]["f_minus"].as_f64()));
    assert_eq!(sites[0]["rank"].as_u64(), Some(1));
    assert!(v["conformers"]["f_minus_spread"].as_f64().is_some());
    assert!(v["cite"]
        .as_array()
        .unwrap()
        .iter()
        .all(|c| c["doi"].as_str().is_some()));
}

#[test]
fn gas_phase_option() {
    let v = run("c1ccccc1", "{\"solvent\":\"none\",\"conformers\":1}");
    assert_eq!(v["solvent"].as_str(), Some("none"));
    assert!(v["conformers"]["f_minus_spread"].is_null());
    assert_eq!(v["ch_sites"].as_array().unwrap().len(), 1);
}

#[test]
fn errors_are_keys() {
    assert_eq!(
        reactivity_json("C1CC", "").unwrap_err().key,
        "error.chem.smiles"
    );
    assert_eq!(
        reactivity_json("CCO", "{\"solvent\":\"octanol\"}")
            .unwrap_err()
            .key,
        "error.chem.opts"
    );
    assert_eq!(
        reactivity_json("CCO", "{bad").unwrap_err().key,
        "error.chem.opts"
    );
}

#[test]
fn eht_fallback_has_no_charges() {
    let mol = mzlab_feat::chem::grafo::Mol::parse("c1ccccc1O").unwrap();
    let m = mzlab_chem::Mol3d::from_mol(&mol).unwrap();
    let c = mzlab_chem::embed(&m, 42, 1).unwrap();
    let (f, homo, lumo) = mzlab_chem::eht::fukui(&m.el, &c[0].xyz).unwrap();
    assert!(f.q.iter().all(Option::is_none) && f.f_zero.iter().all(Option::is_none));
    assert!(lumo > homo);
    assert!((f.f_minus.iter().sum::<f64>() - 1.0).abs() < 0.05);
}

#[test]
fn hydrogen_bond_geometry() {
    // Water dimer: H1 points at O3 (H···O 1.94 Å, 180°); the hydrogens of the second water are 2.6 Å or more from O0.
    let m = mzlab_chem::Mol3d {
        el: vec![8, 1, 1, 8, 1, 1],
        xyz: vec![
            [0.0, 0.0, 0.0],
            [0.96, 0.0, 0.0],
            [-0.24, 0.93, 0.0],
            [2.9, 0.0, 0.0],
            [3.2, 0.9, 0.0],
            [3.2, -0.9, 0.0],
        ],
        bonds: vec![(0, 1, 1), (0, 2, 1), (3, 4, 1), (3, 5, 1)],
        heavy_index: vec![Some(0), None, None, Some(1), None, None],
        charge: vec![0; 6],
        arom: vec![false; 6],
    };
    assert_eq!(mzlab_chem::mol3d::hbonds(&m, &m.xyz), vec![(1, 3)]);
    let far: Vec<[f64; 3]> = m
        .xyz
        .iter()
        .map(|p| {
            if p[0] > 2.0 {
                [p[0] + 0.7, p[1], p[2]]
            } else {
                *p
            }
        })
        .collect();
    assert!(mzlab_chem::mol3d::hbonds(&m, &far).is_empty());
}

#[test]
fn boltzmann_average_writes_weights_and_keeps_default_off() {
    let smiles = "COc1ccc(CCNCC(O)COc2ccccc2)cc1";
    let off: serde_json::Value =
        serde_json::from_str(&reactivity_json(smiles, "").unwrap()).unwrap();
    assert!(off["conformers"].get("weights").is_none());
    let on: serde_json::Value =
        serde_json::from_str(&reactivity_json(smiles, "{\"average\":\"boltzmann\"}").unwrap())
            .unwrap();
    let w: Vec<f64> = on["conformers"]["weights"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert_eq!(w.len(), on["conformers"]["n"].as_u64().unwrap() as usize);
    assert!((w.iter().sum::<f64>() - 1.0).abs() < 1e-3);
    assert!(w.windows(2).all(|p| p[0] >= p[1]), "lowest energy first");
    let sum_f: f64 = on["atoms"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["f_minus"].as_f64().unwrap())
        .sum();
    assert!((sum_f - 1.0).abs() < 1e-2, "{sum_f}");
    assert!(reactivity_json(smiles, "{\"average\":\"x\"}").is_err());
    let one: serde_json::Value = serde_json::from_str(
        &reactivity_json("CCO", "{\"average\":\"boltzmann\",\"conformers\":1}").unwrap(),
    )
    .unwrap();
    assert!(one["conformers"].get("weights").is_none());
}
