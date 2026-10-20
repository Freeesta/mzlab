#![cfg(feature = "interp")]
//! Substitutions of a group: Cl→OH, NH2→OH and CH3→COOH. Each one has a single atom as site, named by its net formula change.

use mzlab_core::elements::parse_formula;
use mzlab_feat::chem::grafo::Mol;
use mzlab_feat::chem::posizioni::{check_formulas, is_multi, sites};
use mzlab_feat::interp::sito::candidates;

fn transform(smiles: &str, parent: &str, tp: &str) -> String {
    candidates(
        smiles,
        &parse_formula(parent).unwrap(),
        &parse_formula(tp).unwrap(),
    )
    .unwrap()
    .transform
    .to_string()
}

#[test]
fn chlorine_replaced_by_hydroxyl_is_a_single_site() {
    // 4-chlorotoluene: the only chlorine is on the ring
    let mol = Mol::parse("Cc1ccc(Cl)cc1").unwrap();
    let s = sites(&mol, "−Cl+OH").unwrap();
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].kind, "dechloro_oh");
    assert!(!is_multi(s[0].kind));
    assert_eq!(s[0].tp_formula, "C7H8O");
    check_formulas(&s).unwrap();
    assert_eq!(transform("Cc1ccc(Cl)cc1", "C7H7Cl", "C7H8O"), "-Cl+OH");
}

#[test]
fn a_primary_amine_becomes_hydroxyl() {
    let mol = Mol::parse("Cc1ccc(N)cc1").unwrap();
    let s = sites(&mol, "NH2>OH").unwrap();
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].kind, "amino_oh");
    assert_eq!(s[0].tp_formula, "C7H8O");
    // a secondary amine has no NH2
    let sec = Mol::parse("CNc1ccccc1").unwrap();
    assert!(sites(&sec, "NH2>OH").is_err());
}

#[test]
fn a_terminal_methyl_becomes_carboxyl_and_shares_the_net_formula_with_a_carbonyl_pair() {
    let mol = Mol::parse("CCc1ccc(O)cc1").unwrap();
    let s = sites(&mol, "CH3>COOH").unwrap();
    assert!(s.iter().all(|x| x.kind == "methyl_cooh"));
    assert!(s.iter().all(|x| x.tp_formula == "C8H8O3"));
    assert_eq!(s.len(), 1);
    // the same net change +2O-2H also offers the carbonyl pairs: both kinds are candidates
    let all = sites(&mol, "+2O−2H").unwrap();
    assert!(all.iter().any(|x| x.kind == "methyl_cooh"));
    assert!(all.iter().any(|x| x.kind == "carbonyl_oh"));
    assert_eq!(transform("CCc1ccc(O)cc1", "C8H10O", "C8H8O3"), "+2O-2H");
}
