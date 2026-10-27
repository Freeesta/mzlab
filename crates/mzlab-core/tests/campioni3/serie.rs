//! The lab ID convention (`MAT-COND-Ttt` ...), several samples in one file and the new role `pool`, with synthetic names only.

use super::codes;
use super::modello::run as run_model;
use mzlab_core::campioni::{self, tabella_da_json, Campione};
use serde_json::Value;

const MODEL: &str = "{data}_{polarita}_{n}_{campione_id}";

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
    let cs = out["campioni"]["campioni"].as_array().unwrap();
    let c = &cs[0];
    assert_eq!(field(c, "tipo"), "sample");
    assert_eq!(field(c, "matrice"), "TW");
    assert_eq!(field(c, "serie"), "PMS");
    assert_eq!(field(c, "gruppo"), "TW-PMS");
    assert_eq!(field(c, "tempo_min"), 2.5);
    assert_eq!(field(c, "campione_id"), "TW-PMS-T2p5");
    assert_eq!(
        (field(&cs[1], "matrice"), field(&cs[1], "tempo_min")),
        ("MQ".into(), 30.into())
    );
    assert_eq!(
        (field(&cs[2], "tipo"), field(&cs[2], "tempo_min")),
        ("dark".into(), 10.into())
    );
    assert_eq!(
        (field(&cs[3], "tipo"), field(&cs[3], "concentrazione")),
        ("std".into(), 5.into())
    );
    assert_eq!(field(&cs[3], "unita"), "mg/L");
    assert_eq!(
        (field(&cs[4], "tipo"), field(&cs[4], "matrice")),
        ("qc".into(), "TW".into())
    );
    assert_eq!(field(&cs[5], "tipo"), "pool");
    assert_eq!(field(&cs[6], "tipo"), "blank");
    assert_eq!(field(&cs[7], "tipo"), "blank");
    assert_eq!(
        (field(&cs[8], "tipo"), field(&cs[8], "tempo_min")),
        ("sample".into(), 5.into())
    );
}

#[test]
fn a_captured_field_wins_over_the_id() {
    let out = run_model("{campione_id}_{tempo}min", &["TW-PMS-T5_15min.mzML"]);
    let c = &out["campioni"]["campioni"][0];
    assert_eq!(field(c, "tempo_min"), 15);
    assert_eq!(field(c, "serie"), "PMS");
}

#[test]
fn the_model_reads_a_time_with_p() {
    let out = run_model("{campione_id}_{tempo}min", &["S1_2p5min.mzML"]);
    assert_eq!(field(&out["campioni"]["campioni"][0], "tempo_min"), 2.5);
}

#[test]
fn several_samples_in_one_file_are_not_duplicates() {
    let row = |sub: &str| {
        let mut c = Campione::new("ReRun.wiff", "ab", 1);
        c.sub = sub.into();
        c
    };
    let t = |subs: &[&str]| campioni::Tabella {
        campioni: subs.iter().map(|s| row(s)).collect(),
        colonne: Vec::new(),
    };
    let ok = t(&["0", "1", "2"]);
    assert!(!codes(&ok).iter().any(|c| c == "duplicateKey"));
    assert!(codes(&t(&["1", "1"])).iter().any(|c| c == "duplicateKey"));
    let again = campioni::to_json3(&ok);
    assert!(again.contains("\"campione\":\"2\""));
    assert_eq!(
        campioni::to_json3(&tabella_da_json(again.as_bytes()).unwrap()),
        again
    );
}

#[test]
fn valida_warns_about_an_unknown_condition_and_a_bad_time() {
    let mut a = Campione::new("a.mzML", "", 1);
    a.sample_id = "TW-XYZ-T5".into();
    let mut b = Campione::new("b.mzML", "", 2);
    b.sample_id = "TW-PMS-T5x".into();
    let t = campioni::Tabella {
        campioni: vec![a, b],
        colonne: Vec::new(),
    };
    let got = codes(&t);
    assert!(
        got.contains(&"unknownCondition:serie".to_string()),
        "{got:?}"
    );
    assert!(got.contains(&"typeMismatch:tempo".to_string()), "{got:?}");
}

#[test]
fn pool_is_a_role_and_pos_neg_replicates_stay_apart() {
    let out = run_model(
        MODEL,
        &["20300101_neg_001_P-PMS.mzML", "20300101_pos_002_P-PMS.mzML"],
    );
    let cs = out["campioni"]["campioni"].as_array().unwrap();
    assert_eq!(field(&cs[0], "tipo"), "pool");
    assert_eq!(
        (field(&cs[0], "replica"), field(&cs[1], "replica")),
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
    let cs = out["campioni"]["campioni"].as_array().unwrap();
    assert_eq!(
        (field(&cs[0], "reiniezione"), field(&cs[1], "reiniezione")),
        (false.into(), true.into())
    );
    assert_eq!(field(&cs[1], "campione_id"), "TW-PMS-T5-R");
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
    let kind = |n: &str| mzlab_core::campioni::ruolo_e_minuti(n).0;
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
