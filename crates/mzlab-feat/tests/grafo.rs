#![cfg(feature = "chem")]
//! The graph of a structure: formula, groups and cleavages. The parent is written only by its SMILES (an amino alcohol with a morpholine and a thiadiazole,
//! C13H24N4O3S), the answers are checked by hand.

use mzlab_core::elements;
use mzlab_feat::chem::grafo::Mol;

const PARENT: &str = "CC(C)(C)NCC(COc1nsnc1N1CCOCC1)O";

fn formula(s: &str) -> String {
    elements::fmt(&Mol::parse(s).unwrap().formula())
}

#[test]
fn formulas_from_smiles() {
    assert_eq!(formula(PARENT), "C13H24N4O3S");
    assert_eq!(formula("CCO"), "C2H6O");
    assert_eq!(formula("c1ccccc1"), "C6H6");
    assert_eq!(formula("c1ccc2[nH]ccc2c1"), "C8H7N");
    assert_eq!(formula("OC(=O)c1ccc(Cl)cc1"), "C7H5ClO2");
    assert_eq!(formula("CS(=O)(=O)N"), "CH5NO2S");
    assert_eq!(formula("C[N+](=O)[O-]"), "CH3NO2");
    assert_eq!(formula("[13CH3]C"), "C2H6");
    assert_eq!(formula("c1ccoc1"), "C4H4O");
    assert_eq!(formula("C1CC1C%11CC%11"), "C6H10");
}

#[test]
fn bad_smiles_are_refused() {
    for s in ["", "C1CC", "CC(", "C.C", "CX", "[Xx]C", "C)"] {
        assert!(Mol::parse(s).is_err(), "{s}");
    }
    assert_eq!(Mol::parse("C.C").unwrap_err().key, "error.grafo.mixture");
}

fn has(s: &str, g: &str) -> bool {
    Mol::parse(s).unwrap().groups().contains(&g)
}

#[test]
fn groups_of_the_parent() {
    let g = Mol::parse(PARENT).unwrap().groups();
    for want in [
        "alcohol",
        "alpha_ch_hetero",
        "amine_secondary",
        "amine_tertiary",
        "amine_aromatic",
        "ether",
        "o_alkyl",
        "n_alkyl",
        "n_tbutyl",
        "ring_hetero_saturated",
        "sulfide",
        "heteroaromatic",
        "aromatic_ring",
    ] {
        assert!(g.contains(&want), "{want} missing in {g:?}");
    }
    for no in [
        "amide",
        "ester",
        "carboxylic_acid",
        "halogen",
        "nitro",
        "sulfonyl",
        "n_methyl",
        "n_isopropyl",
        "amine_primary",
        "phenol",
        "ketone",
        "aldehyde",
    ] {
        assert!(!g.contains(&no), "{no} found in {g:?}");
    }
    let mut sorted = g.clone();
    sorted.sort();
    assert_eq!(g, sorted);
}

#[test]
fn groups_one_by_one() {
    for (s, g) in [
        ("CC(=O)O", "carboxylic_acid"),
        ("CC(=O)OC", "ester"),
        ("CC(=O)NC", "amide"),
        ("CC=O", "aldehyde"),
        ("CC(=O)C", "ketone"),
        ("CN", "amine_primary"),
        ("CNC", "n_methyl"),
        ("CC(C)NC", "n_isopropyl"),
        ("Oc1ccccc1", "phenol"),
        ("CS(=O)(=O)NC", "sulfonamide"),
        ("CS(=O)(=O)NC", "sulfonyl"),
        ("CS(=O)C", "sulfoxide"),
        ("CSC", "sulfide"),
        ("c1ccccc1[N+](=O)[O-]", "nitro"),
        ("Clc1ccccc1", "chloro"),
        ("Brc1ccccc1", "halogen"),
        ("C1CCOCC1", "ring_hetero_saturated"),
        ("C1CCNCC1", "ring_hetero_saturated"),
        ("c1ccncc1", "heteroaromatic"),
        ("CCOCC", "alpha_ch_hetero"),
    ] {
        assert!(
            has(s, g),
            "{s} should have {g}: {:?}",
            Mol::parse(s).unwrap().groups()
        );
    }
    for (s, g) in [
        ("CC(=O)NC", "amine_secondary"),
        ("CC(=O)OC", "ether"),
        ("C1CCCCC1", "ring_hetero_saturated"),
        ("c1ccncc1", "ring_hetero_saturated"),
        ("c1ccccc1", "alpha_ch_hetero"),
        ("CC(=O)O", "alcohol"),
    ] {
        assert!(!has(s, g), "{s} should not have {g}");
    }
}

#[test]
fn cleavages_of_an_ester_and_an_amine() {
    let m = Mol::parse("CC(=O)OCC").unwrap();
    let cuts = m.cleavages();
    assert_eq!(cuts.len(), 4);
    let ester: Vec<_> = cuts.iter().filter(|c| c.kind == "ester").collect();
    assert_eq!(ester.len(), 2);
    // ethyl acetate C4H8O2: acetic acid (C2H4O2) and ethanol (C2H6O) are the hydrolysis
    let all = m.fragment_formulas();
    assert!(
        all.contains(&"C2H4O2".to_string()) && all.contains(&"C2H6O".to_string()),
        "{all:?}"
    );
    assert!(!all.contains(&"C4H8O2".to_string()));
    // no product has an element beyond the parent
    let a = Mol::parse("CCN").unwrap();
    assert!(a.fragment_formulas().iter().all(|f| !f.contains('O')));
}

#[test]
fn cleavages_skip_rings_and_double_bonds() {
    let m = Mol::parse("C1CCCCC1").unwrap();
    assert!(m.cleavages().is_empty());
    let m = Mol::parse("C=CC").unwrap();
    assert_eq!(m.cleavages().len(), 1);
    let p = Mol::parse(PARENT).unwrap();
    let f = p.fragment_formulas();
    assert!(f.windows(2).all(|w| w[0] < w[1]), "sorted and distinct");
    assert!(f.iter().all(|x| x != "C13H24N4O3S"));
    // the amine side: tert-butylamine C4H11N is among the products
    assert!(f.contains(&"C4H11N".to_string()), "{f:?}");
}

#[test]
fn json_for_the_page() {
    let j = mzlab_feat::chem::grafo::analyse_json(PARENT).unwrap();
    let v: serde_json::Value = serde_json::from_str(&j).unwrap();
    assert_eq!(v["format"], "grafo/1");
    assert_eq!(v["formula"], "C13H24N4O3S");
    assert!(v["groups"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g == "ring_hetero_saturated"));
    assert!(!v["fragments"].as_array().unwrap().is_empty());
    assert_eq!(
        mzlab_feat::chem::grafo::analyse_json("C1CC")
            .unwrap_err()
            .key,
        "error.grafo.smiles"
    );
}
