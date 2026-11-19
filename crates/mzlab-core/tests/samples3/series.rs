//! The lab ID convention (`MAT-COND-Ttt` ...), several samples in one file and the new role `pool`, with synthetic names only.

use super::codes;
use super::model::run as run_model;
use mzlab_core::samples::{self, table_from_json, Sample};
use serde_json::Value;

const MODEL: &str = "{data}_{polarity}_{n}_{sample_id}";

fn field(c: &Value, k: &str) -> Value {
    c[k].clone()
}

#[test]
fn ids_of_the_series_fill_the_fields_the_model_did_not_capture() {
    let names = [
        "20300101_neg_001_TW-PMS-T2p5.mzML",
        "20300101_pos_002_MQ-C-T30.mzML",
        "20300101_neg_003_TW-H2O2D-T10.mzML",
        "20300101_neg_004_TW-STD5.mzML",
        "20300101_neg_005_QC-TW.mzML",
        "20300101_neg_006_P-PMS.mzML",
        "20300101_neg_007_BLKP.mzML",
        "20300101_neg_008_MQ_Wash.mzML",
        "20300101_neg_009_TW-PMS-T5-R.mzML",
    ];
    let out = run_model(MODEL, &names);
    let cs = out["samples"]["samples"].as_array().unwrap();
    let c = &cs[0];
    assert_eq!(field(c, "kind"), "sample");
    assert_eq!(field(c, "matrix"), "TW");
    assert_eq!(field(c, "series"), "PMS");
    assert_eq!(field(c, "group"), "TW-PMS");
    assert_eq!(field(c, "time_min"), 2.5);
    assert_eq!(field(c, "sample_id"), "TW-PMS-T2p5");
    assert_eq!(
        (field(&cs[1], "matrix"), field(&cs[1], "time_min")),
        ("MQ".into(), 30.into())
    );
    assert_eq!(
        (field(&cs[2], "kind"), field(&cs[2], "time_min")),
        ("dark".into(), 10.into())
    );
    assert_eq!(
        (field(&cs[3], "kind"), field(&cs[3], "concentration")),
        ("std".into(), 5.into())
    );
    assert_eq!(field(&cs[3], "concentration_unit"), "mg/L");
    assert_eq!(
        (field(&cs[4], "kind"), field(&cs[4], "matrix")),
        ("qc".into(), "TW".into())
    );
    assert_eq!(field(&cs[5], "kind"), "pool");
    assert_eq!(field(&cs[6], "kind"), "blank");
    assert_eq!(field(&cs[7], "kind"), "blank");
    assert_eq!(
        (field(&cs[8], "kind"), field(&cs[8], "time_min")),
        ("sample".into(), 5.into())
    );
}

#[test]
fn a_captured_field_wins_over_the_id() {
    let out = run_model("{sample_id}_{time}min", &["TW-PMS-T5_15min.mzML"]);
    let c = &out["samples"]["samples"][0];
    assert_eq!(field(c, "time_min"), 15);
    assert_eq!(field(c, "series"), "PMS");
}

#[test]
fn the_model_reads_a_time_with_p() {
    let out = run_model("{sample_id}_{time}min", &["S1_2p5min.mzML"]);
    assert_eq!(field(&out["samples"]["samples"][0], "time_min"), 2.5);
}

#[test]
fn several_samples_in_one_file_are_not_duplicates() {
    let row = |sub: &str| {
        let mut c = Sample::new("ReRun.wiff", "ab", 1);
        c.sub = sub.into();
        c
    };
    let t = |subs: &[&str]| samples::Table {
        samples: subs.iter().map(|s| row(s)).collect(),
        columns: Vec::new(),
    };
    let ok = t(&["0", "1", "2"]);
    assert!(!codes(&ok).iter().any(|c| c == "duplicateKey"));
    assert!(codes(&t(&["1", "1"])).iter().any(|c| c == "duplicateKey"));
    let again = samples::table_to_json(&ok);
    assert!(again.contains("\"sub_sample\":\"2\""));
    assert_eq!(
        samples::table_to_json(&table_from_json(again.as_bytes()).unwrap()),
        again
    );
}

#[test]
fn valida_warns_about_an_unknown_condition_and_a_bad_time() {
    let mut a = Sample::new("a.mzML", "", 1);
    a.sample_id = "TW-XYZ-T5".into();
    let mut b = Sample::new("b.mzML", "", 2);
    b.sample_id = "TW-PMS-T5x".into();
    let t = samples::Table {
        samples: vec![a, b],
        columns: Vec::new(),
    };
    let got = codes(&t);
    assert!(
        got.contains(&"unknownCondition:series".to_string()),
        "{got:?}"
    );
    assert!(got.contains(&"typeMismatch:time".to_string()), "{got:?}");
}

#[test]
fn pool_is_a_role_and_pos_neg_replicates_stay_apart() {
    let out = run_model(
        MODEL,
        &["20300101_neg_001_P-PMS.mzML", "20300101_pos_002_P-PMS.mzML"],
    );
    let cs = out["samples"]["samples"].as_array().unwrap();
    assert_eq!(field(&cs[0], "kind"), "pool");
    assert_eq!(
        (field(&cs[0], "replicate"), field(&cs[1], "replicate")),
        (1.into(), 1.into())
    );
}

#[test]
fn a_reinjection_keeps_its_id_and_is_flagged() {
    let out = run_model(
        MODEL,
        &[
            "20300101_neg_001_TW-PMS-T5.mzML",
            "20300101_neg_002_TW-PMS-T5-R.mzML",
        ],
    );
    let cs = out["samples"]["samples"].as_array().unwrap();
    assert_eq!(
        (field(&cs[0], "reinjection"), field(&cs[1], "reinjection")),
        (false.into(), true.into())
    );
    assert_eq!(field(&cs[1], "sample_id"), "TW-PMS-T5-R");
}

