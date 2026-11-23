//! The sample-ID convention on synthetic IDs: underscores read like dashes, dark controls, dark matrix blanks, washes, standards and QC.

use super::{apply, read};
use crate::samples::Sample;

#[test]
fn underscores_read_like_dashes() {
    for (a, b) in [
        ("MQ-H2O2-T30", "MQ_H2O2_T30"),
        ("TW-PMS-T2P5", "TW_PMS_T2P5"),
        ("MQ-STD05", "MQ_STD05"),
        ("TW-BLKD-T00", "TW_BLKD_T00"),
        ("MQ-H2O2-T30-R", "MQ_H2O2_T30_R"),
    ] {
        assert_eq!(read(a), read(b), "{a} / {b}");
    }
    let i = read("MQ_H2O2_T30");
    assert_eq!(
        (i.kind, i.matrix, i.cond.as_deref(), i.time_min),
        (Some("sample"), Some("MQ"), Some("H2O2"), Some(30.0))
    );
    assert!(read("MQ_H2O2_T30_R").reinjection);
}

#[test]
fn dark_controls_and_dark_blanks_are_dark_not_time_zero() {
    for id in [
        "MQ-H2O2D-T00",
        "MQ_H2O2D_T00",
        "MQ_PMSD_T60",
        "TW_BLKD_T00",
        "TW-BLKD-T00",
        "MQ_N2D_T10",
        "MQ_H2O2_D_T30",
        "MQ-H2O2-D-T30",
    ] {
        let i = read(id);
        assert_eq!(i.kind, Some("dark"), "{id}");
        assert!(!i.unknown_cond, "{id}");
    }
    // the apart `D` joins its condition
    assert_eq!(read("MQ_H2O2_D_T30").cond.as_deref(), Some("H2O2D"));
    // a dark control keeps its time (a time zero it is not): the role of the sample is `dark`
    let mut c = Sample::new("f", "", 1);
    c.sample_id = "TW_BLKD_T00".into();
    apply(&mut c, &[]);
    assert_eq!((c.kind.as_str(), c.role()), ("dark", "dark"));
    assert_eq!(c.time_min, Some(0.0));
}

#[test]
fn roles_std_qc_blank_and_wash_by_id() {
    assert_eq!(read("MQ_STD05").kind, Some("std"));
    assert_eq!(read("MQ_STD05").conc, Some(5.0));
    assert_eq!(read("QC_TW").kind, Some("qc"));
    assert_eq!(read("MQ_BLK").kind, Some("blank"));
    assert_eq!(read("BLK").kind, Some("blank"));
    for id in ["MQ_WASH", "MQ-WASH", "WASH"] {
        let i = read(id);
        assert_eq!(
            (i.kind, i.blank_kind),
            (Some("blank"), Some("wash")),
            "{id}"
        );
    }
    assert_eq!(read("P_mix_a").kind, Some("pool"));
    // an ID of no known shape says nothing
    assert_eq!(read("run_17"), Default::default());
    // a time that is not a number is flagged, an unknown condition too
    assert!(read("MQ_H2O2_Tx").bad_time);
    assert!(read("MQ_ZZZ_T10").unknown_cond);
}

#[test]
fn a_wash_id_sets_its_kind_of_blank_unless_the_model_captured_it() {
    let mut c = Sample::new("f", "", 1);
    c.sample_id = "MQ_WASH".into();
    apply(&mut c, &[]);
    assert_eq!(
        (c.kind.as_str(), c.blank_kind.as_deref()),
        ("blank", Some("wash"))
    );
    let mut c = Sample::new("f", "", 1);
    c.sample_id = "MQ_WASH".into();
    apply(&mut c, &["blank_kind"]);
    assert_eq!(c.blank_kind, None);
}
