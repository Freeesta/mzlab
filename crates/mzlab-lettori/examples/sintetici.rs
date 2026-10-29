//! Writes the synthetic Thermo, Bruker and ANDI datasets of the reader tests (invented numbers, no lab data) to a folder, so that
//! tests/test_lettori_parita.py can feed the same bytes to the .wasm modules and to `mzlab-cli convert`. A test tool: it never enters a .wasm.
//!   sintetici OUTDIR
use mzlab_core::lettori::{FileSet, Membro};
use std::path::Path;

#[path = "../tests/comune/andi.rs"]
mod andi;
#[path = "../tests/comune_sqlite/mod.rs"]
mod comune_sqlite;
#[path = "../tests/comune/thermo.rs"]
mod thermo;

/// The helper the shared ANDI module expects from its parent.
fn insieme(nome: &str, byte: Vec<u8>) -> FileSet {
    FileSet {
        membri: vec![Membro {
            nome: nome.into(),
            dim: byte.len() as u64,
            dati: Box::new(byte),
            sha1: None,
        }],
    }
}

fn main() {
    let out = std::env::args().nth(1).expect("usage: sintetici OUTDIR");
    let out = Path::new(&out);
    let w = |rel: &str, b: Vec<u8>| {
        let p = out.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b).unwrap();
    };
    w("thermo/synt.raw", thermo::Raw::tsq(63, 12).bytes());
    w("bruker/synt.d/analysis.tdf", comune_sqlite::tdf_minimo(3));
    w("bruker/synt.d/analysis.tdf_bin", comune_sqlite::blocco2());
    w("andi/synt.cdf", andi::andi_ms(&andi::PIATTA));
}
