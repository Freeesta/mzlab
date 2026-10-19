#![cfg(feature = "attributio")]
//! The SIRIUS input (`.ms` and MGF) of the synthetic campaign.

#[macro_use]
mod attributio_comune;
use attributio_comune::*;
use mzlab_feat::attributio::io::Io;

#[test]
fn sirius_export_of_the_synthetic_campaign() {
    use mzlab_feat::attributio::export::sirius::{collect, write_mgf, write_ms};
    let fx = fixture_overlay(&["campagna_sintetica", "campagna_esportazione"]);
    with_io!(fx, io);
    let mut warnings = Vec::new();
    let f = collect(&fx.campaign, &io, &mut warnings).expect("collect");
    // The only feature with an MS2 has no adduct: it is left out and the report says so.
    assert!(f.is_empty());
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("no adduct is known"));
    for x in &f {
        assert!(x.name.starts_with("feature_") && !x.ms2.is_empty());
    }
    let (ms, mgf) = (write_ms(&f), write_mgf(&f));
    assert_eq!(ms.matches(">compound ").count(), f.len());
    assert!(!ms.contains(".mzML") && !mgf.contains("FILENAME"));
}
