#![cfg(feature = "chem")]
//! The transformation `-2H`: the sites are the C-C single bonds with an H on each carbon, and the symmetry classes join the classes of the two carbons.

use std::collections::BTreeSet;

use mzlab_feat::chem::grafo::Mol;
use mzlab_feat::chem::posizioni::{check_formulas, expected_fragments, sites, Pol};

#[test]
fn butane_has_two_bond_classes_by_symmetry() {
    let mol = Mol::parse("CCCC").unwrap();
    let s = sites(&mol, "-2H").unwrap();
    let bonds: Vec<(usize, Option<usize>)> = s.iter().map(|x| (x.atom, x.partner)).collect();
    assert_eq!(bonds, [(0, Some(1)), (1, Some(2)), (2, Some(3))]);
    let classes: BTreeSet<u32> = s.iter().map(|x| x.class).collect();
    // the two end bonds are the same position, the middle one is not
    assert_eq!(classes.len(), 2);
    assert_eq!(s[0].class, s[2].class);
    assert!(s
        .iter()
        .all(|x| x.kind == "dehydro" && x.tp_formula == "C4H8"));
    check_formulas(&s).unwrap();
}

#[test]
fn bonds_without_an_h_on_both_sides_are_not_sites() {
    // the ring carbon has no H on the bond side that would be dehydrogenated into the ring, and the aromatic bonds are not single
    let mol = Mol::parse("CCc1ccccc1").unwrap();
    let s = sites(&mol, "-2H").unwrap();
    assert_eq!(s.len(), 1, "only the CH3-CH2 bond");
    assert!(s.iter().all(|x| !mol.atoms[x.atom].arom));
    let t = sites(&Mol::parse("CC(C)(C)C").unwrap(), "-2H");
    assert!(
        t.is_err(),
        "a quaternary centre has no neighbour with an H on both sides"
    );
}

#[test]
fn the_bond_that_becomes_double_is_not_cut() {
    let mol = Mol::parse("CCCO").unwrap();
    let s = sites(&mol, "-2H").unwrap();
    let site = &s[0];
    let partner = site.partner.unwrap();
    for f in expected_fragments(&mol, site, Pol::Pos) {
        let (a, b) = (f.atoms.contains(&site.atom), f.atoms.contains(&partner));
        assert_eq!(a, b, "the double bond stays inside one fragment");
    }
}
