#![cfg(feature = "waters")]
//! A MassLynx folder built in the test (`comune_waters`): the scans, the MRM channels, the TIC and the method are read back to the values that were encoded.
mod comune_waters;
use comune_waters::*;
use mzlab_core::lettori::{Lettore, Limiti};

fn apri() -> Box<dyn Lettore> {
    mzlab_lettori::apri_nudo(insieme_sintetico(), &Limiti::sito())
        .unwrap_or_else(|e| panic!("{}", e.key))
}

#[test]
fn the_scans_read_back_to_the_encoded_peaks() {
    let mut l = apri();
    assert_eq!(l.formato().id(), "waters-raw");
    let ix = l.indice(0).unwrap().to_vec();
    assert_eq!(ix.len(), 3, "only the scanning function has spectra");
    for (k, v) in ix.iter().enumerate() {
        assert_eq!(v.livello, 1);
        assert!((v.rt - f64::from(RT[k])).abs() < 1e-6);
        assert_eq!(v.id, format!("function=1 process=0 scan={}", k + 1));
        let s = l.scansione(0, k).unwrap();
        let want = picchi(k as u32);
        assert_eq!(s.mz.len(), want.len());
        for (i, (mz, int)) in want.iter().enumerate() {
            assert!((s.mz[i] - mz).abs() < 1e-6, "m/z {} vs {mz}", s.mz[i]);
            assert!((f64::from(s.intensity[i]) - f64::from(*int)).abs() <= f64::from(*int) * 1e-6);
        }
        let tic: f64 = want.iter().map(|p| f64::from(p.1)).sum();
        assert!((v.tic - tic).abs() < 1e-3);
    }
}

#[test]
fn the_mrm_function_is_chromatograms_not_spectra() {
    let mut l = apri();
    let ch = l.cromatogrammi(0).unwrap();
    let ids: Vec<&str> = ch.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids[0], "TIC");
    assert_eq!(ids.len(), 3, "{ids:?}");
    for (j, (q1, q3)) in [("200.1", "100.2"), ("300.3", "150.4")].iter().enumerate() {
        assert_eq!(
            ids[1 + j],
            format!("SRM SIC Q1={q1} Q3={q3} function=2 offset={j}")
        );
        let c = l.cromatogramma(0, ids[1 + j]).unwrap();
        assert_eq!(c.time.len(), 3);
        for k in 0..3 {
            assert_eq!(c.intensity[k] as u32, canali_mrm(k as u32)[j]);
            assert!((c.time[k] - f64::from(RT[k])).abs() < 1e-6);
        }
    }
}

#[test]
fn the_method_and_the_header_carry_the_header_txt_and_hide_the_operator() {
    let mut l = apri();
    let m = l.metodo(0).unwrap();
    let doc = controlla_schema(&m, "synthetic");
    assert_eq!(doc["origine"]["formato"], "waters-raw");
    assert_eq!(doc["campione"]["nome"], "synthetic sample");
    assert_eq!(doc["campione"]["vial"], "1:34");
    assert!(m.contains("XEVO-TQXS"), "instrument");
    let h = l.testata_mzml(0).unwrap().unwrap();
    let doc = &h
        .run_params
        .iter()
        .find(|(k, _)| k == "mzLab metodo/1")
        .unwrap()
        .1;
    assert!(doc.contains("identita"), "operator marked as identity");
    assert!(
        h.sources.iter().any(|s| s.contains("_FUNC001.DAT")),
        "sources: {:?}",
        h.sources
    );
}
