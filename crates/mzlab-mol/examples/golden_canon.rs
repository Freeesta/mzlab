//! Writes `tests/data/golden_canon.json`: for every record of the synthetic set (`tests/canon_set`), its SMILES, `can()`, the SHA-256
//! of `canonical().to_json()` and the SHA-256 of `to_smiles()`. Built against the search before the component automorphisms it gave the
//! same bytes (cubane x40 took 89 min there, the whole set takes seconds now).
//! Usage: `cargo run --release -p mzlab-mol --example golden_canon > crates/mzlab-mol/tests/data/golden_canon.json`

#[path = "../tests/canon_set/mod.rs"]
mod canon_set;

use mzlab_mol::{hex, Mol};
use sha2::{Digest, Sha256};

fn main() {
    let mut rows = Vec::new();
    for smiles in canon_set::smiles() {
        let m = Mol::from_smiles(&smiles).expect("a SMILES of the set");
        let record = hex(&Sha256::digest(m.canonical().to_json().as_bytes()));
        let written = hex(&Sha256::digest(m.to_smiles().expect("SMILES").as_bytes()));
        let row = serde_json::json!([smiles, m.can(), record, written]);
        rows.push(serde_json::to_string(&row).expect("a JSON row"));
    }
    println!("[\n{}\n]", rows.join(",\n"));
}
