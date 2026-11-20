//! Unit tests of `selection/1` (`sel.rs`): Kekulé and aromatic-typed benzene, fused and saturated rings, charge, radical and pieces, request errors.

use super::*;

/// V2000 of atoms `els` with bonds `(a, b, type)` (1-based) and property lines.
pub(super) fn mf(els: &[&str], bonds: &[(usize, usize, u8)], props: &[&str]) -> String {
    let mut s = format!(
        "\n  t\n\n{:>3}{:>3}  0  0  0  0  0  0  0  0999 V2000\n",
        els.len(),
        bonds.len()
    );
    for (i, e) in els.iter().enumerate() {
        s += &format!(
            "{:>10.4}{:>10.4}{:>10.4} {e:<3} 0  0  0  0  0  0  0  0  0  0  0  0\n",
            i as f64, 0.0, 0.0
        );
    }
    for (a, b, o) in bonds {
        s += &format!("{a:>3}{b:>3}{o:>3}  0  0  0  0\n");
    }
    props.iter().for_each(|p| s += &format!("{p}\n"));
    s + "M  END"
}
pub(super) fn sel(mf: &str, part: bool) -> serde_json::Value {
    let req = serde_json::json!({ "molfile": mf, "part": part }).to_string();
    serde_json::from_str(&sel_json(&req).unwrap()).unwrap()
}
const BENZENE: [(usize, usize, u8); 6] = [
    (1, 2, 2),
    (2, 3, 1),
    (3, 4, 2),
    (4, 5, 1),
    (5, 6, 2),
    (6, 1, 1),
];

#[test]
fn kekule_benzene_is_one_aromatic_ring() {
    let v = sel(&mf(&["C"; 6], &BENZENE, &[]), false);
    assert_eq!(v["schema"], "selection/1");
    assert_eq!(v["label"]["formula"], "C6H6");
    assert!((v["label"]["mass"].as_f64().unwrap() - 78.04695).abs() < 1e-5);
    assert!((v["average"].as_f64().unwrap() - 78.114).abs() < 1e-3);
    assert_eq!(
        (
            v["heavy"].as_i64(),
            v["rings"].as_i64(),
            v["aromaticRings"].as_i64(),
            v["aromaticAtoms"].as_i64()
        ),
        (Some(6), Some(1), Some(1), Some(6))
    );
    assert!(v["label"]["mz"].is_null() && v["part"] == false);
}

#[test]
fn benzene_written_aromatic_counts_the_same() {
    let b: Vec<_> = BENZENE.iter().map(|&(a, c, _)| (a, c, 4)).collect();
    let v = sel(&mf(&["C"; 6], &b, &[]), false);
    assert_eq!(
        (
            v["label"]["formula"].as_str(),
            v["rings"].as_i64(),
            v["aromaticRings"].as_i64(),
            v["aromaticAtoms"].as_i64()
        ),
        (Some("C6H6"), Some(1), Some(1), Some(6)),
        "{v}"
    );
}

#[test]
fn naphthalene_two_rings_cyclohexane_none_aromatic() {
    let mut b = BENZENE.to_vec();
    b[5] = (6, 1, 1);
    b.extend([(1, 7, 1), (7, 8, 2), (8, 9, 1), (9, 10, 2), (10, 6, 1)]);
    b[0] = (1, 2, 2);
    let v = sel(&mf(&["C"; 10], &b, &[]), false);
    assert_eq!(
        (
            v["label"]["formula"].as_str(),
            v["rings"].as_i64(),
            v["aromaticRings"].as_i64()
        ),
        (Some("C10H8"), Some(2), Some(2)),
        "{v}"
    );
    let hex: Vec<_> = (1..=6).map(|i| (i, i % 6 + 1, 1)).collect();
    let v = sel(&mf(&["C"; 6], &hex, &[]), true);
    assert_eq!(
        (
            v["label"]["formula"].as_str(),
            v["rings"].as_i64(),
            v["aromaticRings"].as_i64(),
            v["part"].as_bool()
        ),
        (Some("C6H12"), Some(1), Some(0), Some(true))
    );
}

#[test]
fn charge_radical_and_pieces() {
    // acetate: CC(=O)[O-]
    let v = sel(
        &mf(
            &["C", "C", "O", "O"],
            &[(1, 2, 1), (2, 3, 2), (2, 4, 1)],
            &["M  CHG  1   4  -1"],
        ),
        false,
    );
    assert_eq!(
        (v["label"]["text"].as_str(), v["label"]["charge"].as_i64()),
        (Some("C2H3O2-"), Some(-1))
    );
    assert!(
        (v["label"]["mz"].as_f64().unwrap() - 59.01385).abs() < 1e-4,
        "{v}"
    );
    // methyl radical and a separate methane: two pieces, one unpaired electron
    let v = sel(&mf(&["C", "C"], &[], &["M  RAD  1   1   2"]), true);
    assert_eq!(
        (
            v["label"]["formula"].as_str(),
            v["pieces"].as_i64(),
            v["label"]["unpaired"].as_i64()
        ),
        (Some("C2H7"), Some(2), Some(1)),
        "{v}"
    );
}

