#![cfg(feature = "interp")]
//! The transformations `+2O` and `+2O-2H`: the sites are pairs of atoms, a fragment carries the change of each atom it holds, and the position is never
//! reported as a single site.

use mzlab_core::elements::parse_formula;
use mzlab_feat::chem::grafo::Mol;
use mzlab_feat::chem::posizioni::{check_formulas, expected_fragments, is_pair, sites, Peak, Pol};
use mzlab_feat::interp::disegno::drawing;
use mzlab_feat::interp::sito::{analyse, candidates, Facts};

const FACTS: Facts = Facts {
    scans: 3,
    clean: 3,
    isomers: "single",
    has_reference: true,
};

// 4-ethylphenol: oxidisable atoms are the ring CH (4), the CH2 (7 atoms: 0 CH3, 1 CH2) and the CH3; the OH oxygen is not a site.
const PHENOL: &str = "CCc1ccc(O)cc1";

#[test]
fn pairs_are_distinct_atoms_with_one_product_formula() {
    let mol = Mol::parse(PHENOL).unwrap();
    let s = sites(&mol, "+2O").unwrap();
    assert!(s.iter().all(|x| is_pair(x.kind) && x.kind == "dioxid"));
    assert!(s.iter().all(|x| x.partner.is_some_and(|p| p > x.atom)));
    assert!(s.iter().all(|x| x.tp_formula == "C8H10O3"));
    check_formulas(&s).unwrap();
    // 0 CH3, 1 CH2 and the four ring CH can each take an oxygen: 6 atoms, 15 pairs
    assert_eq!(s.len(), 15);
}

#[test]
fn a_carbonyl_pair_needs_a_ch2_and_another_atom() {
    let mol = Mol::parse(PHENOL).unwrap();
    let s = sites(&mol, "+2O−2H").unwrap();
    assert!(s.iter().all(|x| x.kind == "carbonyl_oh" && x.atom == 1));
    assert!(s.iter().all(|x| x.tp_formula == "C8H8O3"));
    assert_eq!(s.len(), 5);
}

#[test]
fn a_fragment_carries_the_change_of_every_atom_it_holds() {
    let mol = Mol::parse(PHENOL).unwrap();
    let s = sites(&mol, "+2O").unwrap();
    // both oxygens on the ring: every fragment that holds only one of them carries +O, the whole ring side carries +2O
    let pair = s
        .iter()
        .find(|x| (x.atom, x.partner) == (3, Some(4)))
        .unwrap();
    let frags = expected_fragments(&mol, pair, Pol::Pos);
    assert!(frags.iter().any(|f| f.modified));
    let base = expected_fragments(&mol, &sites(&mol, "+O").unwrap()[0], Pol::Pos);
    assert!(!base.is_empty());
    let both = frags
        .iter()
        .find(|f| f.atoms.contains(&3) && f.atoms.contains(&4) && !f.atoms.contains(&1))
        .unwrap();
    let plain = frags
        .iter()
        .find(|f| f.bond == both.bond && f.atoms == both.atoms)
        .unwrap();
    assert_eq!(plain.formula, both.formula);
    assert!(both.formula.contains("O3") || both.formula.contains("O2"));
}

#[test]
fn two_changes_are_never_a_site_and_the_tree_is_not_used() {
    let mol = Mol::parse(PHENOL).unwrap();
    let all = sites(&mol, "+2O").unwrap();
    let peaks: Vec<Peak> = expected_fragments(&mol, &all[0], Pol::Pos)
        .iter()
        .enumerate()
        .map(|(i, f)| Peak {
            mz: f.mz,
            inten: 100.0 - i as f64,
        })
        .collect();
    let c = candidates(
        PHENOL,
        &parse_formula("C8H10O").unwrap(),
        &parse_formula("C8H10O3").unwrap(),
    )
    .unwrap();
    assert_eq!(c.transform, "+2O");
    let r = analyse(&c, &peaks, Pol::Pos, &FACTS, None);
    assert_eq!(r.source, "cuts");
    assert_ne!(r.confidence, "site");
    let d = drawing(PHENOL, &r);
    assert_ne!(d.mode, "structure");
}

#[test]
fn the_delta_of_the_pair_is_named_by_transform_not_refused() {
    for (tp, want) in [("C8H10O3", "+2O"), ("C8H8O3", "+2O-2H")] {
        let c = candidates(
            PHENOL,
            &parse_formula("C8H10O").unwrap(),
            &parse_formula(tp).unwrap(),
        )
        .unwrap();
        assert_eq!(c.transform, want);
    }
}
