//! The rules `R-*` of `mzlab_llm::params_rules` against the real registry: every target exists with its limits, one test per rule, the cases G15-G22 of
//! `docs/studi/assistente-dati/06-valutazione.md` (series G01 at unit resolution, series G08 DDA at high resolution) and the bytes are stable.

use mzlab_core::json::{parse, Value};
use mzlab_llm::params_rules::{propose_text, target_param, TARGETS};

use crate::flow::steps;

/// A unit-resolution full-scan series (G01): 3 QC, 1 blank, 6 samples; the measures are the ones the synthetic files give.
pub(crate) const G01: &str = r#"{"schema":"ispezione/1","files":[{"alias":"f1","status":"ok","class":"unit","class_basis":["mass_decimals:1"],"model":"3200 QTRAP",
"polarity":"positive","mode":"centroid","acquisition":["full_scan"],"cycle_ms1_s":1.054,"ms2_per_cycle":0,"dead_min":1.1,"noise":120,
"mass_error":{"unit":"Da","value":0.12,"sigma":0.04,"n":40},"peak_width":{"fwhm_min":0.14,"n":25},"rt_range_min":[0.5,20],
"gradient":[{"t_min":0,"B_pct":5},{"t_min":20,"B_pct":95}]}],
"series":{"files":1,"polarities":["positive"],"classes":["unit"],"roles":{"sample":6,"blank":1,"qc":3},
"qc":{"schema":"qc/1","qc":["f2","f6","f10"],"confidence":"high","proposal":{"features.align.rt_gap":0.14,"features.align.rt_warp":false}}}}"#;

/// A data-dependent Orbitrap series (G08): no QC, no blank.
pub(crate) const G08: &str = r#"{"schema":"ispezione/1","files":[{"alias":"f1","status":"ok","class":"high","class_basis":["analyzer:FTMS"],"model":"Orbitrap Exploris 120",
"polarity":"positive","mode":"centroid","acquisition":["full_scan","dda"],"cycle_ms1_s":0.9,"ms2_per_cycle":10,"noise":5000,
"isolation":{"windows":10,"width_mz":[1.2,1.2]},"mass_error":{"unit":"ppm","value":1.2,"sigma":1.4,"n":60},"peak_width":{"fwhm_min":0.1,"n":40},
"rt_range_min":[0.5,30]}],"series":{"files":1,"polarities":["positive"],"classes":["high"],"roles":{"sample":4}}}"#;

pub(crate) fn run(inspection: &str, answers: &str) -> Value {
    let out = propose_text(inspection, answers, &steps()).expect("a proposal");
    parse(out.as_bytes()).expect("JSON")
}

pub(crate) fn find<'a>(v: &'a Value, step: &str, name: &str) -> Option<&'a Value> {
    v.get("proposals").unwrap().arr().iter().find(|p| {
        p.get("step")
            .and_then(Value::str)
            .is_some_and(|s| s.starts_with(step))
            && p.get("name").and_then(Value::str) == Some(name)
    })
}

pub(crate) fn val(v: &Value, step: &str, name: &str) -> Value {
    find(v, step, name)
        .unwrap_or_else(|| panic!("no proposal for {step}.{name}"))
        .get("value")
        .unwrap()
        .clone()
}

pub(crate) fn n(v: &Value, step: &str, name: &str) -> f64 {
    val(v, step, name)
        .f64()
        .unwrap_or_else(|| panic!("{step}.{name} is not a number"))
}

pub(crate) fn conf<'a>(v: &'a Value, step: &str, name: &str) -> &'a str {
    find(v, step, name)
        .unwrap()
        .get("confidence")
        .and_then(Value::str)
        .unwrap()
}

pub(crate) fn asked(v: &Value, signal: &str) -> bool {
    v.get("missing")
        .unwrap()
        .arr()
        .iter()
        .any(|m| m.get("signal").and_then(Value::str) == Some(signal))
}

pub(crate) fn near(x: f64, want: f64) {
    assert!((x - want).abs() < 1e-6, "{x} against {want}");
}

pub(crate) fn with(base: &str, from: &str, to: &str) -> String {
    assert!(base.contains(from), "{from} is not in the fixture");
    base.replacen(from, to, 1)
}

