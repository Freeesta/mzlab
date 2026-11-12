//! Writes the synthetic Thermo, Bruker and ANDI datasets of the reader tests (invented numbers, no lab data) to a folder, so that
//! tests/test_lettori_parita.py can feed the same bytes to the .wasm modules and to `mzlab-cli convert`. A test tool: it never enters a .wasm.
//!   sintetici OUTDIR
use mzlab_core::readers::{FileSet, Member};
use std::path::Path;

#[path = "../tests/common/andi.rs"]
mod andi;
#[path = "../tests/common_sqlite/mod.rs"]
mod common_sqlite;
#[path = "../tests/common/thermo.rs"]
mod thermo;

/// The helper the shared ANDI module expects from its parent.
fn file_set(name: &str, byte: Vec<u8>) -> FileSet {
    FileSet {
        membri: vec![Member {
            nome: name.into(),
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
    // one of every damaged unit: a controller before the MS one and an analog one that cannot be read, a scan pointing past the end, a record that does
    // not decode; every other scan and the sound analog channel are read
    let mut d = thermo::Raw::tsq(63, 12);
    (d.stray_first, d.analog) = (true, vec![true, false]);
    (d.entry_past_the_end, d.entry_too_big) = (Some(4), Some(7));
    w("thermo_danneggiato/danneggiato.raw", d.bytes());
    w("bruker/synt.d/analysis.tdf", common_sqlite::minimal_tdf(3));
    w("bruker/synt.d/analysis.tdf_bin", common_sqlite::blocco2());
    // five frames, the copy of `tdf_bin` cut inside the fourth block: three spectra read, two counted as missing
    let b = common_sqlite::blocco2();
    let n = b.len() as i64;
    let tdf = common_sqlite::tdf_frames(&[0, n, 2 * n, 3 * n, 4 * n], 3);
    w("bruker_tagliato/tagliato.d/analysis.tdf", tdf);
    let bin: Vec<u8> = b.iter().copied().cycle().take(3 * b.len() + 9).collect();
    w("bruker_tagliato/tagliato.d/analysis.tdf_bin", bin);
    w("andi/synt.cdf", andi::andi_ms(&andi::FLAT_RUN));
}
