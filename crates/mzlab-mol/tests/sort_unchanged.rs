//! The sort and the sorted-vec sets of this crate keep every output byte: canonical form and SMILES of symmetric molecules (the values were
//! produced before the shared sort replaced the std sorts), and the validation of long untrusted lists in O(n log n).

use mzlab_mol::{Atom, Bond, Mol, MolError};
use std::time::{Duration, Instant};

/// (name, SMILES in, `can()`, `to_smiles()`).
const SYMMETRIC: &[(&str, &str, &str, &str)] = &[
    (
        "c60",
        "C12=C3C4=C5C6=C1C7=C8C9=C1C%10=C%11C(=C29)C3=C2C3=C4C4=C5C5=C9C6=C7C6=C7C8=C1C1=C8C%10=C%10C%11=C2C2=C3C3=C4C4=C5C5=C%11C%12=C(C6=C95)C7=C1C1=C%12C5=C%11C4=C3C3=C5C(=C81)C%10=C23",
        "mol1:3bbf1a5b3eea5f7ebc361e8f3e728d6f579fcce1ed2fe86686a3f823eec6a054",
        "[C]12=[C]3[C]4=[C]5[C]6=[C]1[C]1=[C]7[C]8=[C]9[C]%10=[C]%11[C](=[C]28)[C]3=[C]2[C]3=[C]4[C]4=[C]5[C]5=[C]8[C]6=[C]1[C]1=[C]6[C]7=[C]9[C]7=[C]9[C]%10=[C]%10[C]%11=[C]2[C]2=[C]3[C]3=[C]4[C]4=[C]5[C]5=[C]%11[C]%12=[C]([C]1=[C]85)[C]6=[C]7[C]1=[C]%12[C]5=[C]%11[C]4=[C]3[C]3=[C]5[C](=[C]91)[C]%10=[C]23",
    ),
    (
        "cubane",
        "C12C3C4C1C5C2C3C45",
        "mol1:dc2ab5c7ad6c7413888793177b7b0246379dea23aefe589982d2eb0188156555",
        "[CH]12[CH]3[CH]4[CH]1[CH]1[CH]2[CH]3[CH]41",
    ),
    (
        "adamantane",
        "C1C2CC3CC1CC(C2)C3",
        "mol1:67a4f462b7c9ebd8296e524715974c21c44c5a5fcb5290a81991e2ec3b10f650",
        "[CH2]1[CH]2[CH2][CH]3[CH2][CH]1[CH2][CH]([CH2]2)[CH2]3",
    ),
    (
        "neopentane",
        "CC(C)(C)C",
        "mol1:b7307a48e99a86b35e34e43c59016bb323cd12f4ea0e48f5364354d95165231b",
        "[CH3][C]([CH3])([CH3])[CH3]",
    ),
    (
        "triphenylbenzene",
        "c1ccc(cc1)-c1cc(-c2ccccc2)cc(-c2ccccc2)c1",
        "mol1:9255eab3e6b2747281ee383d6e978403f58e7b0c2cbbaf3b18d50f0a5c9df09f",
        "[CH]=1[CH]=[CH][C](=[CH][CH]1)[C]=1[CH]=[C]([C]=2[CH]=[CH][CH]=[CH][CH]2)[CH]=[C]([C]=2[CH]=[CH][CH]=[CH][CH]2)[CH]1",
    ),
    (
        "cyclooctane",
        "C1CCCCCCC1",
        "mol1:92d90b4e96ad2b391e59d58365ada592d2799ff9a21754ce23edb4b555081b46",
        "[CH2]1[CH2][CH2][CH2][CH2][CH2][CH2][CH2]1",
    ),
];

#[test]
fn canonical_form_and_smiles_of_symmetric_molecules_are_unchanged() {
    for (name, input, can, smiles) in SYMMETRIC {
        let m = Mol::from_smiles(input).expect(name);
        assert_eq!(m.can(), *can, "{name} can");
        assert_eq!(m.to_smiles().expect(name), *smiles, "{name} smiles");
    }
}

fn atoms(n: u32) -> Vec<Atom> {
    (0..n)
        .map(|_| Atom {
            el: "C".into(),
            iso: 0,
            chg: 0,
            rad: 0,
            h: 0,
        })
        .collect()
}

/// `count` distinct bonds a < b, in DESCENDING order (the worst case of a sorted insert).
fn descending(n: u32, count: usize) -> Vec<Bond> {
    let mut v = Vec::with_capacity(count);
    'all: for b in (1..n).rev() {
        for a in (0..b).rev() {
            if v.len() == count {
                break 'all;
            }
            v.push(Bond { a, b, o: 1 });
        }
    }
    v
}

#[test]
fn validating_100k_descending_bonds_is_fast_and_reports_a_repeat_by_index() {
    let t = Instant::now();
    let mut bonds = descending(1000, 100_000);
    let m = Mol::new(atoms(1000), bonds.clone(), vec![], vec![]);
    // valence errors may come from later checks; the bond list itself must pass
    if let Err(MolError::Invalid(msg)) = m.validate() {
        assert!(!msg.contains("repeats"), "{msg}");
    }
    bonds.push(bonds[7].clone());
    let m = Mol::new(atoms(1000), bonds.clone(), vec![], vec![]);
    let Err(MolError::Invalid(msg)) = m.validate() else {
        panic!("a repeated bond is invalid");
    };
    assert!(msg.starts_with("bonds[100000] repeats the bond"), "{msg}");
    assert!(t.elapsed() < Duration::from_secs(3), "{:?}", t.elapsed());
}

#[test]
fn a_repeat_before_a_malformed_bond_is_reported_first() {
    let bad = |order: u8| Bond {
        a: 0,
        b: 1,
        o: order,
    };
    let mk = |bonds: Vec<Bond>| Mol::new(atoms(4), bonds, vec![], vec![]).validate();
    let line = |bonds: Vec<Bond>| match mk(bonds) {
        Err(MolError::Invalid(m)) => m,
        other => panic!("{other:?}"),
    };
    let ok = |a, b| Bond { a, b, o: 1 };
    assert!(line(vec![ok(0, 1), ok(1, 2), ok(0, 1), bad(9)]).starts_with("bonds[2] repeats"));
    assert!(line(vec![ok(0, 1), bad(9), ok(0, 1)]).starts_with("bonds[1].o is 9"));
}
