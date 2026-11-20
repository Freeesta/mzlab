//! Unit tests of the aromatic hydrogens of a molfile (`aromatic_h.rs`): `graph::Mol` (selection, cut) and `mzlab_mol::Mol` (check, export)
//! give every ring the same hydrogens or the same refusal with the same reason.

use super::tests::{mf, sel};
use super::*;

/// The lines of an `MRV_IMPLICIT_H` data group `g` on `atom` with the data `IMPL_H<count>`.
fn group(g: usize, atom: usize, count: &str) -> [String; 4] {
    [
        format!("M  STY  1 {g:>3} DAT"),
        format!("M  SAL {g:>3}  1 {atom:>3}"),
        format!("M  SDT {g:>3} MRV_IMPLICIT_H"),
        format!("M  SED {g:>3} IMPL_H{count}"),
    ]
}

/// The hydrogens of every atom as the two readers give them: `graph::Mol` (selection, cut) and `mzlab_mol::Mol` (check, export).
fn both(text: &str) -> (Vec<u32>, Vec<u32>) {
    let feat = graph::Mol::from_molfile(text).unwrap();
    let mol = mzlab_mol::Mol::from_molfile(text).unwrap();
    (
        feat.atoms.iter().map(|a| a.h).collect(),
        mol.atoms.iter().map(|a| u32::from(a.h)).collect(),
    )
}

/// `mf` with one `MRV_IMPLICIT_H` group per `(atom, count)` after the lines `more`.
fn with(
    els: &[&str],
    bonds: &[(usize, usize, u8)],
    groups: &[(usize, &str)],
    more: &[&str],
) -> String {
    let g: Vec<String> = groups
        .iter()
        .enumerate()
        .flat_map(|(k, (atom, count))| group(k + 1, *atom, count))
        .collect();
    let mut lines: Vec<&str> = more.to_vec();
    lines.extend(g.iter().map(String::as_str));
    mf(els, bonds, &lines)
}

const PYRROLE: [&str; 5] = ["N", "C", "C", "C", "C"];

const RING5: [(usize, usize, u8); 5] = [(1, 2, 4), (2, 3, 4), (3, 4, 4), (4, 5, 4), (5, 1, 4)];

#[test]
fn a_hydrogen_count_no_atom_carries_is_refused() {
    for count in ["5", "4294967295", "99999999999999999999", "-1", "x"] {
        let g = group(1, 1, count);
        let lines: Vec<&str> = g.iter().map(String::as_str).collect();
        // on a ring and on a chain alike
        for text in [
            mf(&PYRROLE, &RING5, &lines),
            mf(&["C", "C"], &[(1, 2, 1)], &lines),
        ] {
            // the same refusal with the same evidence (the value as written), also for a selected part
            let reason = mzlab_mol::Mol::from_molfile(&text)
                .err()
                .unwrap()
                .to_string();
            assert!(
                reason.contains(&format!("IMPL_H{count}")) && reason.contains("is not 0 to 4"),
                "{reason}"
            );
            let Err(e) = graph::Mol::from_molfile(&text) else {
                panic!("{count}: read")
            };
            assert_eq!(e.key, "error.grafo.molfile", "{count}");
            assert!(e.to_string().contains(&reason), "{e} / {reason}");
        }
    }
}

#[test]
fn the_two_readers_give_a_stated_hydrogen_the_same_meaning() {
    // the N of pyrrole: one H whatever the count says above it, and from the larger of two groups
    for groups in [
        &[(1, "1")][..],
        &[(1, "2")],
        &[(1, "4")],
        &[(1, "0"), (1, "1")],
    ] {
        let (feat, mol) = both(&with(&PYRROLE, &RING5, groups, &[]));
        assert_eq!(feat, [1, 1, 1, 1, 1], "{groups:?}");
        assert_eq!(mol, feat, "{groups:?}");
    }
    // a count on an aromatic CH that already has its H, or a zero, changes nothing
    let (feat, mol) = both(&with(
        &PYRROLE,
        &RING5,
        &[(1, "1"), (2, "1"), (3, "0")],
        &[],
    ));
    assert_eq!((feat.clone(), mol), (vec![1, 1, 1, 1, 1], feat));
    // both tautomers of 4-methylimidazole (CH3, C, CH, N, CH, N)
    for (atom, h) in [(4, [3, 0, 1, 1, 1, 0]), (6, [3, 0, 1, 0, 1, 1])] {
        let (feat, mol) = both(&with(&IMIDAZOLE, &IMIDAZOLE_BONDS, &[(atom, "1")], &[]));
        assert_eq!((feat, mol), (h.to_vec(), h.to_vec()));
    }
    // outside an aromatic ring a count is not read: the methyl radical keeps its three H (not the two a count of 3 minus the radical would give), ethane its six
    let (feat, mol) = both(&with(&["C"], &[], &[(1, "3")], &["M  RAD  1   1   2"]));
    assert_eq!((feat, mol), (vec![3], vec![3]));
    let (feat, mol) = both(&with(&["C", "C"], &[(1, 2, 1)], &[(1, "1"), (2, "4")], &[]));
    assert_eq!((feat, mol), (vec![3, 3], vec![3, 3]));
}

