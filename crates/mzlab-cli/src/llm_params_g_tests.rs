//! The cases G15-G22 of `docs/studi/assistente-dati/06-valutazione.md` for `mzlab_llm::params_rules`: series G01 (unit resolution) and G08 (DDA, high
//! resolution), expected values from the rules, against the real registry.

use super::params::{conf, find, n, near, run, val, G01, G08};

#[test]
fn g15_unit_series_gets_the_unit_axis_and_a_tolerance_in_da() {
    let v = run(G01, "{}");
    for step in ["features.estimate", "features.detect"] {
        assert_eq!(val(&v, step, "axis").str(), Some("Da"));
    }
    assert_eq!(n(&v, "features.detect", "polarity"), 1.0);
    near(n(&v, "tp.dig_lr", "tol_da"), 0.25);
    near(n(&v, "spectra.ms2", "tol_da"), 0.25);
    assert!(find(&v, "features.detect", "ppm").is_none());
}

#[test]
fn g16_width_of_the_g01_peaks() {
    let v = run(G01, "{}");
    let w = val(&v, "features.detect", "peak_width_s");
    near(w.at(0).unwrap().f64().unwrap(), 4.2);
    near(w.at(1).unwrap().f64().unwrap(), 16.8);
    assert_eq!(conf(&v, "features.detect", "peak_width_s"), "medium");
}

#[test]
fn g17_minimum_points_of_the_g01_peaks() {
    let v = run(G01, "{}");
    assert_eq!(n(&v, "features.detect", "min_points"), 3.0);
    assert_eq!(n(&v, "features.gapfill", "min_points"), 3.0);
    assert_eq!(n(&v, "features.detect", "max_gap"), 1.0);
}

#[test]
fn g18_window_of_retention_time_of_the_g01_series() {
    let v = run(G01, "{}");
    near(n(&v, "features.align", "rt_gap"), 0.14);
    near(n(&v, "features.gapfill", "rt_tol"), 0.14);
    near(n(&v, "tp.dig_lr", "rt_min"), 1.1);
    assert_eq!(conf(&v, "features.align", "rt_gap"), "high");
}

#[test]
fn g19_ppm_of_the_high_resolution_series() {
    let v = run(G08, "{}");
    assert_eq!(val(&v, "features.detect", "axis").str(), Some("ppm"));
    near(n(&v, "features.detect", "ppm"), 4.5);
    near(n(&v, "features.align", "ppm"), 4.5);
    assert_eq!(conf(&v, "features.detect", "ppm"), "high");
    assert!(find(&v, "tp.dig_lr", "tol_da").is_none());
}

#[test]
fn g20_width_of_the_high_resolution_peaks() {
    let v = run(G08, "{}");
    let w = val(&v, "features.detect", "peak_width_s");
    near(w.at(0).unwrap().f64().unwrap(), 3.0);
    near(w.at(1).unwrap().f64().unwrap(), 12.0);
    assert_eq!(n(&v, "features.detect", "min_points"), 3.0);
}

#[test]
fn g21_ms2_link_takes_half_of_the_isolation_window() {
    let v = run(G08, "{}");
    near(n(&v, "features.ms2_link", "default_half_window"), 0.6);
    assert_eq!(conf(&v, "features.ms2_link", "default_half_window"), "high");
}

#[test]
fn g22_a_valid_dda_high_resolution_plan() {
    let v = run(G08, r#"{"identify":true,"elements":"CHNOS"}"#);
    assert_eq!(val(&v, "table.filter", "require_ms2").bool(), Some(true));
    assert_eq!(n(&v, "features.detect", "max_gap"), 2.0);
    near(n(&v, "formula.ms1", "tol_ppm"), 4.5);
    near(n(&v, "formula.ms2", "ppm_ion"), 4.5);
    assert_eq!(val(&v, "formula.ms1", "elements").str(), Some("CHNOS"));
    assert!(v.get("warnings").unwrap().arr().is_empty());
}
