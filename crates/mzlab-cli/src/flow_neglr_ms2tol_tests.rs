//! What a plan leaves to the files at unit resolution: the record of `features.detect` names the preset the files decided, and `qc.mass_accuracy` (a ppm error means nothing on
//! a 0.1 Da axis) is refused by the check of its port. Child of `flow_neglr_tests.rs`, whose series and plans it reuses.

use super::{activity, error_of, fixture, run, with, CASES};

const QC: &str = r#",{"id":"qc","step":"qc.mass_accuracy@1","in":{"raw":["idx.index"]}}]}"#;

#[test]
fn the_record_names_the_preset_the_files_decided_and_the_ppm_error_is_refused_at_unit_resolution() {
    for (tag, pol, hr) in CASES {
        let d = fixture(&format!("nl_m2_{tag}"), pol, hr);
        if hr {
            assert_eq!(error_of(&run(&d, &with(QC)), "qc"), None, "{tag} qc");
        } else {
            let e = crate::flow::run_plan(with(QC).as_bytes(), &d, &d.join("work"), 2, None)
                .expect_err("a ppm error on unit-resolution files");
            assert_eq!(
                e.params,
                vec![("problems", "qc.raw:err.flow.facet".into())],
                "{tag}"
            );
        }
        // the second run takes the detection from the cache and keeps the preset
        for round in ["first", "cached"] {
            let prov = run(&d, &with("]}"));
            let preset = activity(&prov, "det").get("preset").and_then(|v| v.str());
            assert_eq!(preset, Some(["lr", "hr"][usize::from(hr)]), "{tag} {round}");
            assert_eq!(error_of(&prov, "det"), None, "{tag} {round}");
        }
        let _ = std::fs::remove_dir_all(&d);
    }
}
