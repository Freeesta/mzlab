//! The mark of a broad-band feature goes into the table of the file (`.mzfeatf`) and back, and is free of the quality reasons.
#![cfg(feature = "untargeted")]

use mzlab_feat::quality::{Q_POINTS, Q_SHAPE, Q_SNR, REASON_BROAD};
use mzlab_feat::untargeted::pipeline::{from_file_features, to_file_features};
use mzlab_feat::untargeted::Feature;

fn feature(broad: bool) -> Feature {
    Feature {
        mz: 300.0,
        rt: 5.0,
        rt_start: 4.0,
        rt_end: 6.0,
        height: 1e5,
        area: 1e4,
        broad,
        ..Feature::default()
    }
}

#[test]
fn the_mark_round_trips_through_the_table_of_the_file() {
    let f = to_file_features(&[feature(true), feature(false)]);
    assert_eq!(f.qreason, vec![REASON_BROAD, 0]);
    let back = from_file_features(&f);
    assert!(back[0].broad && !back[1].broad);
    assert_eq!(REASON_BROAD & (Q_SHAPE | Q_SNR | Q_POINTS), 0);
}

#[test]
fn the_untargeted_recipe_switches_the_broad_pass_in_the_resolve_block() {
    use mzlab_feat::untargeted::pipeline::UntParams;
    assert!(!UntParams::from_json("").unwrap().broad.enabled);
    assert!(
        !UntParams::from_json(r#"{"resolve":{"min_ratio":1.5}}"#)
            .unwrap()
            .broad
            .enabled
    );
    assert!(
        UntParams::from_json(r#"{"resolve":{"broad":true}}"#)
            .unwrap()
            .broad
            .enabled
    );
    assert!(UntParams::from_json(r#"{"resolve":{"broad":3}}"#).is_err());
}
