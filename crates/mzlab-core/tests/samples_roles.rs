//! The roles proposed from file names with the lab conventions (`MAT_COND_Ttt` with underscores or dashes, dark controls, dark matrix blanks, standards, QC,
//! blanks and washes): a dark control is never a time zero.

use mzlab_core::samples::{propose, Sample};

fn table(names: &[&str]) -> Vec<Sample> {
    let files: Vec<_> = names
        .iter()
        .map(|n| (n.to_string(), String::new(), String::new()))
        .collect();
    propose(&files)
}

#[test]
fn roles_of_the_lab_names() {
    #[rustfmt::skip]
    let rows: [(&str, &str, Option<f64>); 17] = [
        ("MQ-H2O2-T00", "t0", Some(0.0)), ("MQ_H2O2_T00", "t0", Some(0.0)),
        ("MQ_H2O2_T30", "sample", Some(30.0)), ("MQ-H2O2-T30", "sample", Some(30.0)),
        // dark controls: suffix D on the condition or apart; the time is kept, the role is not a time zero
        ("MQ_H2O2D_T00", "dark", Some(0.0)), ("MQ-H2O2D-T00", "dark", Some(0.0)), ("MQ_H2O2_D_T60", "dark", Some(60.0)),
        ("MQ-PMS-D-T60", "dark", Some(60.0)), ("TW_PMSD_T30", "dark", Some(30.0)),
        // dark matrix blanks
        ("TW_BLKD_T00", "dark", Some(0.0)), ("TW-BLKD-T00", "dark", Some(0.0)),
        ("MQ_STD05", "std", None), ("QC_MQ", "qc", None), ("MQ_BLK_1", "blank", None), ("MQ_Wash_1", "blank", None),
        ("MQ_dark_T10", "dark", Some(10.0)), ("MQ_T00_dark", "dark", Some(0.0)),
    ];
    for (name, kind, time) in rows {
        let c = table(&[name]).remove(0);
        assert_eq!((c.kind.as_str(), c.time_min), (kind, time), "{name}");
    }
}

#[test]
fn a_wash_is_a_blank_of_its_kind_and_a_dark_control_has_no_role_t0() {
    let t = table(&["MQ_Wash_1", "MQ_BLKD_T00", "MQ_H2O2_T00"]);
    assert_eq!(
        (t[0].kind.as_str(), t[0].blank_kind.as_deref()),
        ("blank", Some("wash"))
    );
    assert_eq!(t[1].role(), "dark");
    assert_eq!(t[2].role(), "t0");
}

#[test]
fn a_d_after_a_number_is_days_not_a_dark_mark() {
    for name in ["S 2 d", "S 3 d (2)", "S 1d", "S d1"] {
        let c = table(&[name]).remove(0);
        assert_eq!(c.kind, "sample", "{name}");
        assert!(c.time_min.is_some(), "{name}");
    }
}

#[test]
fn the_lone_d_is_read_from_the_stem_after_a_known_condition_only() {
    // an Agilent dataset folder, a folder called d, a d that follows a time or an unknown condition: none is a dark control, the t0 stays
    for name in [
        "MQ_H2O2_T00.d",
        "MQ_H2O2_T00.D",
        "data/d/MQ_H2O2_T00",
        "d/MQ_H2O2_T00.mzML",
    ] {
        let c = table(&[name]).remove(0);
        assert_eq!((c.kind.as_str(), c.time_min), ("t0", Some(0.0)), "{name}");
    }
    for name in ["mq_h2o2_t30_d", "x_d", "MQ_COND_D_T30", "S 2 d"] {
        let c = table(&[name]).remove(0);
        assert_ne!(c.kind, "dark", "{name}");
    }
    // the same rule for the role alone (the table roles of mzlab-feat and the finder) as for the table
    for (name, kind) in [
        ("MQ_H2O2_T00.d", "t0"),
        ("MQ_H2O2_D_T00", "dark"),
        ("MQ_COND_D_T00", "t0"),
    ] {
        assert_eq!(
            mzlab_core::samples::role_and_minutes(name).0,
            kind,
            "{name}"
        );
        assert_eq!(
            mzlab_core::samples::table_kind_of_name(name),
            kind,
            "{name}"
        );
    }
}

#[test]
fn the_evidence_of_a_lone_d_is_the_d_that_decided() {
    // a folder and a leading token `d1` hold a d too: the span is the d after the condition, in characters of the whole name
    let name = "D1_MQ_H2O2_D_T30";
    let files = [(name.to_string(), String::new(), String::new())];
    let p = &mzlab_core::samples::propose_explained(&files)[0];
    assert_eq!(p.sample.kind, "dark");
    let kind = p
        .positions
        .iter()
        .find(|x| x.field == "kind")
        .expect("kind position");
    assert_eq!(&name[kind.start..kind.end], "D");
    assert_eq!((kind.start, kind.end), (11, 12));
    let name = "d/MQ_H2O2_D_T30";
    let files = [(name.to_string(), String::new(), String::new())];
    let p = &mzlab_core::samples::propose_explained(&files)[0];
    let kind = p
        .positions
        .iter()
        .find(|x| x.field == "kind")
        .expect("kind position");
    assert_eq!((kind.start, kind.end), (10, 11));
}