const IMIDAZOLE: [&str; 6] = ["C", "C", "C", "N", "C", "N"];
const IMIDAZOLE_BONDS: [(usize, usize, u8); 6] = [
    (1, 2, 1),
    (2, 3, 4),
    (3, 4, 4),
    (4, 5, 4),
    (5, 6, 4),
    (6, 2, 4),
];
const RING6: [(usize, usize, u8); 6] = [
    (1, 2, 4),
    (2, 3, 4),
    (3, 4, 4),
    (4, 5, 4),
    (5, 6, 4),
    (6, 1, 4),
];

/// The hydrogens `graph::Mol` gives the atoms of `text`.
fn hs(text: &str) -> Vec<u32> {
    let m = graph::Mol::from_molfile(text).unwrap();
    m.atoms.iter().map(|a| a.h).collect()
}

#[test]
fn a_ring_that_cannot_be_kekulised_is_read_by_valence_and_never_refused() {
    let pyridine = ["N", "C", "C", "C", "C", "C"];
    for (name, text, h) in [
        (
            "pyridine N with IMPL_H1",
            with(&pyridine, &RING6, &[(1, "1")], &[]),
            vec![1, 1, 1, 1, 1, 1],
        ),
        (
            "imidazole with IMPL_H1 on both N",
            with(&IMIDAZOLE, &IMIDAZOLE_BONDS, &[(4, "1"), (6, "1")], &[]),
            vec![3, 0, 1, 1, 1, 1],
        ),
        (
            "benzene C with IMPL_H2",
            with(&["C"; 6], &RING6, &[(1, "2")], &[]),
            vec![2, 1, 1, 1, 1, 1],
        ),
        (
            "cyclopentadiene written aromatic",
            mf(&["C"; 5], &RING5, &[]),
            vec![1, 1, 1, 1, 1],
        ),
    ] {
        // `mzlab-mol` strict refuses, its lenient reading (`check/1`) warns `kekule`; selection, cut and fragments read on
        assert!(mzlab_mol::Mol::from_molfile(&text).is_err(), "{name}");
        assert_eq!(hs(&text), h, "{name}");
        let req = serde_json::json!({ "molfile": text, "part": false }).to_string();
        assert!(sel_json(&req).is_ok(), "{name}");
        assert!(
            crate::chem::fragments::fragments_json(&text, -1, -1).is_ok(),
            "{name}"
        );
    }
}

#[test]
fn what_a_cut_leaves_of_an_aromatic_ring_is_read_again() {
    // 3-methylpyrrole (N1 with its stated H) and 3-methylfuran after the cut of C2-C3: the other type 4 bonds stay, C2 and C3 are radicals
    let open = [(1, 2, 4), (3, 4, 4), (4, 5, 4), (5, 1, 4), (4, 6, 1)];
    let rad = ["M  RAD  2   2   2   3   2"];
    let pyrrole = with(&["N", "C", "C", "C", "C", "C"], &open, &[(1, "1")], &rad);
    assert_eq!(hs(&pyrrole), [1, 1, 1, 0, 1, 3]);
    assert_eq!(sel(&pyrrole, false)["label"]["text"], "C5H7N");
    assert_eq!(sel(&pyrrole, false)["label"]["unpaired"], 2);
    let furan = mf(&["O", "C", "C", "C", "C", "C"], &open, &rad);
    assert_eq!(hs(&furan), [0, 1, 1, 0, 1, 3]);
    for text in [&pyrrole, &furan] {
        // a second cut (the methyl) and the fragments of the piece
        let req = serde_json::json!({ "molfile": text, "bonds": [[3, 5]], "mode": "homolytic" })
            .to_string();
        let cut = crate::chem::cut::cut_json(&req);
        assert!(cut.is_ok(), "{:?}", cut.err().map(|e| e.to_string()));
        assert!(crate::chem::fragments::fragments_json(text, -1, -1).is_ok());
    }
}

