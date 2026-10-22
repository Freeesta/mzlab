//! The structure side of the rules of `fragrules/1`: each group and site on a small synthetic molecule, the products of the rules that compute one,
//! and the counterexamples where a rule must stay silent. Synthetic structures only.

use mzlab_core::msngrafo::{tabella_regole, Attesa};
use mzlab_feat::chem::grafo::Mol;
use mzlab_feat::chem::regole::{attese, nomi_sconosciuti};

fn tutte(smiles: &str) -> Vec<Attesa> {
    attese(smiles, tabella_regole()).expect("valid SMILES")
}

fn ids(smiles: &str) -> Vec<String> {
    let mut v: Vec<String> = tutte(smiles).into_iter().map(|a| a.regola).collect();
    v.dedup();
    v
}

fn prodotti(smiles: &str, regola: &str) -> Vec<String> {
    let mut v: Vec<String> = tutte(smiles)
        .into_iter()
        .filter(|a| a.regola == regola)
        .map(|a| a.formula)
        .collect();
    v.sort();
    v
}

#[test]
fn every_group_and_site_of_the_table_is_known() {
    assert_eq!(nomi_sconosciuti(tabella_regole()), Vec::<String>::new());
}

#[test]
fn the_new_groups_are_found() {
    let g = |s: &str| Mol::parse(s).unwrap().groups_ext();
    assert!(g("CP(=O)(O)O").contains(&"phosphate"));
    assert!(g("COS(=O)(=O)O").contains(&"sulfate"));
    assert!(!g("CS(=O)(=O)N").contains(&"sulfate"));
    let glc = g("OCC1OC(Oc2ccccc2)C(O)C(O)C1O");
    assert!(glc.contains(&"o_glycoside") && !glc.contains(&"glucuronide"));
    let glcur = g("OC(=O)C1OC(Oc2ccccc2)C(O)C(O)C1O");
    assert!(glcur.contains(&"o_glycoside") && glcur.contains(&"glucuronide"));
    assert!(!g("OCC1OC(O)C(O)C(O)C1O").contains(&"o_glycoside"));
}

#[test]
fn groups_ext_keeps_the_groups_of_the_proof_rules() {
    for s in [
        "NCCCC(=O)O",
        "CCOC(=O)c1ccc(cc1)CNC(C)C",
        "Clc1ccc(cc1)[N+](=O)[O-]",
    ] {
        let m = Mol::parse(s).unwrap();
        let base = m.groups();
        let ext = m.groups_ext();
        assert!(base.iter().all(|g| ext.contains(g)));
        assert_eq!(ext.len(), base.len());
    }
}

#[test]
fn each_site_selects_its_rule() {
    let casi = [
        ("CCC(N)=O", "loss_nh3_amide"),
        ("NC(=O)NC", "loss_hnco_ureido"),
        ("CN(C)CCc1ccccc1", "loss_c2h7n_n_dimethyl"),
        ("CCNCc1ccccc1", "loss_c2h4_n_ethyl"),
        ("COC(=O)CC", "loss_ch4o_ester"),
        ("COCCN(C)C", "loss_ch4o_methoxy_ether"),
        ("CC(=O)OCC", "loss_c2h4o2_acetate"),
        ("CC(=O)Nc1ccccc1", "loss_c2h2o_acetyl"),
        ("CCS(=O)(=O)O", "loss_so3_sulfate_neg"),
        ("COc1ccccc1", "loss_ch3_radical_methoxyaryl"),
        ("CNC(=O)Oc1ccccc1", "loss_c2h3no_n_methylcarbamate"),
        ("O=c1cc(-c2ccccc2)oc2cc(O)ccc12", "loss_co2_ring_neg"),
        ("O=c1cc(-c2ccccc2)oc2cc(O)ccc12", "loss_c3o2_ring_neg"),
        ("CC1=CCCCC1", "rda_six_ring"),
        ("CCOC(=O)CC", "loss_alkene_ester"),
        ("C(NC)c1ccccc1", "ion_tropylium"),
        ("CCCN", "ion_immonium_alpha"),
        ("CC(=O)c1ccccc1", "ion_acylium"),
    ];
    for (s, id) in casi {
        assert!(ids(s).iter().any(|x| x == id), "{id} on {s}");
    }
}

