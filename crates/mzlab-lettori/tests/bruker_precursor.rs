//! The precursor of a DDA-PASEF spectrum in the mzML: the selected ion is the monoisotopic m/z of `Precursors`, the isolation target the centre of the
//! window the instrument isolated (`IsolationMz`), the charge of `Precursors` and the collision energy and half-width of the window are written (synthetic folder, no lab data).
#![cfg(feature = "bruker")]

use mzlab_core::readers::{FileSet, Limits, Member};

mod common_sqlite;

fn cv(block: &str, acc: &str) -> Option<f64> {
    let i = block.find(&format!("accession=\"{acc}\""))?;
    let rest = &block[i..];
    let v = rest.find("value=\"")? + 7;
    rest[v..v + rest[v..].find('"')?].parse().ok()
}

#[test]
fn isolation_target_is_isolation_mz_and_charge_is_written() {
    let (tdf, bin) = common_sqlite::pasef::folder(1, 200, 100);
    let m = |n: &str, b: Vec<u8>| Member {
        nome: format!("x.d/{n}"),
        dim: b.len() as u64,
        dati: Box::new(b),
        sha1: None,
    };
    let fs = FileSet {
        membri: vec![m("analysis.tdf", tdf), m("analysis.tdf_bin", bin)],
    };
    let mut l = mzlab_lettori::apri(fs, &Limits::desktop()).unwrap();
    let mut out = Vec::new();
    mzlab_core::readers_mzml::scrivi(&mut *l, 0, &mut out, &mut |_, _| {}).unwrap();
    let x = String::from_utf8(out).unwrap();
    let blocks: Vec<&str> = x.split("<precursor>").skip(1).collect();
    assert_eq!(blocks.len(), 120);
    for b in blocks {
        let b = &b[..b.find("</precursor>").unwrap()];
        let target = cv(b, "MS:1000827").expect("isolation target");
        let selected = cv(b, "MS:1000744").expect("selected ion");
        let d = target - selected - common_sqlite::pasef::ISO_OFFSET;
        assert!(d.abs() < 1e-9, "target {target} selected {selected}");
        assert_eq!(cv(b, "MS:1000041"), Some(2.0), "charge state");
        // the energy of `PasefFrameMsMsInfo` and half its width (2) on both sides of the target
        assert_eq!(cv(b, "MS:1000045"), Some(30.0), "collision energy");
        assert_eq!(cv(b, "MS:1000828"), Some(1.0), "lower offset");
        assert_eq!(cv(b, "MS:1000829"), Some(1.0), "upper offset");
    }
}