#[test]
fn a_selected_part_of_an_aromatic_ring_follows_one_rule() {
    // pyridine without C4 and pyridine without C3: both are open pieces, both read by valence (N without H, the ends with two)
    let without_c4 = mf(
        &["N", "C", "C", "C", "C"],
        &[(3, 2, 4), (2, 1, 4), (1, 5, 4), (5, 4, 4)],
        &[],
    );
    assert_eq!(hs(&without_c4), [0, 1, 2, 2, 1]);
    assert_eq!(sel(&without_c4, true)["label"]["text"], "C4H6N");
    let without_c3 = mf(
        &["N", "C", "C", "C", "C"],
        &[(2, 1, 4), (1, 5, 4), (5, 4, 4), (4, 3, 4)],
        &[],
    );
    assert_eq!(hs(&without_c3), [0, 2, 2, 1, 1]);
    assert_eq!(sel(&without_c3, true)["label"]["text"], "C4H6N");
    // three atoms of pyrrole (N with its stated H, two C)
    let chain = with(&["C", "N", "C"], &[(1, 2, 4), (2, 3, 4)], &[(2, "1")], &[]);
    assert_eq!(hs(&chain), [2, 1, 2]);
    assert_eq!(sel(&chain, true)["label"]["text"], "C2H5N");
    // a part and a drawing are read the same way: the whole ring, and the open piece
    let ring = with(&PYRROLE, &RING5, &[(1, "1")], &[]);
    for text in [&ring, &without_c4, &chain] {
        assert_eq!(sel(text, true)["label"], sel(text, false)["label"]);
    }
}

/// `text` with the charge code `code` in the atom line of atom `atom` (1-based).
fn charge_code(text: &str, atom: usize, code: u8) -> String {
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    lines[3 + atom].replace_range(36..39, &format!("{code:>3}"));
    lines.join("\n")
}

#[test]
fn the_two_readers_read_charges_property_lines_and_bond_types_the_same() {
    let pyridine = ["N", "C", "C", "C", "C", "C"];
    let plain = mf(&pyridine, &RING6, &[]);
    // pyridinium written only in the charge column of the atom line (code 3 = +1)
    let column = charge_code(&plain, 1, 3);
    assert_eq!(
        both(&column),
        (vec![1, 1, 1, 1, 1, 1], vec![1, 1, 1, 1, 1, 1])
    );
    assert_eq!(sel(&column, false)["label"]["text"], "C5H6N+");
    // an `M  CHG` line anywhere takes the place of every charge of the column
    let both_ways = mf(&pyridine, &RING6, &["M  CHG  1   2   0"]);
    let (feat, mol) = both(&charge_code(&both_ways, 1, 3));
    assert_eq!(
        (feat, mol),
        (vec![0, 1, 1, 1, 1, 1], vec![0, 1, 1, 1, 1, 1])
    );
    // pyridine N-oxide in the charge column: N+ (code 3), O- (code 5)
    let mut bonds = RING6.to_vec();
    bonds.push((1, 7, 1));
    let oxide = mf(&["N", "C", "C", "C", "C", "C", "O"], &bonds, &[]);
    let oxide = charge_code(&charge_code(&oxide, 1, 3), 7, 5);
    assert_eq!(
        both(&oxide),
        (vec![0, 1, 1, 1, 1, 1, 0], vec![0, 1, 1, 1, 1, 1, 0])
    );
    // a line after the end line, a line without the property prefix, a pair beyond the count of its line: none is read
    for text in [
        format!("{plain}\nM  CHG  1   1   1"),
        mf(&pyridine, &RING6, &["X  CHG  1   1   1"]),
        mf(&pyridine, &RING6, &["M  CHG  1   2   0   1   1"]),
    ] {
        assert_eq!(
            both(&text),
            (vec![0, 1, 1, 1, 1, 1], vec![0, 1, 1, 1, 1, 1]),
            "{text}"
        );
        assert_eq!(sel(&text, false)["label"]["text"], "C5H5N");
    }
    // the valence column on an aromatic atom (15 = no hydrogens on a ring carbon bearing nothing else: a carbene-like centre)
    let mut lines: Vec<String> = plain.lines().map(String::from).collect();
    lines[5].replace_range(48..51, "  3");
    let stated = lines.join("\n");
    let (feat, mol) = both(&stated);
    assert_eq!(feat, mol, "valence column");
    // a query bond type (8 = any) is a single bond in both lenient readings: ethane with it has six H
    let any = mf(&["C", "C"], &[(1, 2, 8)], &[]);
    assert_eq!(hs(&any), [3, 3]);
    let lenient = mzlab_mol::Mol::drawing_from_molfile(&any).unwrap().mol;
    assert_eq!(
        lenient.atoms.iter().map(|a| a.h).collect::<Vec<_>>(),
        [3, 3]
    );
}

