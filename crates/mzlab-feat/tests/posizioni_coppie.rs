#![cfg(feature = "interp")]
//! The transformations `+2O` and `+2O-2H`: the sites are pairs of atoms, a fragment carries the change of each atom it holds, and the position is never
//! reported as a single site.

use mzlab_core::elements::parse_formula;
use mzlab_feat::chem::grafo::Mol;
use mzlab_feat::chem::posizioni::{check_formulas, expected_fragments, is_multi, sites, Peak, Pol};
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
    assert!(s.iter().all(|x| is_multi(x.kind) && x.kind == "dioxid"));
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
    // the methyl-to-carboxyl site shares this net change and is counted apart
    let pairs: Vec<_> = s.iter().filter(|x| x.kind == "carbonyl_oh").collect();
    assert!(pairs.iter().all(|x| x.atom == 1));
    assert!(s.iter().all(|x| x.tp_formula == "C8H8O3"));
    assert_eq!(pairs.len(), 5);
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

// 2-ethylphenol-like ring with three oxidisable chain/ring atoms is too big for a hand count, so the triple test uses a short molecule.
#[test]
fn three_oxygens_are_triples_of_distinct_atoms() {
    // propylbenzene-free toy: ethylbenzene has the ring CH x5 plus CH2 and CH3 = 7 atoms that can take an oxygen
    let mol = Mol::parse("CCc1ccccc1").unwrap();
    let s = sites(&mol, "+3O").unwrap();
    assert_eq!(s.len(), 35, "C(7,3)");
    assert!(s.iter().all(|x| x.kind == "trioxid" && x.third.is_some()));
    assert!(s
        .iter()
        .all(|x| x.atom < x.partner.unwrap() && x.partner < x.third));
    assert!(s.iter().all(|x| x.tp_formula == "C8H10O3"));
    let classes: std::collections::BTreeSet<u32> = s.iter().map(|x| x.class).collect();
    assert_eq!(classes.len(), s.len());
    let frags = expected_fragments(&mol, &s[0], Pol::Pos);
    assert!(frags.iter().any(|f| f.modified));
}

#[test]
fn too_many_oxidisable_atoms_for_triples_are_refused_by_name() {
    // 20 ring CH-like atoms would give a cube of sets; the chain below has 16 aliphatic CH2
    let smiles = format!("C{}C", "C".repeat(15));
    let mol = Mol::parse(&smiles).unwrap();
    let e = sites(&mol, "+3O").unwrap_err();
    assert_eq!(e.key, "error.posiz.noSites");
}

#[test]
fn an_ethyl_group_is_lost_whole_from_a_heteroatom() {
    let mol = Mol::parse("CCN(CC)c1ccccc1").unwrap();
    let s = sites(&mol, "-C2H4").unwrap();
    assert_eq!(s.len(), 2);
    assert!(s
        .iter()
        .all(|x| x.kind == "deethyl" && x.tp_formula == "C8H11N"));
    assert!(s
        .iter()
        .all(|x| x.partner.is_some_and(|p| mol.atoms[p].h == 3)));
    assert_eq!(
        s[0].class, s[1].class,
        "the two ethyls are the same position"
    );
    // neither bond of the lost ethyl is cut, so both of its atoms sit on the nitrogen side of every fragment
    for f in expected_fragments(&mol, &s[0], Pol::Pos) {
        let (a, p) = (
            f.atoms.contains(&s[0].atom),
            f.atoms.contains(&s[0].partner.unwrap()),
        );
        assert_eq!(a, p);
    }
    let c = candidates(
        "CCN(CC)c1ccccc1",
        &parse_formula("C10H15N").unwrap(),
        &parse_formula("C8H11N").unwrap(),
    )
    .unwrap();
    assert_eq!(c.transform, "-C2H4");
}

#[test]
fn an_ethyl_on_carbon_is_not_a_site() {
    let mol = Mol::parse("CCc1ccccc1").unwrap();
    assert!(sites(&mol, "-C2H4").is_err());
}