#[test]
fn every_target_exists_in_the_registry_and_every_rule_has_one() {
    let reg = steps();
    for (rule, targets) in TARGETS {
        assert!(!targets.is_empty(), "{rule}");
        for (step, alg, name) in *targets {
            assert!(
                target_param(&reg, step, alg, name).is_some(),
                "{rule}: {step} {alg} {name}"
            );
        }
    }
    assert_eq!(TARGETS.len(), 24);
}

#[test]
fn proposals_name_registry_steps_and_stay_in_range() {
    let reg = steps();
    for text in [G01, G08] {
        let v = run(text, r#"{"identify":true}"#);
        assert_eq!(
            v.get("schema").and_then(Value::str),
            Some("proposta-param/1")
        );
        for p in v.get("proposals").unwrap().arr() {
            let step = p.get("step").and_then(Value::str).unwrap();
            let (id, ver) = step.split_once('@').unwrap();
            let s = reg
                .iter()
                .find(|s| s.id == id && s.version.to_string() == ver)
                .unwrap_or_else(|| panic!("{step}"));
            let name = p.get("name").and_then(Value::str).unwrap();
            let alg = p.get("algorithm").and_then(Value::str).unwrap_or("");
            assert!(
                target_param(&reg, s.id, alg, name).is_some(),
                "{step} {name}"
            );
            assert_eq!(
                p.get("range_ok").and_then(Value::bool),
                Some(true),
                "{step} {name}"
            );
            assert!(p
                .get("proof")
                .and_then(Value::str)
                .unwrap()
                .starts_with("ispezione:f1:"));
            assert!(!p.get("signals").unwrap().arr().is_empty(), "{step} {name}");
        }
    }
}

#[test]
fn the_same_input_gives_the_same_bytes() {
    let a = propose_text(G01, "{}", &steps()).unwrap();
    assert_eq!(a, propose_text(G01, "{}", &steps()).unwrap());
    assert!(propose_text("{\"schema\":\"x\"}", "", &steps()).is_err());
}

#[test]
fn r_pol_1_mixed_polarity_is_a_warning_and_a_question() {
    let v = run(
        &with(
            G01,
            r#""polarities":["positive"]"#,
            r#""polarities":["positive","negative"]"#,
        ),
        "{}",
    );
    assert_eq!(
        v.get("warnings").unwrap().at(0).and_then(Value::str),
        Some("warn.params.mixed_polarity")
    );
    assert!(asked(&v, "polarity"));
    let neg = run(
        &with(
            &with(G01, r#""polarity":"positive""#, r#""polarity":"negative""#),
            r#""polarities":["positive"]"#,
            r#""polarities":["negative"]"#,
        ),
        "{}",
    );
    assert_eq!(n(&neg, "features.table", "polarity"), -1.0);
}

#[test]
fn r_axis_1_unknown_class_is_a_question() {
    let v = run(
        &with(G01, r#""class":"unit""#, r#""class":"unknown""#),
        "{}",
    );
    assert!(find(&v, "features.detect", "axis").is_none());
    assert!(asked(&v, "resolution"));
}

#[test]
fn r_ppm_1_three_sigma_up_to_half_a_ppm() {
    let v = run(
        &with(G08, r#""sigma":1.4,"n":60"#, r#""sigma":0.2,"n":10"#),
        "{}",
    );
    near(n(&v, "features.detect", "ppm"), 2.0);
    assert_eq!(
        find(&v, "features.detect", "ppm")
            .unwrap()
            .get("rule")
            .and_then(Value::str),
        Some("R-ppm-1")
    );
    assert_eq!(conf(&v, "features.detect", "ppm"), "medium");
}

#[test]
fn r_ppm_2_the_model_when_nothing_is_measured() {
    let bare = with(
        G08,
        r#""mass_error":{"unit":"ppm","value":1.2,"sigma":1.4,"n":60},"#,
        "",
    );
    let v = run(&bare, "{}");
    near(n(&v, "features.detect", "ppm"), 5.0);
    assert_eq!(
        find(&v, "features.detect", "ppm")
            .unwrap()
            .get("rule")
            .and_then(Value::str),
        Some("R-ppm-2")
    );
    let tof = run(&with(&bare, "Orbitrap Exploris 120", "Q-TOF 6600"), "{}");
    near(n(&tof, "features.detect", "ppm"), 10.0);
    let other = run(&with(&bare, "Orbitrap Exploris 120", "X"), "{}");
    assert_eq!(conf(&other, "features.detect", "ppm"), "low");
}

#[test]
fn r_da_1_the_typical_value_is_a_question_without_a_measure() {
    let v = run(
        &with(
            G01,
            r#""mass_error":{"unit":"Da","value":0.12,"sigma":0.04,"n":40},"#,
            "",
        ),
        "{}",
    );
    near(n(&v, "tp.dig_lr", "tol_da"), 0.35);
    assert_eq!(conf(&v, "tp.dig_lr", "tol_da"), "low");
}

#[test]
fn r_floor_1_and_r_height_1_come_from_the_noise() {
    let v = run(G01, "{}");
    near(n(&v, "features.detect", "floor"), 120.0);
    near(n(&v, "features.gapfill", "floor"), 120.0);
    near(n(&v, "features.detect", "min_height"), 1200.0);
    near(n(&v, "table.filter", "min_height"), 1200.0);
    let none = run(&with(G01, r#""noise":120,"#, ""), "{}");
    assert!(find(&none, "features.detect", "floor").is_none());
}

#[test]
fn r_snr_1_five_for_weak_signals_ten_otherwise() {
    assert_eq!(
        n(
            &run(G08, r#"{"weak_signals":true}"#),
            "features.detect",
            "snr"
        ),
        5.0
    );
    assert_eq!(
        n(
            &run(G08, r#"{"weak_signals":false}"#),
            "features.detect",
            "snr"
        ),
        10.0
    );
    assert_eq!(conf(&run(G08, "{}"), "features.detect", "snr"), "low");
}

#[test]
fn r_width_1_and_r_pts_1_ask_when_the_width_is_absent() {
    let v = run(
        &with(G01, r#""peak_width":{"fwhm_min":0.14,"n":25},"#, ""),
        "{}",
    );
    assert!(find(&v, "features.detect", "peak_width_s").is_none());
    assert!(asked(&v, "peak_width"));
    assert!(find(&v, "features.detect", "min_points").is_none());
    assert!(asked(&v, "cycle"));
}

#[test]
fn r_pts_1_wide_peaks_need_more_points() {
    let v = run(&with(G01, r#""fwhm_min":0.14"#, r#""fwhm_min":0.5"#), "{}");
    assert_eq!(n(&v, "features.detect", "min_points"), 14.0);
}

#[test]
fn r_gap_1_a_regular_cycle_allows_one_missing_scan() {
    assert_eq!(n(&run(G01, "{}"), "features.gapfill", "max_gap"), 1.0);
    assert_eq!(n(&run(G08, "{}"), "features.gapfill", "max_gap"), 2.0);
}

#[test]
fn r_rt_1_one_value_for_the_alignment_and_the_tables() {
    let v = run(G01, "{}");
    for (step, name) in [
        ("features.align", "rt_gap"),
        ("features.gapfill", "rt_tol"),
        ("features.table", "rt_tol"),
    ] {
        near(n(&v, step, name), 0.14);
        assert_eq!(
            find(&v, step, name)
                .unwrap()
                .get("rule")
                .and_then(Value::str),
            Some("R-rt-1")
        );
    }
}

#[test]
fn r_rt_2_half_a_percent_of_the_gradient_without_qc() {
    let v = run(G08, "{}");
    near(n(&v, "features.align", "rt_gap"), 0.1475);
    assert_eq!(
        find(&v, "features.align", "rt_gap")
            .unwrap()
            .get("rule")
            .and_then(Value::str),
        Some("R-rt-2")
    );
    let none = run(
        &with(
            &with(G08, r#""rt_range_min":[0.5,30]"#, r#""rt_range_min":null"#),
            r#""noise":5000,"#,
            "",
        ),
        "{}",
    );
    assert!(asked(&none, "gradient"));
}

#[test]
fn r_warp_1_follows_the_drift_of_the_qc_files() {
    assert_eq!(
        val(&run(G01, "{}"), "features.align", "rt_warp").str(),
        Some("off")
    );
    let on = run(
        &with(
            G01,
            r#""features.align.rt_warp":false"#,
            r#""features.align.rt_warp":true"#,
        ),
        "{}",
    );
    assert_eq!(
        val(&on, "features.align", "rt_warp").str(),
        Some("piecewise")
    );
    assert!(find(&run(G08, "{}"), "features.align", "rt_warp").is_none());
}

#[test]
fn r_win_1_the_window_starts_after_the_dead_time() {
    near(n(&run(G01, "{}"), "tp.dig_lr", "rt_min"), 1.1);
    assert!(find(&run(G08, "{}"), "tp.dig_lr", "rt_min").is_none());
}

#[test]
fn r_blank_1_three_times_the_blank_or_the_question() {
    assert_eq!(n(&run(G01, "{}"), "table.filter", "blank_ratio"), 3.0);
    let v = run(G08, "{}");
    assert!(find(&v, "table.filter", "blank_ratio").is_none());
    assert!(asked(&v, "blank_files"));
}

#[test]
fn r_qc_1_thirty_percent_in_half_of_the_qc_files() {
    let v = run(G01, "{}");
    assert_eq!(n(&v, "table.filter", "qc_cv_max"), 30.0);
    assert_eq!(n(&v, "table.filter", "qc_min_files"), 2.0);
    assert!(asked(&run(G08, r#"{"objective":"quant"}"#), "qc_files"));
    assert!(!asked(&run(G08, "{}"), "qc_files"));
}

#[test]
fn r_det_1_half_of_the_smallest_group() {
    assert_eq!(n(&run(G01, "{}"), "table.filter", "min_detected"), 3.0);
    assert_eq!(
        n(
            &run(G08, r#"{"groups":[6,4,10]}"#),
            "table.filter",
            "min_detected"
        ),
        2.0
    );
    assert_eq!(
        n(
            &run(G08, r#"{"groups":[12,9]}"#),
            "table.filter",
            "min_detected"
        ),
        4.0
    );
    let none = run(&with(G08, r#""sample":4"#, r#""sample":0"#), "{}");
    assert!(asked(&none, "groups"));
}

#[test]
fn r_ms2_1_only_when_the_user_identifies_and_the_method_has_ms2() {
    assert!(asked(&run(G08, "{}"), "identify"));
    assert_eq!(
        val(
            &run(G08, r#"{"identify":false}"#),
            "table.filter",
            "require_ms2"
        )
        .bool(),
        Some(false)
    );
    let v = run(G01, r#"{"identify":true}"#);
    assert_eq!(val(&v, "table.filter", "require_ms2").bool(), Some(false));
    assert_eq!(
        v.get("warnings").unwrap().at(0).and_then(Value::str),
        Some("warn.params.no_ms2")
    );
}

#[test]
fn r_drift_1_only_with_five_qc_files() {
    assert!(find(&run(G01, "{}"), "table.drift", "method").is_none());
    let five = with(
        G01,
        r#""qc":["f2","f6","f10"]"#,
        r#""qc":["a","b","c","d","e"]"#,
    );
    assert_eq!(
        val(&run(&five, "{}"), "table.drift", "method").str(),
        Some("linear")
    );
    let many = with(
        G01,
        r#""qc":["f2","f6","f10"]"#,
        r#""qc":["a","b","c","d","e","f","g","h"]"#,
    );
    let v = run(&many, "{}");
    assert_eq!(val(&v, "table.drift", "method").str(), Some("loess"));
    near(n(&v, "table.drift", "span"), 0.75);
}

#[test]
fn r_iso_1_half_of_the_widest_isolation_window() {
    let v = run(&with(G08, "[1.2,1.2]", "[1.0,2.0]"), "{}");
    near(n(&v, "features.ms2_link", "default_half_window"), 1.0);
    assert!(find(&run(G01, "{}"), "features.ms2_link", "default_half_window").is_none());
}

#[test]
fn r_cent_1_the_threshold_of_a_profile_file_is_its_noise() {
    let v = run(
        &with(G08, r#""mode":"centroid""#, r#""mode":"profile""#),
        "{}",
    );
    near(n(&v, "spectra.centroid", "threshold"), 5000.0);
    assert!(asked(&v, "fwhm_mz"));
    assert!(find(&run(G08, "{}"), "spectra.centroid", "threshold").is_none());
}

#[test]
fn r_form_1_and_r_form_2_follow_the_high_resolution_tolerance() {
    let v = run(G08, "{}");
    near(n(&v, "formula.ms1", "tol_ppm"), 4.5);
    assert_eq!(val(&v, "formula.ms1", "elements").str(), Some("CHNO"));
    assert!(find(&run(G01, "{}"), "formula.ms1", "tol_ppm").is_none());
}
