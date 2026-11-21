//! A V3000 molfile gives the same graph as the V2000 one of the same molecule (`selection/1`, `frag/1`, `parz/1` byte for byte), a file above 999
//! atoms (V3000 only) is read by `mzlab-mol` and refused by the graph for its size, and a damaged V3000 file is an error, never an empty molecule.

use super::tests::{mf, sel};
use super::*;
use crate::chem::fragments::{fragments_json, partial_json};

/// The V3000 text of a V2000 molfile written by `mf`: the same atoms, bonds, charges, isotopes and radical codes.
fn to_v3000(v2000: &str) -> String {
    let lines: Vec<&str> = v2000.lines().collect();
    let count = |a: usize, b: usize| lines[3][a..b].trim().parse::<usize>().unwrap();
    let (na, nb) = (count(0, 3), count(3, 6));
    let mut props = vec![String::new(); na];
    for l in &lines[4 + na + nb..] {
        let key = match &l[..l.len().min(6)] {
            "M  CHG" => "CHG",
            "M  ISO" => "MASS",
            "M  RAD" => "RAD",
            _ => continue,
        };
        let v: Vec<i32> = l[6..]
            .split_whitespace()
            .map(|t| t.parse().unwrap())
            .collect();
        for p in v[1..].chunks(2) {
            props[p[0] as usize - 1] += &format!(" {key}={}", p[1]);
        }
    }
    let mut s =
        String::from("\n  t\n\n  0  0  0  0  0  0  0  0  0  0999 V3000\nM  V30 BEGIN CTAB\n");
    s += &format!("M  V30 COUNTS {na} {nb} 0 0 0\nM  V30 BEGIN ATOM\n");
    for (i, l) in lines[4..4 + na].iter().enumerate() {
        let x: Vec<&str> = [&l[0..10], &l[10..20], &l[20..30]].map(str::trim).into();
        s += &format!(
            "M  V30 {} {} {} {} {} 0{}\n",
            i + 1,
            l[31..34].trim(),
            x[0],
            x[1],
            x[2],
            props[i]
        );
    }
    s += "M  V30 END ATOM\nM  V30 BEGIN BOND\n";
    for (i, l) in lines[4 + na..4 + na + nb].iter().enumerate() {
        let n = |a: usize, b: usize| l[a..b].trim().to_string();
        s += &format!("M  V30 {} {} {} {}\n", i + 1, n(6, 9), n(0, 3), n(3, 6));
    }
    s + "M  V30 END BOND\nM  V30 END CTAB\nM  END\n"
}

/// A saturated chain of `n` carbons, V3000 (as `mzlab-mol` writes it above 999 atoms).
fn chain_v3000(n: usize) -> String {
    let mut s =
        String::from("\n  t\n\n  0  0  0  0  0  0  0  0  0  0999 V3000\nM  V30 BEGIN CTAB\n");
    s += &format!("M  V30 COUNTS {n} {} 0 0 0\nM  V30 BEGIN ATOM\n", n - 1);
    for i in 1..=n {
        s += &format!("M  V30 {i} C {}.0 0.0 0.0 0\n", i % 50);
    }
    s += "M  V30 END ATOM\nM  V30 BEGIN BOND\n";
    for i in 1..n {
        s += &format!("M  V30 {i} 1 {i} {}\n", i + 1);
    }
    s + "M  V30 END BOND\nM  V30 END CTAB\nM  END\n"
}

const KEKULE: [(usize, usize, u8); 6] = [
    (1, 2, 2),
    (2, 3, 1),
    (3, 4, 2),
    (4, 5, 1),
    (5, 6, 2),
    (6, 1, 1),
];

fn same(els: &[&str], bonds: &[(usize, usize, u8)], props: &[&str]) {
    let two = mf(els, bonds, props);
    let three = to_v3000(&two);
    for part in [false, true] {
        let req = |m: &str| serde_json::json!({ "molfile": m, "part": part }).to_string();
        assert_eq!(
            sel_json(&req(&three)).unwrap(),
            sel_json(&req(&two)).unwrap(),
            "{three}"
        );
    }
    assert_eq!(
        fragments_json(&three, -1, -1).unwrap(),
        fragments_json(&two, -1, -1).unwrap()
    );
    assert_eq!(
        partial_json(&three, "+O -H2").unwrap(),
        partial_json(&two, "+O -H2").unwrap()
    );
}

