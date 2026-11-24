//! The canonical form of drawings with many equal components is unchanged by the faster orbit pruning: `can()`, the canonical record
//! and the SMILES of the synthetic set (`canon_set`) match `data/golden_canon.json` (written by `examples/golden_canon.rs`; the search
//! before the change gives the same bytes), and any writing order of the same components gives the same canonical record.

mod canon_set;

use mzlab_mol::{hex, Mol};
use sha2::{Digest, Sha256};

/// `[smiles, can(), sha256 of canonical().to_json(), sha256 of to_smiles()]`, from `cargo run --release -p mzlab-mol --example golden_canon`.
const GOLDEN: &str = include_str!("data/golden_canon.json");

fn sha(text: &str) -> String {
    hex(&Sha256::digest(text.as_bytes()))
}

#[test]
fn canonical_form_of_the_synthetic_set_is_unchanged() {
    let golden: Vec<[String; 4]> = serde_json::from_str(GOLDEN).expect("golden_canon.json");
    let set = canon_set::smiles();
    assert_eq!(
        golden.len(),
        set.len(),
        "golden and set have the same entries"
    );
    for ([smiles, can, record, written], want) in golden.iter().zip(&set) {
        assert_eq!(smiles, want, "golden follows the set order");
        let m = Mol::from_smiles(smiles).expect(smiles);
        assert_eq!(&m.can(), can, "{smiles} can");
        assert_eq!(&sha(&m.canonical().to_json()), record, "{smiles} record");
        assert_eq!(
            &sha(&m.to_smiles().expect(smiles)),
            written,
            "{smiles} smiles"
        );
    }
}

#[test]
fn the_order_of_the_components_does_not_change_the_canonical_record() {
    let parts = [
        "c1ccccc1", "CCO", "O", "[Na+]", "[Cl-]", "c1ccncc1", "OCC", "O",
    ];
    let want = Mol::from_smiles(&parts.join("."))
        .unwrap()
        .canonical()
        .core_json();
    // every rotation and the reverse of every rotation
    for r in 0..parts.len() {
        let mut p = parts.to_vec();
        p.rotate_left(r);
        for q in [p.clone(), p.into_iter().rev().collect()] {
            let m = Mol::from_smiles(&q.join(".")).unwrap();
            assert_eq!(m.canonical().core_json(), want, "{}", q.join("."));
        }
    }
}

/// Writings of one constitution with different stereo: enantiomers, defined and unknown, cis and trans, chiral and meso.
const STEREO_PAIRS: &[(&str, &str)] = &[
    ("N[C@@H](C)C(=O)O", "N[C@H](C)C(=O)O"),
    ("N[C@@H](C)C(=O)O", "NC(C)C(=O)O"),
    ("C/C=C/C", "C/C=C\\C"),
    ("C[C@H](O)[C@@H](O)C", "C[C@H](O)[C@H](O)C"),
];

/// A permutation of `0..n` from a seed (Fisher-Yates over a small LCG).
fn shuffled(n: usize, mut seed: u64) -> Vec<u32> {
    let mut p: Vec<u32> = (0..n as u32).collect();
    for i in (1..n).rev() {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        p.swap(i, (seed >> 33) as usize % (i + 1));
    }
    p
}

/// The same canonical record and `can()` for the record and four shuffles of its atoms.
fn same_in_any_order(m: &Mol, what: &str) {
    let want = m.canonical().core_json();
    let can = m.can();
    for seed in 1..=4 {
        let s = m
            .renumbered(&shuffled(m.atoms.len(), seed))
            .expect("a permutation");
        assert_eq!(s.canonical().core_json(), want, "{what} seed {seed}");
        assert_eq!(s.can(), can, "{what} seed {seed}");
    }
}

#[test]
fn equal_pieces_with_different_stereo_give_one_record_in_any_atom_order() {
    // 12 diols (chiral and meso) gave 5 different `can` over 6 atom orders when the search stopped at its leaf limit
    for &(a, b) in STEREO_PAIRS {
        for k in [2usize, 3, 5, 8, 12, 13, 16, 40, 200] {
            let parts: Vec<&str> = (0..k).map(|i| if i % 3 == 1 { b } else { a }).collect();
            let m = Mol::from_smiles(&parts.join(".")).expect("a synthetic SMILES");
            same_in_any_order(&m, &format!("{a} / {b} x{k}"));
        }
    }
}

#[test]
fn groups_across_pieces_keep_their_pieces_together() {
    // the R and S pieces alternate; one AND group joins the centres of two copies, an OR group holds one alone
    for k in [4usize, 13, 40] {
        let parts: Vec<&str> = (0..k)
            .map(|i| {
                if i % 2 == 0 {
                    "C[C@H](O)CC"
                } else {
                    "C[C@@H](O)CC"
                }
            })
            .collect();
        let text = format!("{} |&1:1,11,o1:6|", parts.join("."));
        let m = Mol::from_smiles(&text).expect(&text);
        assert_eq!(m.groups.len(), 2, "{text}");
        same_in_any_order(&m, &text);
    }
}
