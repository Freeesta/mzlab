//! The decoy structure of the validation of the rules: same formula, deterministic for a seed, different bonds. Synthetic structures only.

use mzlab_core::elements::fmt;
use mzlab_feat::chem::grafo::Mol;

fn legami(m: &Mol) -> Vec<(usize, usize)> {
    let mut v: Vec<(usize, usize)> = m
        .bonds
        .iter()
        .map(|b| (b.a.min(b.b), b.a.max(b.b)))
        .collect();
    v.sort();
    v
}

#[test]
fn the_rewired_structure_keeps_the_formula_and_changes_the_bonds() {
    let m = Mol::parse("CC(=O)Nc1ccc(O)cc1CCN").unwrap();
    let d = m.rimescola(7);
    assert_eq!(fmt(&d.formula()), fmt(&m.formula()));
    assert_ne!(legami(&d), legami(&m));
    // every atom keeps the number of bonds and the sum of their orders
    for i in 0..m.atoms.len() {
        let ordini = |x: &Mol| {
            x.bonds
                .iter()
                .filter(|b| b.a == i || b.b == i)
                .map(|b| b.order)
                .sum::<f32>()
        };
        assert_eq!(ordini(&d), ordini(&m), "atom {i}");
    }
}

#[test]
fn the_same_seed_gives_the_same_decoy_and_another_seed_another() {
    let m = Mol::parse("CC(=O)Nc1ccc(O)cc1CCN").unwrap();
    assert_eq!(legami(&m.rimescola(3)), legami(&m.rimescola(3)));
    assert_ne!(legami(&m.rimescola(3)), legami(&m.rimescola(4)));
}

#[test]
fn a_structure_with_one_bond_has_nothing_to_swap() {
    let m = Mol::parse("CO").unwrap();
    assert_eq!(legami(&m.rimescola(1)), legami(&m));
}
