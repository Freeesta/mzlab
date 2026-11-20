//! Implicit hydrogens of the molfile reader where the file leaves them open: Si, As and Se by their valence rules, and the one hydrogen of an
//! aromatic ring written with type 4 bonds. Expected counts are the lowest allowed valence of each atom (`valence.rs`), worked out by hand.
use mzlab_mol::Mol;

/// V2000 of atoms `els` with bonds `(a, b, type)` (1-based) and `M  CHG` charges `(atom, charge)`, no coordinates and no hydrogen counts.
fn plain(els: &[&str], bonds: &[(usize, usize, u8)], charges: &[(usize, i8)]) -> String {
    let mut s = format!(
        "\n  t\n\n{:>3}{:>3}  0  0  0  0  0  0  0  0999 V2000\n",
        els.len(),
        bonds.len()
    );
    for e in els {
        s += &format!("    0.0000    0.0000    0.0000 {e:<3} 0  0  0  0  0  0  0  0  0  0  0  0\n");
    }
    for (a, b, o) in bonds {
        s += &format!("{a:>3}{b:>3}{o:>3}  0\n");
    }
    for (a, c) in charges {
        s += &format!("M  CHG  1{a:>4}{c:>4}\n");
    }
    s + "M  END\n"
}

fn hs(els: &[&str], bonds: &[(usize, usize, u8)], charges: &[(usize, i8)]) -> Vec<u8> {
    let m = Mol::from_molfile(&plain(els, bonds, charges)).unwrap();
    m.atoms.iter().map(|a| a.h).collect()
}

const RING5: &[(usize, usize, u8)] = &[(1, 2, 4), (2, 3, 4), (3, 4, 4), (4, 5, 4), (5, 1, 4)];
const RING6: &[(usize, usize, u8)] = &[
    (1, 2, 4),
    (2, 3, 4),
    (3, 4, 4),
    (4, 5, 4),
    (5, 6, 4),
    (6, 1, 4),
];

#[test]
fn selenium_fills_valence_two_and_keeps_none_when_it_is_full() {
    // dimethyl selenide: Se has its two bonds
    assert_eq!(
        hs(&["C", "Se", "C"], &[(1, 2, 1), (2, 3, 1)], &[]),
        [3, 0, 3]
    );
    // dimethyl selenoxide, as Se=O and as [Se+]-[O-]: no hydrogen on Se or O either way
    let double = hs(
        &["C", "Se", "O", "C"],
        &[(1, 2, 1), (2, 3, 2), (2, 4, 1)],
        &[],
    );
    let charged = hs(
        &["C", "Se", "O", "C"],
        &[(1, 2, 1), (2, 3, 1), (2, 4, 1)],
        &[(2, 1), (3, -1)],
    );
    assert_eq!(double, [3, 0, 0, 3]);
    assert_eq!(charged, [3, 0, 0, 3]);
    // selenic acid HO-Se(=O)(=O)-OH: valence 6, hydrogens only on the two OH
    let selenic = hs(
        &["O", "Se", "O", "O", "O"],
        &[(1, 2, 1), (2, 3, 2), (2, 4, 2), (2, 5, 1)],
        &[],
    );
    assert_eq!(selenic, [1, 0, 0, 0, 1]);
    // trimethylselenonium and methaneselenolate: the charge takes the place of a bond or of a hydrogen
    let onium = hs(
        &["Se", "C", "C", "C"],
        &[(1, 2, 1), (1, 3, 1), (1, 4, 1)],
        &[(1, 1)],
    );
    assert_eq!(onium[0], 0);
    assert_eq!(hs(&["C", "Se"], &[(1, 2, 1)], &[(2, -1)]), [3, 0]);
}

#[test]
fn arsenic_fills_valence_three_or_five() {
    let acid: &[(usize, usize, u8)] = &[(1, 2, 1), (2, 3, 2), (2, 4, 1), (2, 5, 1)];
    // arsenic acid O=As(OH)3 and the arsenate trianion
    assert_eq!(hs(&["O", "As", "O", "O", "O"], acid, &[]), [1, 0, 0, 1, 1]);
    let anion = hs(
        &["O", "As", "O", "O", "O"],
        acid,
        &[(1, -1), (4, -1), (5, -1)],
    );
    assert_eq!(anion, [0, 0, 0, 0, 0]);
    // tetramethylarsonium: four bonds and the charge, no hydrogen
    let onium = hs(
        &["As", "C", "C", "C", "C"],
        &[(1, 2, 1), (1, 3, 1), (1, 4, 1), (1, 5, 1)],
        &[(1, 1)],
    );
    assert_eq!(onium[0], 0);
}