#[test]
fn a_p_in_a_time_is_the_decimal_point_and_the_order_is_numeric() {
    let names = [
        "MQ_PMS_T10p25.mzML",
        "MQ_PMS_T2p5.mzML",
        "MQ_PMS_T00.mzML",
        "MQ_PMS_T0p5.mzML",
        "MQ_PMS_T10.mzML",
    ];
    let mut got: Vec<(f64, &str)> = super::many(&names)
        .iter()
        .zip(names)
        .map(|(c, n)| (c.time_min.unwrap_or(f64::NAN), n))
        .collect();
    let times: Vec<f64> = got.iter().map(|g| g.0).collect();
    assert_eq!(times, [10.25, 2.5, 0.0, 0.5, 10.0], "{got:?}");
    got.sort_by(|a, b| a.0.total_cmp(&b.0));
    let order: Vec<&str> = got.iter().map(|g| g.1).collect();
    assert_eq!(order[1], "MQ_PMS_T0p5.mzML");
    assert_eq!(order[4], "MQ_PMS_T10p25.mzML");
}

#[test]
fn blank_words_make_a_blank_only_without_a_time_or_a_series() {
    let kind = |n: &str| mzlab_core::samples::role_and_minutes(n).0;
    for n in [
        "MQ_Wash_1.mzML",
        "MQ_blk.mzML",
        "milli-q_2.mzML",
        "20300101_neg_008_MQ_Wash.mzML",
    ] {
        assert_eq!(kind(n), "blank", "{n}");
    }
    for n in [
        "MQ_PMS_T00.mzML",
        "TW_H2O2_T00.mzML",
        "MQ_PMS_T2p5.mzML",
        "SQ_PMS_T5.mzML",
    ] {
        assert_ne!(kind(n), "blank", "{n}");
    }
}

#[test]
fn washes_are_blanks_of_kind_wash_and_no_reference() {
    let names = [
        "MQ_Wash_init.mzML",
        "MQ_Wash_1.mzML",
        "MQ_blk.mzML",
        "MQ_PMS_T05.mzML",
    ];
    let cs = super::many(&names);
    let kinds: Vec<(&str, Option<&str>)> = cs
        .iter()
        .map(|c| (c.kind.as_str(), c.blank_kind.as_deref()))
        .collect();
    assert_eq!(
        kinds[..3],
        [
            ("blank", Some("wash")),
            ("blank", Some("wash")),
            ("blank", None)
        ],
        "{kinds:?}"
    );
    let p = mzlab_core::samples::per_tp(&cs);
    assert_eq!(p.blank, [2]);
    assert_eq!(p.treated, [3]);
}

#[test]
fn matrix_and_group_come_from_the_first_word_shared_by_several_files() {
    let names = [
        "TW_PMS_T00.mzML",
        "TW_PMS_T05.mzML",
        "TW_PMSD_T00.mzML",
        "TW_PMSD_T05.mzML",
        "TW_blk.mzML",
        "MQ_PMS_T00.mzML",
        "MQ_PMS_T05.mzML",
        "MQ_blk.mzML",
        "MQ_std_X_1ppm.mzML",
    ];
    let cs = super::many(&names);
    let got: Vec<(&str, &str, &str)> = cs
        .iter()
        .map(|c| (c.matrix.as_str(), c.series.as_str(), c.group.as_str()))
        .collect();
    assert_eq!(got[0], ("TW", "TW_PMS", "TW-PMS"));
    assert_eq!(got[2], ("TW", "TW_PMSD", "TW-PMSD"));
    assert_eq!(got[4], ("TW", "", ""));
    assert_eq!(got[5], ("MQ", "MQ_PMS", "MQ-PMS"));
    assert_eq!(got[8].0, "MQ");
    // one blank per matrix: both are the first of their matrix
    assert_eq!((cs[4].replicate, cs[7].replicate), (1, 1));
}

#[test]
fn a_single_first_word_proposes_no_matrix() {
    let cs = super::many(&["X_TiO2_t005min", "X_TiO2_t010min", "X_blank"]);
    assert!(cs.iter().all(|c| c.matrix.is_empty() && c.group.is_empty()));
}

#[test]
fn propose_gives_the_matrix_and_the_group() {
    let input = r#"{"files":[{"name":"TW_PMS_T00.mzML"},{"name":"TW_PMS_T05.mzML"},{"name":"TW_blk.mzML"},{"name":"MQ_PMS_T00.mzML"},{"name":"MQ_PMS_T05.mzML"},{"name":"MQ_Wash_1.mzML"}]}"#;
    let v: serde_json::Value =
        serde_json::from_str(&mzlab_core::samples::route("propose", input).unwrap()).unwrap();
    assert_eq!(v["schema"], "samples/1");
    let cs = v["samples"].as_array().unwrap();
    let f = |i: usize, k: &str| cs[i][k].clone();
    assert_eq!(
        (f(0, "matrix"), f(0, "group"), f(0, "series")),
        ("TW".into(), "TW-PMS".into(), "TW_PMS".into())
    );
    assert_eq!(f(3, "matrix"), "MQ");
    assert_eq!(f(3, "time_min").as_f64(), Some(0.0));
    assert_eq!(
        (f(5, "kind"), f(5, "blank_kind")),
        ("blank".into(), "wash".into())
    );
}
