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