#[test]
fn errors_keep_their_key() {
    assert_eq!(sel_json("nope").unwrap_err().key, "error.sel.request");
    assert_eq!(
        sel_json("{\"molfile\":\"x\"}").unwrap_err().key,
        "error.grafo.molfile"
    );
    assert_eq!(sel_json("{}").unwrap_err().key, "error.sel.request");
}

/// Formula with charge of a drawn structure.
fn text(els: &[&str], bonds: &[(usize, usize, u8)], props: &[&str]) -> String {
    sel(&mf(els, bonds, props), false)["label"]["text"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn a_charged_atom_with_all_its_bonds_takes_no_hydrogen() {
    // formulas by hand: the charged atom has all its bonds, the carbons fill valence four
    let star = [(1, 2, 1), (2, 3, 1), (2, 4, 1), (2, 5, 1)];
    let plus = ["M  CHG  1   2   1"];
    // tetramethylammonium, tetramethylphosphonium, trimethylsulfonium, trimethyloxonium
    assert_eq!(text(&["C", "N", "C", "C", "C"], &star, &plus), "C4H12N+");
    assert_eq!(text(&["C", "P", "C", "C", "C"], &star, &plus), "C4H12P+");
    assert_eq!(text(&["C", "S", "C", "C"], &star[..3], &plus), "C3H9S+");
    assert_eq!(text(&["C", "O", "C", "C"], &star[..3], &plus), "C3H9O+");
    // trimethylsulfoxonium: S+ with five valences
    assert_eq!(
        text(
            &["C", "S", "C", "C", "O"],
            &[(1, 2, 1), (2, 3, 1), (2, 4, 1), (2, 5, 2)],
            &plus
        ),
        "C3H9OS+"
    );
    // nitromethane and trimethylamine N-oxide drawn with separated charges
    let zw = ["M  CHG  2   2   1   3  -1"];
    assert_eq!(
        text(
            &["C", "N", "O", "O"],
            &[(1, 2, 1), (2, 3, 1), (2, 4, 2)],
            &zw
        ),
        "CH3NO2"
    );
    assert_eq!(text(&["C", "N", "O", "C", "C"], &star, &zw), "C3H9NO");
    // diazomethane C=[N+]=[N-], carbon monoxide [C-]#[O+], methyl isocyanide C[N+]#[C-]
    assert_eq!(
        text(&["C", "N", "N"], &[(1, 2, 2), (2, 3, 2)], &zw),
        "CH2N2"
    );
    assert_eq!(
        text(&["C", "O"], &[(1, 2, 3)], &["M  CHG  2   1  -1   2   1"]),
        "CO"
    );
    assert_eq!(
        text(&["C", "N", "C"], &[(1, 2, 1), (2, 3, 3)], &zw),
        "C2H3N"
    );
    // 1-methylpyridinium, Kekulé and with aromatic bonds
    let ring = |o: [u8; 6]| {
        let mut b: Vec<(usize, usize, u8)> =
            (0..6).map(|i| (i + 1, (i + 1) % 6 + 1, o[i])).collect();
        b.push((1, 7, 1));
        b
    };
    let py = ["N", "C", "C", "C", "C", "C", "C"];
    let n1 = ["M  CHG  1   1   1"];
    assert_eq!(text(&py, &ring([2, 1, 2, 1, 2, 1]), &n1), "C6H8N+");
    assert_eq!(text(&py, &ring([4; 6]), &n1), "C6H8N+");
}

#[test]
fn a_charged_atom_with_open_valences_is_filled_as_before() {
    let one = |el: &str, q: &str| text(&[el], &[], &[&format!("M  CHG  1   1 {q:>3}")]);
    assert_eq!(one("N", "1"), "H4N+");
    assert_eq!(one("O", "1"), "H3O+");
    assert_eq!(one("S", "1"), "H3S+");
    assert_eq!(one("P", "1"), "H4P+");
    assert_eq!(one("O", "-1"), "HO-");
    assert_eq!(one("S", "-1"), "HS-");
    assert_eq!(one("N", "-1"), "H2N-");
    assert_eq!(one("C", "1"), "CH3+");
    assert_eq!(one("C", "-1"), "CH3-");
    assert_eq!(one("Cl", "-1"), "Cl-");
    // pyridinium: the aromatic N+ carries its hydrogen
    let ring: Vec<(usize, usize, u8)> = (0..6).map(|i| (i + 1, (i + 1) % 6 + 1, 4)).collect();
    assert_eq!(
        text(
            &["N", "C", "C", "C", "C", "C"],
            &ring,
            &["M  CHG  1   1   1"]
        ),
        "C5H6N+"
    );
    // hexafluorophosphate keeps six bonds and no hydrogen
    let pf6: Vec<(usize, usize, u8)> = (2..8).map(|i| (1, i, 1)).collect();
    assert_eq!(
        text(
            &["P", "F", "F", "F", "F", "F", "F"],
            &pf6,
            &["M  CHG  1   1  -1"]
        ),
        "F6P-"
    );
}

#[test]
fn an_anion_with_all_its_bonds_takes_no_hydrogen() {
    // dimethylamide (CH3)2N- and methanethiolate CH3S-
    let minus = ["M  CHG  1   2  -1"];
    assert_eq!(
        text(&["C", "N", "C"], &[(1, 2, 1), (2, 3, 1)], &minus),
        "C2H6N-"
    );
    assert_eq!(text(&["C", "S"], &[(1, 2, 1)], &minus), "CH3S-");
}

#[test]
fn neutral_atoms_fill_their_lowest_valence_as_before() {
    assert_eq!(text(&["N"], &[], &[]), "NH3");
    assert_eq!(text(&["P"], &[], &[]), "H3P");
    assert_eq!(text(&["S"], &[], &[]), "H2S");
    // dimethyl sulfoxide (S valence 4) and dimethyl sulfone (S valence 6)
    assert_eq!(
        text(
            &["C", "S", "C", "O"],
            &[(1, 2, 1), (2, 3, 1), (2, 4, 2)],
            &[]
        ),
        "C2H6OS"
    );
    assert_eq!(
        text(
            &["C", "S", "C", "O", "O"],
            &[(1, 2, 1), (2, 3, 1), (2, 4, 2), (2, 5, 2)],
            &[]
        ),
        "C2H6O2S"
    );
    // phosphoric acid O=P(OH)3 (P valence 5) and methanesulfonic acid
    assert_eq!(
        text(
            &["O", "P", "O", "O", "O"],
            &[(1, 2, 2), (2, 3, 1), (2, 4, 1), (2, 5, 1)],
            &[]
        ),
        "H3O4P"
    );
    assert_eq!(
        text(
            &["C", "S", "O", "O", "O"],
            &[(1, 2, 1), (2, 3, 2), (2, 4, 2), (2, 5, 1)],
            &[]
        ),
        "CH4O3S"
    );
}

#[test]
fn an_aromatic_oxygen_or_sulfur_cation_gives_a_ring_electron_and_takes_no_hydrogen() {
    let ring: Vec<(usize, usize, u8)> = (0..6).map(|i| (i + 1, (i + 1) % 6 + 1, 4)).collect();
    let plus = ["M  CHG  1   1   1"];
    // pyrylium C5H5O+ and thiopyrylium C5H5S+ written with aromatic bonds
    assert_eq!(
        text(&["O", "C", "C", "C", "C", "C"], &ring, &plus),
        "C5H5O+"
    );
    assert_eq!(
        text(&["S", "C", "C", "C", "C", "C"], &ring, &plus),
        "C5H5S+"
    );
    // the same rings in Kekule form
    let kekule = [
        (1, 2, 2),
        (2, 3, 1),
        (3, 4, 2),
        (4, 5, 1),
        (5, 6, 2),
        (6, 1, 1),
    ];
    assert_eq!(
        text(&["O", "C", "C", "C", "C", "C"], &kekule, &plus),
        "C5H5O+"
    );
    assert_eq!(
        text(&["S", "C", "C", "C", "C", "C"], &kekule, &plus),
        "C5H5S+"
    );
    // S-methylthiophenium: the S+ has three bonds and gives its lone pair, no hydrogen and no ring electron
    let five = [
        (1, 2, 4),
        (2, 3, 4),
        (3, 4, 4),
        (4, 5, 4),
        (5, 1, 4),
        (1, 6, 1),
    ];
    assert_eq!(
        text(&["S", "C", "C", "C", "C", "C"], &five, &plus),
        "C5H7S+"
    );
    // neutral furan and thiophene are untouched
    assert_eq!(text(&["O", "C", "C", "C", "C"], &five[..5], &[]), "C4H4O");
    assert_eq!(text(&["S", "C", "C", "C", "C"], &five[..5], &[]), "C4H4S");
}

#[test]
fn the_aromatic_hydrogen_that_the_file_states_is_counted() {
    // indole's five-membered ring alone (pyrrole) as Ketcher writes it: aromatic bonds and the H of the N in a data group
    let ring = [(1, 2, 4), (2, 3, 4), (3, 4, 4), (4, 5, 4), (5, 1, 4)];
    let stated = [
        "M  STY  1   1 DAT",
        "M  SLB  1   1   1",
        "M  SAL   1  1   1",
        "M  SDT   1 MRV_IMPLICIT_H",
        "M  SDD   1     0.0000    0.0000    DA    ALL  1       1",
        "M  SED   1 IMPL_H1",
    ];
    let v = sel(&mf(&["N", "C", "C", "C", "C"], &ring, &stated), false);
    assert_eq!(v["label"]["text"], "C4H5N");
    assert_eq!(v["label"]["unpaired"], 0);
}
