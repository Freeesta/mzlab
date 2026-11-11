//! `check/1` on small hand-built drawings: mistakes are found (pentavalent carbon, a neutral four-bonded nitrogen, an impossible isotope,
//! wedges that say nothing), and common functional groups raise no alarm (sulfone, nitro, phosphate, azide, N-oxide, carboxylate, aromatic rings).
use mzlab_feat::chem::check::check_json;

/// An atom (element, charge) and a bond (1-based atoms, type) of a hand-written molfile.
type Atom<'a> = (&'a str, i8);
type Bond = (usize, usize, u8);

/// A V2000 molfile: atoms (symbol, charge, isotope, x, y), bonds (1-based atoms, type, stereo).
fn molfile(atoms: &[(&str, i8, u16, f64, f64)], bonds: &[(usize, usize, u8, u8)]) -> String {
    let mut s = format!(
        "\n  test\n\n{:3}{:3}  0  0  0  0  0  0  0  0999 V2000\n",
        atoms.len(),
        bonds.len()
    );
    for (el, _, _, x, y) in atoms {
        s += &format!(
            "{x:10.4}{y:10.4}{:10.4} {el:<3} 0  0  0  0  0  0  0  0  0  0  0  0\n",
            0.0
        );
    }
    for (a, b, t, st) in bonds {
        s += &format!("{a:3}{b:3}{t:3}{st:3}  0  0  0\n");
    }
    for (i, (_, q, iso, _, _)) in atoms.iter().enumerate() {
        if *q != 0 {
            s += &format!("M  CHG  1 {:3} {q:3}\n", i + 1);
        }
        if *iso != 0 {
            s += &format!("M  ISO  1 {:3} {iso:3}\n", i + 1);
        }
    }
    s + "M  END\n"
}

/// Atoms without coordinates, all neutral and natural.
fn plain(els: &[&str], bonds: &[(usize, usize, u8)]) -> String {
    let atoms: Vec<_> = els
        .iter()
        .enumerate()
        .map(|(i, e)| (*e, 0, 0, i as f64, 0.0))
        .collect();
    let bonds: Vec<_> = bonds.iter().map(|&(a, b, t)| (a, b, t, 0)).collect();
    molfile(&atoms, &bonds)
}

fn check(mf: &str) -> String {
    let req = format!("{{\"molfile\":{}}}", json_str(mf));
    check_json(&req).unwrap()
}

fn json_str(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    )
}

fn codes(out: &str) -> Vec<String> {
    out.match_indices("\"code\":\"")
        .map(|(i, _)| out[i + 8..].split('"').next().unwrap().to_string())
        .collect()
}

