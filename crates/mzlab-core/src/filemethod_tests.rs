use super::*;

const WIFF: &str = r#"{"schema":"method/1","origin":{"format":"sciex-wiff"},"sample":{"name":"Name_237.1"},"ms":{"periods":[{"experiments":[{"index":0,"scan_type":"PI","precursor_mz":253.1,"mz":[50,300],"ce_eV":[35],"ces_eV":[15],"dp_V":[60]}]}]}}"#;
const ITEMS: &str = r#"{"schema":"items/1","items":[{"info":{"file":"dispersione.wiff","ms2_exps":[{"prec":253.1,"ce":35,"n":10}]}}]}"#;

#[test]
fn a_product_ion_segment_lists_its_parameters_and_the_missing_ones_say_why() {
    let d = parse(document(Some(1), Some(WIFF), ITEMS).unwrap().as_bytes()).unwrap();
    let s = d.get("segments").unwrap().at(0).unwrap();
    assert_eq!(s.get("precursor_mz").unwrap().f64(), Some(253.1));
    assert_eq!(s.get("ce_eV").unwrap().f64(), Some(35.0));
    assert_eq!(s.get("ces_eV").unwrap().f64(), Some(15.0));
    assert_eq!(s.get("dp_V").unwrap().f64(), Some(60.0));
    assert!(s.get("fill_time_ms").unwrap().is_null() && s.get("q1_resolution").unwrap().is_null());
    let u = d.get("unavailable").unwrap();
    assert_eq!(u.get("fill_time_ms").unwrap().str(), Some("unknown"));
    assert_eq!(u.get("q1_resolution").unwrap().str(), Some("unknown"));
    assert!(
        u.get("ce_eV").is_none(),
        "a parameter the method holds is not listed"
    );
}

#[test]
fn a_name_mass_that_differs_from_the_precursor_is_a_warning_with_the_numbers() {
    let d = parse(document(Some(1), Some(WIFF), ITEMS).unwrap().as_bytes()).unwrap();
    let w = d.get("warnings").unwrap().at(0).unwrap();
    assert_eq!(w.get("key").unwrap().str(), Some("warn.file.nameMz"));
    assert_eq!(w.get("nearest_name_mz").unwrap().f64(), Some(237.1));
    assert_eq!(w.get("nearest_precursor").unwrap().f64(), Some(253.1));
    assert_eq!(w.get("delta_mz").unwrap().f64(), Some(16.0));
}

#[test]
fn a_file_without_a_method_still_answers_with_null_and_a_reason() {
    let d = parse(document(Some(1), None, ITEMS).unwrap().as_bytes()).unwrap();
    assert!(d.get("format").unwrap().is_null());
    assert_eq!(d.get("segments").unwrap().arr().len(), 0);
    assert_eq!(
        d.get("unavailable").unwrap().get("ce_eV").unwrap().str(),
        Some("not_in_file")
    );
    assert_eq!(
        d.get("acquired")
            .unwrap()
            .at(0)
            .unwrap()
            .get("precursor_mz")
            .unwrap()
            .f64(),
        Some(253.1)
    );
}

#[test]
fn an_mrm_experiment_gives_one_segment_per_transition() {
    let m = r#"{"schema":"method/1","origin":{"format":"sciex-wiff"},"ms":{"periods":[{"experiments":[{"index":0,"transitions":[{"q1":237.1,"q3":194.1,"ce_eV":25,"dp_V":60,"dwell_ms":50,"name":"a"},{"q1":237.1,"q3":179.1,"ce_eV":35,"dp_V":60,"dwell_ms":50,"name":"b"}]}]}]}}"#;
    let d = parse(
        document(None, Some(m), r#"{"items":[]}"#)
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    assert_eq!(d.get("segments").unwrap().arr().len(), 2);
    assert_eq!(
        d.get("segments")
            .unwrap()
            .at(1)
            .unwrap()
            .get("q3_mz")
            .unwrap()
            .f64(),
        Some(179.1)
    );
    assert!(d.get("warnings").unwrap().arr().is_empty());
}

#[test]
fn a_damaged_stream_is_listed_and_its_missing_parameters_say_damaged_not_not_in_method() {
    let damaged = r#"{"schema":"method/1","origin":{"format":"sciex-wiff"},"ms":{"periods":[{"experiments":[{"index":1,"scan_type":"PI","damaged":["mass_range","experiment_header"]}]}]}}"#;
    let d = parse(document(Some(1), Some(damaged), ITEMS).unwrap().as_bytes()).unwrap();
    let bad = d.get("damaged").unwrap().arr();
    assert_eq!(bad.len(), 2);
    assert_eq!(bad[0].get("experiment").unwrap().f64(), Some(1.0));
    assert_eq!(bad[0].get("part").unwrap().str(), Some("mass_range"));
    let u = d.get("unavailable").unwrap();
    for k in ["ce_eV", "ces_eV", "dp_V"] {
        assert_eq!(u.get(k).unwrap().str(), Some("damaged"), "{k}");
    }
    // fill time is not held by a damaged stream: its reason stays what it was
    assert_eq!(u.get("fill_time_ms").unwrap().str(), Some("unknown"));
    let clean = parse(document(Some(1), Some(WIFF), ITEMS).unwrap().as_bytes()).unwrap();
    assert_eq!(clean.get("damaged").unwrap().arr().len(), 0);
}

#[test]
fn the_name_check_of_both_commands_uses_the_same_precursors() {
    // the method fixes precursor 237.1; the scans hold none: neither command may warn about a name that carries 237.1
    let method = r#"{"schema":"method/1","origin":{"format":"sciex-wiff"},"sample":{"name":"Analyte_237.1"},"ms":{"periods":[{"experiments":[{"index":1,"scan_type":"PI","precursor_mz":237.1}]}]}}"#;
    let items = r#"{"schema":"items/1","items":[{"info":{"file":"x.wiff","ms2_exps":[]}}]}"#;
    let d = parse(document(Some(1), Some(method), items).unwrap().as_bytes()).unwrap();
    assert_eq!(d.get("warnings").unwrap().arr().len(), 0);
    let precs = precursors_of(Some(method), &parse(items.as_bytes()).unwrap());
    assert_eq!(precs, vec![237.1]);
    assert!(name_warnings(&["Analyte_237.1"], &precs).is_empty());
}
