//! What a plan leaves to the files at unit resolution: `qc.mass_accuracy` refuses them (a ppm error means nothing on a 0.1 Da axis) and the record of `features.detect`
//! names the preset the files decided. Child of `flow_neglr_tests.rs`, whose series and plans it reuses.

use super::{activity, error_of, fixture, run, with, CASES};

const QC: &str = r#",{"id":"qc","step":"qc.mass_accuracy@1","in":{"raw":["idx.index"]}}]}"#;

#[test]
fn the_ppm_error_is_refused_at_unit_resolution_and_the_record_names_the_preset() {
    for (tag, pol, hr) in CASES {
        let d = fixture(&format!("nl_m2_{tag}"), pol, hr);
        let text = with(QC);
        // the second run takes the detection from the cache and keeps the preset
        for round in ["first", "cached"] {
            let prov = run(&d, &text);
            let want = (!hr).then_some("err.flow.massacc.unit");
            assert_eq!(error_of(&prov, "qc").as_deref(), want, "{tag} {round} qc");
            let preset = activity(&prov, "det").get("preset").and_then(|v| v.str());
            assert_eq!(preset, Some(["lr", "hr"][usize::from(hr)]), "{tag} {round}");
        }
        let _ = std::fs::remove_dir_all(&d);
    }
}