#[test]
fn pentavalent_carbon_is_a_valence_mistake() {
    let out = check(&plain(
        &["C", "C", "C", "C", "C", "C"],
        &[(1, 2, 1), (1, 3, 1), (1, 4, 1), (1, 5, 1), (1, 6, 1)],
    ));
    assert_eq!(codes(&out), ["valence"], "{out}");
    assert!(
        out.contains(r#""atoms":[0],"bonds":[],"el":"C","valence":5,"q":0,"allowed":[4]"#),
        "{out}"
    );
}

#[test]
fn a_neutral_four_bonded_nitrogen_asks_for_a_plus_charge() {
    let els = ["N", "C", "C", "C", "C"];
    let bonds = [(1, 2, 1), (1, 3, 1), (1, 4, 1), (1, 5, 1)];
    let out = check(&plain(&els, &bonds));
    assert_eq!(codes(&out), ["valenceCharge"], "{out}");
    assert!(out.contains(r#""fix":1"#), "{out}");
    let charged: Vec<_> = els
        .iter()
        .enumerate()
        .map(|(i, e)| (*e, if i == 0 { 1 } else { 0 }, 0, i as f64, 0.0))
        .collect();
    let out = check(&molfile(&charged, &bonds.map(|(a, b, t)| (a, b, t, 0))));
    assert!(codes(&out).is_empty(), "{out}");
}

#[test]
fn common_groups_raise_no_alarm() {
    // dimethyl sulfone, DMSO, methanesulfonic acid, trimethyl phosphate
    for mf in [
        plain(
            &["C", "S", "O", "O", "C"],
            &[(1, 2, 1), (2, 3, 2), (2, 4, 2), (2, 5, 1)],
        ),
        plain(&["C", "S", "O", "C"], &[(1, 2, 1), (2, 3, 2), (2, 4, 1)]),
        plain(
            &["C", "S", "O", "O", "O"],
            &[(1, 2, 1), (2, 3, 2), (2, 4, 2), (2, 5, 1)],
        ),
        plain(
            &["P", "O", "O", "O", "O", "C", "C", "C"],
            &[
                (1, 2, 2),
                (1, 3, 1),
                (1, 4, 1),
                (1, 5, 1),
                (3, 6, 1),
                (4, 7, 1),
                (5, 8, 1),
            ],
        ),
        // benzene and pyridine written aromatic, furan in Kekulé form, acetonitrile
        plain(
            &["C", "C", "C", "C", "C", "C"],
            &[
                (1, 2, 4),
                (2, 3, 4),
                (3, 4, 4),
                (4, 5, 4),
                (5, 6, 4),
                (6, 1, 4),
            ],
        ),
        plain(
            &["N", "C", "C", "C", "C", "C"],
            &[
                (1, 2, 4),
                (2, 3, 4),
                (3, 4, 4),
                (4, 5, 4),
                (5, 6, 4),
                (6, 1, 4),
            ],
        ),
        plain(
            &["O", "C", "C", "C", "C"],
            &[(1, 2, 1), (2, 3, 2), (3, 4, 1), (4, 5, 2), (5, 1, 1)],
        ),
        plain(&["C", "C", "N"], &[(1, 2, 1), (2, 3, 3)]),
    ] {
        assert!(codes(&check(&mf)).is_empty(), "{}", check(&mf));
    }
    // nitromethane, methyl azide, pyridine N-oxide, acetate, methyl isocyanide
    let charged: &[(&[Atom], &[Bond])] = &[
        (
            &[("C", 0), ("N", 1), ("O", 0), ("O", -1)],
            &[(1, 2, 1), (2, 3, 2), (2, 4, 1)],
        ),
        (
            &[("C", 0), ("N", 0), ("N", 1), ("N", -1)],
            &[(1, 2, 1), (2, 3, 2), (3, 4, 2)],
        ),
        (
            &[
                ("N", 1),
                ("C", 0),
                ("C", 0),
                ("C", 0),
                ("C", 0),
                ("C", 0),
                ("O", -1),
            ],
            &[
                (1, 2, 2),
                (2, 3, 1),
                (3, 4, 2),
                (4, 5, 1),
                (5, 6, 2),
                (6, 1, 1),
                (1, 7, 1),
            ],
        ),
        (
            &[("C", 0), ("C", 0), ("O", 0), ("O", -1)],
            &[(1, 2, 1), (2, 3, 2), (2, 4, 1)],
        ),
        (&[("C", 0), ("N", 1), ("C", -1)], &[(1, 2, 1), (2, 3, 3)]),
    ];
    for (atoms, bonds) in charged {
        let a: Vec<_> = atoms
            .iter()
            .enumerate()
            .map(|(i, (e, q))| (*e, *q, 0, i as f64, 0.0))
            .collect();
        let b: Vec<_> = bonds.iter().map(|&(x, y, t)| (x, y, t, 0)).collect();
        let out = check(&molfile(&a, &b));
        assert!(codes(&out).is_empty(), "{out}");
    }
}

#[test]
fn a_nitro_group_without_charges_is_a_valence_mistake() {
    let out = check(&plain(
        &["C", "N", "O", "O"],
        &[(1, 2, 1), (2, 3, 2), (2, 4, 2)],
    ));
    assert_eq!(codes(&out), ["valence"], "{out}");
}

#[test]
fn charges_isotopes_pieces_and_queries() {
    let out = check(&molfile(
        &[("C", 2, 0, 0.0, 0.0), ("O", 0, 0, 1.0, 0.0)],
        &[(1, 2, 1, 0)],
    ));
    assert_eq!(codes(&out), ["charge"], "{out}");
    let out = check(&molfile(
        &[("C", 0, 13, 0.0, 0.0), ("C", 0, 40, 1.0, 0.0)],
        &[(1, 2, 1, 0)],
    ));
    assert_eq!(codes(&out), ["isotope"], "{out}");
    assert!(
        out.contains(r#""atoms":[1],"bonds":[],"el":"C","iso":40"#),
        "{out}"
    );
    let out = check(&plain(&["C", "O", "Na", "Cl"], &[(1, 2, 1)]));
    assert_eq!(codes(&out), ["pieces"], "{out}");
    assert!(
        out.contains(r#""pieces":3"#) && out.contains(r#""n":3"#),
        "{out}"
    );
    let out = check(&plain(
        &["C", "A", "C", "C", "C", "C", "C"],
        &[
            (1, 2, 1),
            (1, 3, 1),
            (1, 4, 1),
            (1, 5, 1),
            (1, 6, 1),
            (6, 7, 1),
        ],
    ));
    assert_eq!(codes(&out), ["query", "valence"], "{out}");
}

#[test]
fn an_aromatic_ring_that_cannot_alternate() {
    let out = check(&plain(
        &["C", "C", "C", "C", "C"],
        &[(1, 2, 4), (2, 3, 4), (3, 4, 4), (4, 5, 4), (5, 1, 4)],
    ));
    assert_eq!(codes(&out), ["kekule"], "{out}");
}

/// Butan-2-ol around C2 at the origin; `st` is the stereo code of the C2-O bond, `from_o` writes it from O.
fn butanol(st: u8, order: u8) -> String {
    let atoms = [
        ("C", 0, 0, -0.87, 0.5),
        ("C", 0, 0, 0.0, 0.0),
        ("C", 0, 0, 0.87, 0.5),
        ("C", 0, 0, 1.73, 0.0),
        ("O", 0, 0, 0.0, -1.0),
    ];
    molfile(
        &atoms,
        &[(1, 2, 1, 0), (2, 3, 1, 0), (3, 4, 1, 0), (2, 5, order, st)],
    )
}

#[test]
fn wedges_that_say_nothing() {
    assert!(
        codes(&check(&butanol(1, 1))).is_empty(),
        "{}",
        check(&butanol(1, 1))
    );
    assert!(codes(&check(&butanol(4, 1))).is_empty());
    let atoms = [
        ("C", 0, 0, 0.0, 0.0),
        ("C", 0, 0, 0.87, 0.5),
        ("O", 0, 0, 1.73, 0.0),
    ];
    let out = check(&molfile(&atoms, &[(1, 2, 1, 0), (2, 3, 1, 1)]));
    assert_eq!(codes(&out), ["wedgeNoCentre"], "{out}");
    assert!(out.contains(r#""atoms":[1],"bonds":[[1,2]]"#), "{out}");
    let atoms = [
        ("C", 0, 0, 0.0, 0.0),
        ("C", 0, 0, 0.87, 0.5),
        ("C", 0, 0, 1.73, 0.0),
    ];
    let out = check(&molfile(&atoms, &[(1, 2, 1, 0), (2, 3, 2, 1)]));
    assert_eq!(codes(&out), ["wedgeMultiple"], "{out}");
}

#[test]
fn bad_requests_are_errors() {
    assert_eq!(check_json("{").unwrap_err().key, "error.check.request");
    assert_eq!(
        check_json(r#"{"molfile":"x"}"#).unwrap_err().key,
        "error.check.molfile"
    );
}
