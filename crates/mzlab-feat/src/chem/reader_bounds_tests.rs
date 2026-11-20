//! Unit tests of what the molfile reader of the drawing (`graph::Mol`) decides ring by ring and refuses: an intact ring next to a
//! cut one, charges after a cut, a bond from an atom to itself, a charge no molfile states.

use super::tests::{mf, sel};
use super::*;

fn hs(text: &str) -> Vec<u32> {
    let m = graph::Mol::from_molfile(text).unwrap();
    m.atoms.iter().map(|a| a.h).collect()
}

const IMPL_H1: [&str; 4] = [
    "M  STY  1   1 DAT",
    "M  SAL   1  1   1",
    "M  SDT   1 MRV_IMPLICIT_H",
    "M  SED   1 IMPL_H1",
];

#[test]
fn an_intact_ring_keeps_its_hydrogens_next_to_a_cut_or_unkekulisable_one() {
    let pyrrole = [(1, 2, 4), (2, 3, 4), (3, 4, 4), (4, 5, 4), (5, 1, 4)];
    let alone = mf(&["N", "C", "C", "C", "C"], &pyrrole, &IMPL_H1);
    assert_eq!(hs(&alone), [1, 1, 1, 1, 1]);
    assert_eq!(sel(&alone, false)["label"]["text"], "C4H5N");
    // a second piece: furan after the cut of O6-C7 (open), cyclopentadiene written aromatic (cannot be kekulised)
    let open = [(7, 8, 4), (8, 9, 4), (9, 10, 4), (10, 6, 4)];
    let closed = [(6, 7, 4), (7, 8, 4), (8, 9, 4), (9, 10, 4), (10, 6, 4)];
    for (name, second, piece, h) in [
        (
            "cut furan",
            ["O", "C", "C", "C", "C"],
            &open[..],
            [1, 2, 1, 1, 1],
        ),
        ("cyclopentadiene", ["C"; 5], &closed[..], [1, 1, 1, 1, 1]),
    ] {
        let mut els = vec!["N", "C", "C", "C", "C"];
        els.extend(second);
        let mut bonds = pyrrole.to_vec();
        bonds.extend(piece);
        let got = hs(&mf(&els, &bonds, &IMPL_H1));
        assert_eq!(got[..5], [1, 1, 1, 1, 1], "{name}: the pyrrole");
        // also when the file does not say which atom of the pyrrole carries the hydrogen (the kekulisation finds it)
        let unstated = hs(&mf(&els, &bonds, &[]));
        assert_eq!(
            unstated[..5],
            [1, 1, 1, 1, 1],
            "{name}: the pyrrole, nothing stated"
        );
        // the other piece reads as it does alone
        let shifted: Vec<_> = piece.iter().map(|&(a, b, o)| (a - 5, b - 5, o)).collect();
        assert_eq!(got[5..], hs(&mf(&second, &shifted, &[])), "{name}");
        assert_eq!(got[5..], h, "{name}");
    }
}

/// `text` with the charge column of the atom line `atom` (1-based) set to `code`.
fn charge_column(text: &str, atom: usize, code: u8) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let l = &mut lines[3 + atom];
    l.replace_range(36..39, &format!("{code:>3}"));
    lines.join("\n") + "\n"
}

#[test]
fn a_cut_that_leaves_no_charge_forgets_the_charge_column() {
    // trimethylamine N-oxide with its charges in the atom lines and in `M  CHG`, as editors write them
    let bonds = [(1, 2, 1), (1, 3, 1), (1, 4, 1), (1, 5, 1)];
    let text = mf(
        &["N", "O", "C", "C", "C"],
        &bonds,
        &["M  CHG  2   1   1   2  -1"],
    );
    let text = charge_column(&charge_column(&text, 1, 3), 2, 5);
    assert_eq!(sel(&text, false)["label"]["charge"], 0);
    // O takes the + and N the pair: both neutral, N(CH3)3 has the hydrogens its valence gives (no pin), as without the columns
    let plain = mf(
        &["N", "O", "C", "C", "C"],
        &bonds,
        &["M  CHG  2   1   1   2  -1"],
    );
    let cut = |text: &str| -> serde_json::Value {
        let req = serde_json::json!({ "molfile": text, "bonds": [[0, 1]], "mode": "heterolytic", "cation": [1] });
        serde_json::from_str(&crate::chem::cut::cut_json(&req.to_string()).unwrap()).unwrap()
    };
    let out = cut(&text);
    assert_eq!(out["atoms"][0]["charge"], 0);
    assert_eq!(out["atoms"][0]["pin"], false, "{}", out["atoms"]);
    assert_eq!(out["atoms"], cut(&plain)["atoms"]);
}

#[test]
fn a_bond_from_an_atom_to_itself_is_refused_by_the_reader_of_the_drawing() {
    let text = mf(&["C", "C"], &[(1, 2, 1), (1, 1, 1)], &[]);
    let Err(e) = graph::Mol::from_molfile(&text) else {
        panic!("read")
    };
    assert_eq!(e.key, "error.grafo.molfile");
    assert!(e.to_string().contains("joins atom 1 to itself"), "{e}");
    assert!(mzlab_mol::Mol::from_molfile(&text)
        .err()
        .unwrap()
        .to_string()
        .contains("joins atom 1 to itself"));
}

#[test]
fn a_charge_no_molfile_states_is_refused_with_its_value() {
    for v in [16, -16, 100, 300] {
        let text = mf(
            &["C", "N"],
            &[(1, 2, 1)],
            &[&format!("M  CHG  1   2 {v:>3}")],
        );
        let reason = format!("the charge {v} of atom 2 is not -15 to 15");
        let Err(e) = graph::Mol::from_molfile(&text) else {
            panic!("{v}: read")
        };
        assert!(e.to_string().contains(&reason), "{e}");
        let e = mzlab_mol::Mol::from_molfile(&text)
            .err()
            .unwrap()
            .to_string();
        assert!(e.contains(&reason), "{e}");
    }
    let text = mf(&["C", "N"], &[(1, 2, 1)], &["M  CHG  1   2  15"]);
    assert_eq!(graph::Mol::from_molfile(&text).unwrap().atoms[1].charge, 15);
}
