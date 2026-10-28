//! A double bond that only bridges two separate rings is exocyclic for both: it must not make the ring between them aromatic, and
//! a ring that holds its double bond (phenanthrene in any Kekulé form) stays aromatic.

use mzlab_mol::Mol;

fn count(smiles: &str) -> (usize, usize) {
    let m = Mol::from_smiles(smiles).unwrap();
    let f = m.aromatic_bonds();
    (f.iter().filter(|x| **x).count(), f.len())
}

#[test]
fn bridging_double_bonds_leave_the_central_ring_alone() {
    // two fluorenylidene groups on a p-quinodimethane: only the four benzo rings are aromatic (was 30 of 38 bonds)
    let smi = "C1(=C2C3=CC=CC=C3C3=CC=CC=C23)C=CC(=C2C3=CC=CC=C3C3=CC=CC=C23)C=C1";
    assert_eq!(count(smi), (24, 38));
    let m = Mol::from_smiles(smi).unwrap();
    assert!((m.crippen_logp() - 8.077).abs() < 5e-4, "{}", m.crippen_logp());
    assert_eq!(count("C=C1C=CC(=C)C=C1"), (0, 8));
}

#[test]
fn ring_double_bonds_still_count() {
    for smi in [
        "C1=CC=C2C(=C1)C=CC1=CC=CC=C12",
        "C1=CC=CC2=C3C=CC=CC3=CC=C12",
        "c1ccc2c(c1)ccc1ccccc12",
    ] {
        assert_eq!(count(smi), (16, 16), "{smi}");
    }
}