#[test]
fn the_sites_are_not_found_where_the_structure_lacks_them() {
    assert!(!ids("CCC(=O)NC").contains(&"loss_nh3_amide".to_string()));
    assert!(!ids("CCOCC").contains(&"loss_ch4o_methoxy_ether".to_string()));
    assert!(!ids("CCC(=O)OC").contains(&"loss_c2h4o2_acetate".to_string()));
    assert!(!ids("Cc1ccccc1").contains(&"ion_tropylium".to_string()));
    assert!(!ids("CC1=CC=CC=C1").contains(&"rda_six_ring".to_string()));
}

#[test]
fn the_rules_compute_their_products_from_the_structure() {
    assert_eq!(prodotti("CCOC(=O)CC", "loss_alkene_ester"), ["C2H4"]);
    assert_eq!(prodotti("CC(C)OC(=O)CC", "loss_alkene_ester"), ["C3H6"]);
    assert_eq!(prodotti("CC1=CCCCC1", "rda_six_ring"), ["C2H4", "C5H8"]);
    assert_eq!(prodotti("C(NC)c1ccccc1", "ion_tropylium"), ["C7H7"]);
    assert_eq!(prodotti("CCCN", "ion_immonium_alpha"), ["CH4N"]);
    assert_eq!(
        prodotti("CC(=O)c1ccccc1", "ion_acylium"),
        ["C2H3O", "C7H5O"]
    );
    let t = tutte("CCOC(=O)CC");
    assert!(t.iter().any(|a| a.regola == "ion_acylium" && a.ione));
    assert!(t.iter().all(|a| a.regola != "loss_alkene_ester" || !a.ione));
}

#[test]
fn counterexamples_give_no_loss() {
    // an aromatic OH is not an alcohol and an aromatic NH2 is not the primary amine of the rule
    assert!(!ids("Oc1ccccc1").contains(&"loss_h2o_alcohol".to_string()));
    assert!(!ids("Nc1ccccc1").contains(&"loss_nh3_amine".to_string()));
    assert!(ids("NCCCC(=O)O").contains(&"loss_nh3_amine".to_string()));
    // a glucuronide does not lose a hexose
    assert!(!ids("OC(=O)C1OC(Oc2ccccc2)C(O)C(O)C1O").contains(&"loss_hexose".to_string()));
    assert!(ids("OCC1OC(Oc2ccccc2)C(O)C(O)C1O").contains(&"loss_hexose".to_string()));
}

#[test]
fn a_rule_that_reads_the_structure_without_a_product_gives_nothing() {
    assert!(!ids("CCOC(=O)CC").contains(&"remote_fixed_charge".to_string()));
}

/// How many of the observed neutral losses the rules expect of the structure.
fn spiegate(smiles: &str, perdite: &[&str]) -> usize {
    let t = tutte(smiles);
    let hill =
        |f: &str| mzlab_core::elements::fmt(&mzlab_core::elements::parse_formula(f).unwrap());
    perdite
        .iter()
        .filter(|p| t.iter().any(|a| !a.ione && a.formula == hill(p)))
        .count()
}

#[test]
fn the_true_structure_expects_more_of_the_observed_losses_than_its_decoy() {
    // A: an amino acid, [M+H]+ loses NH3, H2O and HCOOH; the decoy has the same formula and no acid
    let a = ["NH3", "H2O", "CH2O2"];
    assert_eq!(spiegate("NCCCC(=O)O", &a), 3);
    assert!(spiegate("OCC(N)CC=O", &a) < 3);
    // B: an ethyl ester of an N-isopropylbenzylamine loses propene and ethene
    let b = ["C3H6", "C2H4"];
    assert_eq!(spiegate("CCOC(=O)c1ccc(cc1)CNC(C)C", &b), 2);
    assert!(spiegate("COCc1ccc(cc1)CNCC", &b) < 2);
    // C: a nitro-chloro-arene loses NO2, NO and HCl
    let c = ["NO2", "NO", "HCl"];
    assert_eq!(spiegate("Clc1ccc(cc1)[N+](=O)[O-]", &c), 3);
    assert!(spiegate("Clc1ccc(cc1)CCO", &c) < 3);
}
