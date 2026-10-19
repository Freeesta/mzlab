//! Form at pH 7 (task C34): rules of `protomer.rs` on small synthetic molecules, and the `protomer` field of `reatt/1`.

use mzlab_chem::protomer::assign;
use mzlab_chem::reactivity_json;
use mzlab_feat::chem::grafo::Mol;
use serde_json::Value;

/// (written SMILES of the form, total charge, groups changed)
fn form(smiles: &str) -> (String, i32, Vec<&'static str>) {
    let p = assign(smiles, &Mol::parse(smiles).unwrap());
    // the SMILES of the form must read back to the same atoms, hydrogens and charges
    let back = Mol::parse(&p.smiles).unwrap();
    let key = |m: &Mol| -> Vec<(String, u32, i32)> {
        m.atoms
            .iter()
            .map(|a| (a.el.clone(), a.h, a.charge))
            .collect()
    };
    assert_eq!(key(&back), key(&p.mol), "{smiles} -> {}", p.smiles);
    (
        p.smiles.clone(),
        p.charge,
        p.changes.iter().map(|c| c.group).collect(),
    )
}

fn charge(smiles: &str) -> i32 {
    form(smiles).1
}

#[test]
fn amine_is_protonated() {
    assert_eq!(form("CCN"), ("CC[NH3+]".into(), 1, vec!["aliphatic amine"]));
    assert_eq!(charge("CCNC(C)C"), 1);
    assert_eq!(charge("CN(C)C"), 1);
    assert_eq!(charge("C1CCNCC1"), 1);
}

#[test]
fn carboxylic_acid_is_deprotonated() {
    assert_eq!(
        form("CC(=O)O"),
        ("CC(=O)[O-]".into(), -1, vec!["carboxylic acid"])
    );
    assert_eq!(charge("OC(=O)c1ccccc1"), -1);
}

#[test]
fn phenols_follow_their_pka() {
    assert_eq!(form("Oc1ccccc1").2.len(), 0);
    // p-nitrophenol (7.15) stays neutral, 2,4-dinitrophenol (4.1) and pentachlorophenol (4.7) are anions
    assert_eq!(charge("Oc1ccc([N+](=O)[O-])cc1"), 0);
    assert_eq!(charge("Oc1ccc([N+](=O)[O-])cc1[N+](=O)[O-]"), -1);
    assert_eq!(charge("Oc1c(Cl)c(Cl)c(Cl)c(Cl)c1Cl"), -1);
    // triclosan (7.9): the chlorine is meta to the OH
    assert_eq!(charge("Oc1cc(Cl)ccc1Oc1ccc(Cl)cc1Cl"), 0);
}

#[test]
fn zwitterion_has_both_groups() {
    let (smiles, q, groups) = form("NCC(=O)O");
    assert_eq!((q, groups.len()), (0, 2));
    assert!(
        smiles.contains("[NH3+]") && smiles.contains("[O-]"),
        "{smiles}"
    );
    // lysine: two amines far apart and one acid
    assert_eq!(charge("NCCCCC(N)C(=O)O"), 1);
}

#[test]
fn second_amine_close_to_the_first_stays_neutral() {
    assert_eq!(charge("NCCN"), 1);
    assert_eq!(charge("C1CNCCN1"), 1);
    assert_eq!(charge("NCCCCN"), 2);
}

#[test]
fn neutral_groups_stay_as_written() {
    for s in [
        "Nc1ccccc1",
        "CC(=O)N",
        "CC(=O)Nc1ccc(O)cc1",
        "c1ccncc1",
        "c1cnc[nH]1",
        "CN(C)C(=O)Nc1ccc(Cl)c(Cl)c1",
        "Clc1ccc(CN2CCNC2=N[N+](=O)[O-])cn1",
        "[O-][N+](=O)c1ccccc1",
        "C[N+](C)(C)C",
        "CCSC",
    ] {
        let (smiles, _, groups) = form(s);
        assert!(groups.is_empty(), "{s}: {groups:?}");
        assert_eq!(smiles, s);
    }
    assert_eq!(charge("C[N+](C)(C)C"), 1);
}

#[test]
fn other_acidic_and_basic_groups() {
    assert_eq!(charge("CS(=O)(=O)O"), -1);
    assert_eq!(charge("CP(=O)(O)O"), -1);
    assert_eq!(charge("Cc1nnn[nH]1"), -1);
    assert_eq!(charge("NC(=N)N"), 1);
    assert_eq!(charge("NC(=N)c1ccccc1"), 1);
    assert_eq!(charge("CC(=N)N"), 1);
}

#[test]
fn charges_written_in_the_smiles_are_kept() {
    let (smiles, q, groups) = form("C[NH3+]");
    assert_eq!((smiles.as_str(), q, groups.len()), ("C[NH3+]", 1, 0));
    assert_eq!(charge("CC(=O)[O-]"), -1);
}

fn run(smiles: &str, opts: &str) -> Value {
    serde_json::from_str(&reactivity_json(smiles, opts).unwrap()).unwrap()
}

#[test]
fn reactivity_runs_on_the_ph7_form() {
    let v = run("CC(=O)O", "{\"protomer\":\"ph7\",\"conformers\":1}");
    assert_eq!(v["protomer"]["smiles"], "CC(=O)[O-]");
    assert_eq!(v["protomer"]["charge"], -1);
    assert_eq!(v["protomer"]["changes"][0]["action"], "deprotonated");
    assert_eq!(v["protomer"]["changes"][0]["heavy_index"], 3);
    assert_eq!(v["smiles"], "CC(=O)O");
    let atoms = v["atoms"].as_array().unwrap();
    assert_eq!(atoms.len(), 7);
    let q: f64 = atoms.iter().map(|a| a["q"].as_f64().unwrap()).sum();
    let fm: f64 = atoms.iter().map(|a| a["f_minus"].as_f64().unwrap()).sum();
    assert!(
        (q + 1.0).abs() < 2e-3 && (fm - 1.0).abs() < 2e-3,
        "q {q}, f- {fm}"
    );
    assert_eq!(v["scc"]["converged"], serde_json::json!([true, true, true]));

    let v = run("CCN", "{\"protomer\":\"ph7\",\"conformers\":1}");
    let q: f64 = v["atoms"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["q"].as_f64().unwrap())
        .sum();
    assert!((q - 1.0).abs() < 2e-3, "q {q}");
    assert_eq!(v["atoms"].as_array().unwrap().len(), 11);
}

#[test]
fn zwitterion_and_phenol_run() {
    let v = run("NCC(=O)O", "{\"protomer\":\"ph7\",\"conformers\":1}");
    assert_eq!(v["protomer"]["charge"], 0);
    assert_eq!(v["protomer"]["changes"].as_array().unwrap().len(), 2);
    assert!(v["protomer"]["note"].as_str().unwrap().contains("Several"));
    let v = run("Oc1ccccc1", "{\"protomer\":\"ph7\",\"conformers\":1}");
    assert_eq!(v["protomer"]["changes"].as_array().unwrap().len(), 0);
    assert_eq!(v["protomer"]["smiles"], "Oc1ccccc1");
}

#[test]
fn protomer_option_is_validated_and_off_by_default() {
    assert!(run("CCN", "{\"conformers\":1}")["protomer"].is_null());
    let e = reactivity_json("CCN", "{\"protomer\":\"ph9\"}").unwrap_err();
    assert_eq!(e.key, "error.chem.opts");
}
