//! The routes that do not read the TIC of the index rows (probe, mzML, conv/1) skip the decoding of every PASEF frame: same mzML byte for byte,
//! and the full index asked afterwards still carries the TIC summed from the points (synthetic DDA-PASEF folder, no lab data).
#![cfg(feature = "bruker")]

use mzlab_core::lettori::{FileSet, Lettore, Limiti, Membro};

mod comune_sqlite;

fn apri() -> Box<dyn Lettore> {
    let (tdf, bin) = comune_sqlite::pasef::cartella(3, 3000, 800);
    let m = |n: &str, b: Vec<u8>| Membro {
        nome: format!("x.d/{n}"),
        dim: b.len() as u64,
        dati: Box::new(b),
        sha1: None,
    };
    let fs = FileSet {
        membri: vec![m("analysis.tdf", tdf), m("analysis.tdf_bin", bin)],
    };
    mzlab_lettori::apri(fs, &Limiti::desktop()).unwrap()
}

fn mzml(l: &mut dyn Lettore) -> Vec<u8> {
    let mut out = Vec::new();
    mzlab_core::lettori_mzml::scrivi(l, 0, &mut out, &mut |_, _| {}).unwrap();
    out
}

#[test]
fn mzml_uguale_con_e_senza_il_tic_delle_ms2() {
    let senza = mzml(&mut *apri());
    let mut l = apri();
    assert!(l
        .index(0)
        .unwrap()
        .iter()
        .any(|v| v.livello > 1 && v.tic > 0.0));
    let con = mzml(&mut *l);
    assert!(senza.len() > 10_000);
    assert!(
        senza == con,
        "mzML differs: {} vs {} bytes",
        senza.len(),
        con.len()
    );
}

#[test]
fn indice_dopo_le_righe_leggere_ha_il_tic_delle_ms2() {
    let pieno = apri().index(0).unwrap().to_vec();
    let mut l = apri();
    assert!(!mzlab_lettori::srm::ha_scansioni_srm(&mut *l, 0));
    let leggere = l.index_without_tic(0).unwrap().to_vec();
    assert_eq!(leggere.len(), pieno.len());
    assert!(leggere
        .iter()
        .zip(&pieno)
        .all(|(a, b)| a.id == b.id && a.rt == b.rt && a.livello == b.livello));
    let dopo = l.index(0).unwrap().to_vec();
    for (k, (a, b)) in dopo.iter().zip(&pieno).enumerate() {
        assert_eq!(a.tic, b.tic, "#{k}");
        let s = l.scan(0, k).unwrap();
        let summed: f64 = (0..s.mz.len()).map(|i| s.intensity_f64(i)).sum();
        if s.level > 1 {
            assert_eq!(summed, b.tic, "#{k}");
        }
    }
}
