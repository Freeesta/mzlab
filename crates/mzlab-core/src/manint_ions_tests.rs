//! `manint::table_text_ions` on synthetic traces of a known factor, one per scan type: the columns come after every other one, the cps bytes do not
//! change, and each factor is the one `ionscale::factor` gives for the method values (or the ion quantum of the trace for `other`).

use super::*;

/// A trace of counts: a flat background of 0..3 ions per point and a peak, every point a whole number of `q` cps; a point every 0.01 min.
fn counts(q: f64) -> String {
    let (mut rt, mut y) = (Vec::new(), Vec::new());
    let mut s: u64 = 7;
    for i in 0..400 {
        let t = i as f64 * 0.01;
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let bg = ((s >> 33) % 3) as f64;
        let peak = 40.0 * (-((t - 2.0) / 0.1).powi(2)).exp();
        rt.push(format!("{t:.2}"));
        y.push(format!("{}", (bg + peak.round()) * q));
    }
    format!(
        "{{\"q1\":100.1,\"rt\":[{}],\"y\":[{}]}}",
        rt.join(","),
        y.join(",")
    )
}

fn files() -> String {
    format!(
        "[{{\"name\":\"a.mzML\",\"transitions\":[{}]}}]",
        counts(20.0)
    )
}

const W: &str = "[{\"file\":0,\"tr\":0,\"a\":1.7,\"b\":2.3,\"baseline\":\"zero\"}]";

fn row(ions: &str) -> (String, Vec<String>) {
    let t = table_text_ions(&files(), W, false, ions).unwrap();
    let head = t.lines().next().unwrap().to_string();
    let r = t
        .lines()
        .nth(1)
        .unwrap()
        .split(',')
        .map(str::to_string)
        .collect();
    (head, r)
}

fn col(head: &str, r: &[String], name: &str) -> String {
    r[head.split(',').position(|h| h == name).unwrap()].clone()
}

fn factor_of(ions: &str) -> (String, String, String) {
    let (h, r) = row(ions);
    (
        col(&h, &r, "ions_per_cps"),
        col(&h, &r, "ion_source"),
        col(&h, &r, "ion_scan_type"),
    )
}

#[test]
fn the_ion_columns_come_last_and_the_cps_bytes_are_unchanged() {
    let plain = table_text(&files(), W, true).unwrap();
    let t = table_text_ions(
        &files(),
        W,
        true,
        "{\"samples\":[{\"scan_type\":\"sim\",\"dwell_ms\":50}]}",
    )
    .unwrap();
    assert!(t
        .lines()
        .next()
        .unwrap()
        .starts_with(plain.lines().next().unwrap()));
    assert!(t.lines().next().unwrap().ends_with(IONS_HEAD));
    for (a, b) in plain.lines().zip(t.lines()) {
        assert!(b.starts_with(a), "{b} does not start with {a}");
    }
    // the sum row keeps its cells, the ion cells empty
    let sum = t.lines().last().unwrap();
    assert!(sum.contains(",sum,") && sum.ends_with(",,,,,,,"));
    // an empty request is the same table as no request
    let none = table_text_ions(&files(), W, true, "{}").unwrap();
    assert!(none.lines().next().unwrap().ends_with(IONS_HEAD));
}

#[test]
fn every_scan_type_gives_its_own_factor() {
    // ems / epi: the fill time; sim: the dwell; q1: scan time x step / range
    let ems = factor_of("{\"samples\":[{\"scan_type\":\"ems\",\"fill_ms\":100}]}");
    assert_eq!(ems, ("0.1".into(), "method".into(), "ems".into()));
    let epi = factor_of("{\"samples\":[{\"scan_type\":\"epi\",\"fill_ms\":250}]}");
    assert_eq!(epi, ("0.25".into(), "method".into(), "epi".into()));
    let sim = factor_of("{\"samples\":[{\"scan_type\":\"sim\",\"dwell_ms\":50}]}");
    assert_eq!(sim, ("0.05".into(), "method".into(), "sim".into()));
    let q1 = factor_of("{\"samples\":[{\"scan_type\":\"q1\",\"scan_s\":2,\"step_da\":0.1,\"mass_lo\":100,\"mass_hi\":300}]}");
    assert_eq!(q1, ("0.001".into(), "method".into(), "q1".into()));
    // other: no method time exists, the quantum of the trace (20 cps per ion) says 1 / 20 s
    let other = factor_of("{\"samples\":[{\"scan_type\":\"other\"}]}");
    assert_eq!(other, ("0.05".into(), "data".into(), "other".into()));
    // a kind that needs a field the call lacks falls back to the data estimate, said in the source column
    let lacking = factor_of("{\"samples\":[{\"scan_type\":\"ems\"}]}");
    assert_eq!(lacking.1, "data");
}