#[test]
fn v2000_and_v3000_of_one_molecule_give_the_same_bytes() {
    same(&["C", "C", "O"], &[(1, 2, 1), (2, 3, 1)], &[]);
    same(&["C"; 6], &KEKULE, &[]);
    let arom: Vec<_> = KEKULE.iter().map(|&(a, b, _)| (a, b, 4)).collect();
    same(&["C"; 6], &arom, &[]);
    same(
        &["C", "N", "C"],
        &[(1, 2, 1), (2, 3, 1)],
        &["M  CHG  1   2   1"],
    );
    same(
        &["C", "C", "C"],
        &[(1, 2, 1), (2, 3, 1)],
        &["M  ISO  1   1  13"],
    );
    for code in [1, 2, 3] {
        same(
            &["C", "C"],
            &[(1, 2, 1)],
            &[&format!("M  RAD  1   2   {code}")],
        );
    }
    same(&["N", "C", "C", "C", "C", "C"], &arom, &[]);
}

#[test]
fn a_v3000_file_reads_as_its_molecule_never_as_an_empty_one() {
    let v = sel(
        &to_v3000(&mf(&["C", "C", "O"], &[(1, 2, 1), (2, 3, 1)], &[])),
        false,
    );
    assert_eq!(
        (v["label"]["formula"].as_str(), v["heavy"].as_i64()),
        (Some("C2H6O"), Some(3))
    );
}

#[test]
fn a_chain_the_graph_takes_is_the_same_in_both_formats() {
    let els = vec!["C"; 150];
    let bonds: Vec<_> = (1..150).map(|i| (i, i + 1, 1u8)).collect();
    // `mf` writes the V2000 columns; the 150-atom chain stays inside the graph limit
    let two = mf(&els, &bonds, &[]);
    assert_eq!(
        sel_json(&serde_json::json!({ "molfile": to_v3000(&two) }).to_string()).unwrap(),
        sel_json(&serde_json::json!({ "molfile": two }).to_string()).unwrap()
    );
}

#[test]
fn above_999_atoms_mzlab_mol_reads_the_file_and_the_graph_refuses_its_size() {
    let big = chain_v3000(1200);
    let m = mzlab_mol::Mol::from_molfile(&big).unwrap();
    assert_eq!((m.atoms.len(), m.bonds.len()), (1200, 1199));
    assert_eq!(m.atoms.iter().map(|a| u32::from(a.h)).sum::<u32>(), 2402);
    let e = sel_json(&serde_json::json!({ "molfile": big }).to_string()).unwrap_err();
    assert_eq!(e.key, "error.grafo.tooBig");
}

#[test]
fn a_damaged_v3000_file_is_refused_with_its_reason() {
    let ok = to_v3000(&mf(&["C", "C", "O"], &[(1, 2, 1), (2, 3, 1)], &[]));
    let cases = [
        // the counts line says more than the blocks hold: a file cut short
        (ok.replace("M  V30 2 1 2 3\n", ""), "error.grafo.molfile"),
        (
            ok.replace("COUNTS 3 2", "COUNTS 4 2"),
            "error.grafo.molfile",
        ),
        // a bond to an atom the file does not have
        (
            ok.replace("M  V30 2 1 2 3", "M  V30 2 1 2 9"),
            "error.grafo.molfile",
        ),
        // no atoms at all
        (
            ok.replace("M  V30 BEGIN ATOM", "M  V30 BEGIN OTHER")
                .replace("M  V30 END ATOM", "M  V30 END OTHER"),
            "error.grafo.molfile",
        ),
        // not an element
        (ok.replace(" O ", " Qq "), "error.grafo.element"),
    ];
    for (text, key) in cases {
        let req = serde_json::json!({ "molfile": text }).to_string();
        assert_eq!(sel_json(&req).unwrap_err().key, key, "{text}");
        assert_eq!(
            fragments_json(&text, -1, -1).unwrap_err().key,
            key,
            "{text}"
        );
    }
}