#[test]
fn of_two_damaged_groups_the_same_one_is_named_every_time() {
    let mut lines: Vec<String> = group(1, 1, "5").to_vec();
    lines.extend(group(2, 2, "7"));
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    let text = mf(&PYRROLE, &RING5, &lines);
    for _ in 0..20 {
        let e = graph::Mol::from_molfile(&text)
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(e.contains("IMPL_H5"), "{e}");
        let e = mzlab_mol::Mol::from_molfile(&text)
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default();
        assert!(e.contains("IMPL_H5"), "{e}");
    }
}

#[test]
fn a_bond_from_an_atom_to_itself_is_named_as_such() {
    let mut bonds = RING5.to_vec();
    bonds.push((1, 1, 1));
    let text = mf(&PYRROLE, &bonds, &[]);
    let e = mzlab_mol::Mol::from_molfile(&text)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    assert!(
        e.contains("joins atom 1 to itself") && !e.contains("outside"),
        "{e}"
    );
    let atoms: Vec<mzlab_mol::DrawnAtom> = PYRROLE
        .iter()
        .map(|el| mzlab_mol::DrawnAtom {
            el,
            chg: 0,
            rad: 0,
            valence: None,
        })
        .collect();
    let e = mzlab_mol::drawn_hydrogens(&atoms, &[(0, 0, 1)], Vec::new())
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    assert!(e.contains("joins atom 1 to itself"), "{e}");
}

#[test]
fn a_ring_the_file_states_no_hydrogen_for_is_read_the_same_by_both_readers() {
    let g = group(1, 1, "1");
    let lines: Vec<&str> = g.iter().map(String::as_str).collect();
    let stated = mf(&PYRROLE, &RING5, &lines);
    for (name, text) in [
        ("no group", mf(&PYRROLE, &RING5, &[])),
        (
            "a group after the end line",
            stated.replacen("M  STY", "M  END\nM  STY", 1),
        ),
        (
            "lines without the property prefix",
            stated.replace("M  S", "X  S"),
        ),
        ("IMPL_H0", with(&PYRROLE, &RING5, &[(1, "0")], &[])),
        (
            "a group that names an atom outside the file",
            with(&PYRROLE, &RING5, &[(9, "1")], &[]),
        ),
        (
            "another data field",
            stated.replace("MRV_IMPLICIT_H", "MRV_OTHER"),
        ),
    ] {
        // the ring needs one H on the N to be kekulised: both readers put it there
        let (feat, mol) = both(&text);
        assert_eq!(
            (feat, mol),
            (vec![1, 1, 1, 1, 1], vec![1, 1, 1, 1, 1]),
            "{name}"
        );
        assert_eq!(sel(&text, false)["label"]["text"], "C4H5N", "{name}");
    }
    // the rings that need no stated hydrogen keep theirs: pyridine, furan, pyridinium, the phenyl radical
    let pyridine = ["N", "C", "C", "C", "C", "C"];
    for (els, bonds, more, h) in [
        (&pyridine[..], &RING6[..], &[][..], vec![0, 1, 1, 1, 1, 1]),
        (&["O", "C", "C", "C", "C"], &RING5, &[], vec![0, 1, 1, 1, 1]),
        (
            &pyridine,
            &RING6,
            &["M  CHG  1   1   1"],
            vec![1, 1, 1, 1, 1, 1],
        ),
        (
            &["C"; 6],
            &RING6,
            &["M  RAD  1   1   2"],
            vec![0, 1, 1, 1, 1, 1],
        ),
    ] {
        let (feat, mol) = both(&mf(els, bonds, more));
        assert_eq!((feat, mol), (h.clone(), h), "{els:?} {more:?}");
    }
}

#[test]
fn many_data_groups_are_read_in_one_pass() {
    // 20,000 groups (a file of more than 1 MB): each line is looked at once
    let g: Vec<String> = (1..=20_000).flat_map(|k| group(k, 1, "1")).collect();
    let lines: Vec<&str> = g.iter().map(String::as_str).collect();
    let text = mf(&PYRROLE, &RING5, &lines);
    let start = std::time::Instant::now();
    let m = graph::Mol::from_molfile(&text).unwrap();
    assert_eq!(m.atoms[0].h, 1);
    assert!(start.elapsed().as_secs() < 5, "{:?}", start.elapsed());
}