#[test]
fn the_ion_values_are_the_cps_ones_at_the_factor() {
    let (h, r) = row("{\"samples\":[{\"scan_type\":\"ems\",\"fill_ms\":100}]}");
    let height: f64 = col(&h, &r, "height").parse().unwrap();
    let h_ions: f64 = col(&h, &r, "height_ions").parse().unwrap();
    assert!((h_ions - height * 0.1).abs() <= 1e-4 * h_ions.abs());
    // the area of the table is cps x s; the point spacing is 0.6 s, so the ions are the area / 0.6 x 0.1
    let area: f64 = col(&h, &r, "area").parse().unwrap();
    let a_ions: f64 = col(&h, &r, "area_ions").parse().unwrap();
    assert!(
        (a_ions - area / 0.6 * 0.1).abs() <= 1e-3 * a_ions.abs(),
        "{a_ions} vs {area}"
    );
    let noise: f64 = col(&h, &r, "noise_ions").parse().unwrap();
    assert!(noise > 0.0);
    assert!(col(&h, &r, "sn_ions").parse::<f64>().unwrap() > 3.0);
}

#[test]
fn a_factor_the_trace_brings_wins_and_says_so() {
    let (h, r) = row("{\"samples\":[{\"scan_type\":\"ems\",\"fill_ms\":100,\"traces\":[{\"ions_per_cps\":0.2}]}]}");
    assert_eq!(col(&h, &r, "ions_per_cps"), "0.2");
    assert_eq!(col(&h, &r, "ion_source"), "trace");
    // the dwell of one transition fills a SIM sample that has none of its own
    let (h, r) = row("{\"samples\":[{\"scan_type\":\"sim\",\"traces\":[{\"dwell_ms\":30}]}]}");
    assert_eq!(col(&h, &r, "ions_per_cps"), "0.03");
}

#[test]
fn no_factor_leaves_the_ion_cells_empty_and_the_cps_ones_stand() {
    // a trace that is not counts (a smooth curve) has no accepted quantum, and a SIM sample with no dwell has no method time either
    let smooth: Vec<String> = (0..200)
        .map(|i| format!("{}", 1000.0 + (i as f64 * 0.37).sin() * 3.3))
        .collect();
    let rt: Vec<String> = (0..200)
        .map(|i| format!("{:.2}", i as f64 * 0.01))
        .collect();
    let f = format!(
        "[{{\"name\":\"a.mzML\",\"transitions\":[{{\"q1\":100.1,\"rt\":[{}],\"y\":[{}]}}]}}]",
        rt.join(","),
        smooth.join(",")
    );
    let w = "[{\"file\":0,\"tr\":0,\"a\":0.8,\"b\":1.2}]";
    let t = table_text_ions(&f, w, false, "{\"samples\":[{\"scan_type\":\"sim\"}]}").unwrap();
    let r: Vec<&str> = t.lines().nth(1).unwrap().split(',').collect();
    let n = r.len();
    assert_eq!(&r[n - 7..n - 1], ["", "", "", "", "", ""]);
    assert_eq!(r[n - 1], "sim");
    assert!(!r[5].is_empty(), "the cps area stands");
}

#[test]
fn an_unknown_scan_type_is_an_error() {
    let e = table_text_ions(
        &files(),
        W,
        false,
        "{\"samples\":[{\"scan_type\":\"qqq\"}]}",
    )
    .unwrap_err();
    assert_eq!(e.key, "err.manint.ions");
}

#[test]
fn the_choices_name_the_scan_type_and_where_it_came_from() {
    let c = choices_text(
        &files(),
        "{\"samples\":[{\"scan_type\":\"sim\",\"scan_from\":\"the chromatograms of the file\",\"dwell_ms\":50}]}",
    )
    .unwrap();
    assert!(
        c.contains("\"scan_type\":\"sim\"")
            && c.contains("the chromatograms of the file")
            && c.contains("\"dwell_ms\":50")
    );
    let none = choices_text(&files(), "{}").unwrap();
    assert!(none.contains("\"scan_type\":\"other\"") && none.contains("not named"));
}
