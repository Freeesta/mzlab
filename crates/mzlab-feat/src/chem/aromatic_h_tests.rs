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
            for e in [
                graph::Mol::from_molfile(&text),
                graph::Mol::part_from_molfile(&text),
            ] {
                let e = e.err().unwrap();
                assert_eq!(e.key, "error.grafo.molfile", "{count}");
                assert!(e.to_string().contains(&reason), "{e} / {reason}");
            }
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

#[test]
fn a_ring_with_a_hydrogen_too_many_is_refused_by_both_readers_with_the_same_reason() {
    let benzene = ["C"; 6];
    let pyridine = ["N", "C", "C", "C", "C", "C"];
    for (name, text) in [
        (
            "pyridine N with IMPL_H1",
            with(&pyridine, &RING6, &[(1, "1")], &[]),
        ),
        (
            "imidazole with IMPL_H1 on both N",
            with(&IMIDAZOLE, &IMIDAZOLE_BONDS, &[(4, "1"), (6, "1")], &[]),
        ),
        (
            "benzene C with IMPL_H2",
            with(&benzene, &RING6, &[(1, "2")], &[]),
        ),
        (
            "pyrrole NH and a C with IMPL_H2",
            with(&PYRROLE, &RING5, &[(1, "1"), (2, "2")], &[]),
        ),
        (
            "cyclopentadiene written aromatic",
            mf(&["C"; 5], &RING5, &[]),
        ),
    ] {
        let Err(reason) = mzlab_mol::Mol::from_molfile(&text) else {
            panic!("{name}: read by mzlab-mol")
        };
        let reason = reason.to_string();
        assert!(reason.contains("cannot be kekulised"), "{name}: {reason}");
        let Err(e) = graph::Mol::from_molfile(&text) else {
            panic!("{name}: read by graph::Mol")
        };
        assert_eq!(e.key, "error.grafo.molfile", "{name}");
        assert!(e.to_string().contains(&reason), "{name}: {e}");
        let req = serde_json::json!({ "molfile": text, "part": false }).to_string();
        let Err(e) = sel_json(&req) else {
            panic!("{name}: read by selection/1")
        };
        assert!(e.to_string().contains(&reason), "{name}: {e}");
    }
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
fn a_selected_part_of_an_aromatic_ring_is_read_by_valence() {
    // three atoms of pyrrole (N with its stated H, two C): no ring to kekulise, so a whole drawing is refused and a part is read
    let chain = [(1, 2, 4), (2, 3, 4)];
    let text = with(&["C", "N", "C"], &chain, &[(2, "1")], &[]);
    assert!(graph::Mol::from_molfile(&text).is_err());
    let m = graph::Mol::part_from_molfile(&text).unwrap();
    assert_eq!(m.atoms.iter().map(|a| a.h).collect::<Vec<_>>(), [2, 1, 2]);
    assert_eq!(sel(&text, true)["label"]["text"], "C2H5N");
    // a part that is a whole ring is read as the drawing is
    let ring = with(&PYRROLE, &RING5, &[(1, "1")], &[]);
    assert_eq!(sel(&ring, true)["label"], sel(&ring, false)["label"]);
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
