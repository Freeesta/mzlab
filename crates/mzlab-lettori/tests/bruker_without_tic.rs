//! The routes that do not read the TIC of the index rows (probe, mzML, conv/1) skip the decoding of every PASEF frame: same mzML byte for byte,
//! and the full index asked afterwards still carries the TIC summed from the points (synthetic DDA-PASEF folder, no lab data).
#![cfg(feature = "bruker")]

use mzlab_core::lettori::{FileSet, Limits, Member, Reader};

mod common_sqlite;

fn open() -> Box<dyn Reader> {
    let (tdf, bin) = common_sqlite::pasef::folder(3, 3000, 800);
    let m = |n: &str, b: Vec<u8>| Member {
        nome: format!("x.d/{n}"),
        dim: b.len() as u64,
        dati: Box::new(b),
        sha1: None,
    };
    let fs = FileSet {
        membri: vec![m("analysis.tdf", tdf), m("analysis.tdf_bin", bin)],
    };
    mzlab_lettori::apri(fs, &Limits::desktop()).unwrap()
}

fn mzml(l: &mut dyn Reader) -> Vec<u8> {
    let mut out = Vec::new();
    mzlab_core::lettori_mzml::scrivi(l, 0, &mut out, &mut |_, _| {}).unwrap();
    out
}

#[test]
fn mzml_equal_with_and_without_the_ms2_tic() {
    let without = mzml(&mut *open());
    let mut l = open();
    assert!(l
        .index(0)
        .unwrap()
        .iter()
        .any(|v| v.livello > 1 && v.tic > 0.0));
    let con = mzml(&mut *l);
    assert!(without.len() > 10_000);
    assert!(
        without == con,
        "mzML differs: {} vs {} bytes",
        without.len(),
        con.len()
    );
}

#[test]
fn index_after_the_light_rows_has_the_ms2_tic() {
    let full = open().index(0).unwrap().to_vec();
    let mut l = open();
    assert!(!mzlab_lettori::srm::has_srm_scans(&mut *l, 0));
    let to_read = l.index_without_tic(0).unwrap().to_vec();
    assert_eq!(to_read.len(), full.len());
    assert!(to_read
        .iter()
        .zip(&full)
        .all(|(a, b)| a.id == b.id && a.rt == b.rt && a.livello == b.livello));
    let after = l.index(0).unwrap().to_vec();
    for (k, (a, b)) in after.iter().zip(&full).enumerate() {
        assert_eq!(a.tic, b.tic, "#{k}");
        let s = l.scan(0, k).unwrap();
        let summed: f64 = (0..s.mz.len()).map(|i| s.intensity_f64(i)).sum();
        if s.level > 1 {
            assert_eq!(summed, b.tic, "#{k}");
        }
    }
}