#[test]
fn silicon_fills_valence_four_with_or_without_a_charge() {
    // the silyl anion SiH3-, and hexafluorosilicate where the six bonds leave nothing to fill
    assert_eq!(hs(&["Si"], &[], &[(1, -1)]), [3]);
    let silicate = hs(
        &["Si", "F", "F", "F", "F", "F", "F"],
        &[
            (1, 2, 1),
            (1, 3, 1),
            (1, 4, 1),
            (1, 5, 1),
            (1, 6, 1),
            (1, 7, 1),
        ],
        &[(1, -2)],
    );
    assert_eq!(silicate[0], 0);
}

#[test]
fn silicon_arsenic_and_selenium_in_an_aromatic_ring() {
    // selenophene: Se gives its lone pair, no hydrogen; 1H-arsole: the As carries the one hydrogen the file leaves open
    assert_eq!(hs(&["Se", "C", "C", "C", "C"], RING5, &[]), [0, 1, 1, 1, 1]);
    assert_eq!(hs(&["As", "C", "C", "C", "C"], RING5, &[]), [1, 1, 1, 1, 1]);
    // silabenzene has Si-H like a ring carbon, arsabenzene (arsinine) has a bare As like pyridine
    assert_eq!(
        hs(&["Si", "C", "C", "C", "C", "C"], RING6, &[]),
        [1, 1, 1, 1, 1, 1]
    );
    assert_eq!(
        hs(&["As", "C", "C", "C", "C", "C"], RING6, &[]),
        [0, 1, 1, 1, 1, 1]
    );
}

#[test]
fn the_open_hydrogen_of_an_aromatic_ring_goes_to_the_first_heteroatom_that_can_take_it() {
    // carbazole: ring A 1-6 (4 and 5 fused), N 7, ring B 8-13 (8 and 13 fused); the H is on the N, the four fusion carbons have none
    let carbazole = hs(
        &[
            "C", "C", "C", "C", "C", "C", "N", "C", "C", "C", "C", "C", "C",
        ],
        &[
            (1, 2, 4),
            (2, 3, 4),
            (3, 4, 4),
            (4, 5, 4),
            (5, 6, 4),
            (6, 1, 4),
            (5, 7, 4),
            (7, 8, 4),
            (8, 9, 4),
            (9, 10, 4),
            (10, 11, 4),
            (11, 12, 4),
            (12, 13, 4),
            (13, 8, 4),
            (13, 4, 4),
        ],
        &[],
    );
    assert_eq!(carbazole, [1, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 1, 0]);
    // two nitrogens that could both take it (tautomers the file does not tell apart): the first in file order, logged nowhere, so pinned here
    let imidazole = hs(
        &["C", "C", "N", "C", "C", "N"],
        &[
            (1, 2, 1),
            (2, 3, 4),
            (3, 4, 4),
            (4, 5, 4),
            (5, 6, 4),
            (6, 2, 4),
        ],
        &[],
    );
    assert_eq!(imidazole, [3, 0, 1, 1, 1, 0]);
    assert_eq!(hs(&["N", "N", "C", "C", "C"], RING5, &[]), [1, 0, 1, 1, 1]);
    // benzimidazole: N 7 before N 9
    let benzimidazole = hs(
        &["C", "C", "C", "C", "C", "C", "N", "C", "N"],
        &[
            (1, 2, 4),
            (2, 3, 4),
            (3, 4, 4),
            (4, 5, 4),
            (5, 6, 4),
            (6, 1, 4),
            (5, 7, 4),
            (7, 8, 4),
            (8, 9, 4),
            (9, 4, 4),
        ],
        &[],
    );
    assert_eq!(benzimidazole, [1, 1, 1, 0, 0, 1, 1, 1, 0]);
    // a ring where only one heteroatom can: oxazole needs no extra hydrogen, 1-methylimidazole has its N substituted
    assert_eq!(hs(&["O", "C", "N", "C", "C"], RING5, &[]), [0, 1, 0, 1, 1]);
    let methylated = hs(
        &["C", "N", "C", "N", "C", "C"],
        &[
            (1, 2, 1),
            (2, 3, 4),
            (3, 4, 4),
            (4, 5, 4),
            (5, 6, 4),
            (6, 2, 4),
        ],
        &[],
    );
    assert_eq!(methylated, [3, 0, 1, 0, 1, 1]);
}
