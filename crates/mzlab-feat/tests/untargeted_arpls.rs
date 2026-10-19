//! arPLS baseline: parity with the independent numpy reference (`tools/arpls_riferimento.py`, dense solve) and properties.

use mzlab_feat::untargeted::arpls::{baseline, correct, correct_excluding_peaks};
use serde_json::Value;

fn cases() -> Vec<Value> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/data/untargeted/arpls/casi.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn vec_of(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

/// Tolerance 1e-9 of the largest value of the series: with lambda 1e5 the system is ill-conditioned, so the dense solve and the banded one agree
/// to that scale, not value by value.
fn close(a: &[f64], b: &[f64], name: &str) {
    assert_eq!(a.len(), b.len(), "{name}: length");
    let scale = b.iter().fold(1.0f64, |m, v| m.max(v.abs()));
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        assert!((x - y).abs() <= 1e-9 * scale, "{name}[{i}]: {x} vs {y}");
    }
}

#[test]
fn matches_the_numpy_reference() {
    for c in cases() {
        let name = c["name"].as_str().unwrap();
        let y = vec_of(&c["y"]);
        let (lam, it) = (
            c["lambda"].as_f64().unwrap(),
            c["max_iter"].as_u64().unwrap() as usize,
        );
        close(&baseline(&y, lam, it), &vec_of(&c["baseline"]), name);
        close(&correct(&y, lam, it), &vec_of(&c["corrected"]), name);
    }
}

#[test]
fn short_traces_give_a_zero_baseline() {
    assert_eq!(baseline(&[4.0, 9.0], 1e5, 50), vec![0.0, 0.0]);
    assert_eq!(baseline(&[], 1e5, 50), Vec::<f64>::new());
    assert_eq!(correct(&[4.0, 9.0], 1e5, 50), vec![4.0, 9.0]);
}

#[test]
fn no_value_below_the_baseline_returns_at_once() {
    // all residuals are 0: no negative one, the first solution is returned
    let z = baseline(&[0.0; 10], 1e5, 50);
    assert!(z.iter().all(|v| *v == 0.0));
}

#[test]
fn the_corrected_trace_is_never_negative_and_the_peak_survives() {
    for c in cases() {
        let y = vec_of(&c["y"]);
        let out = correct(
            &y,
            c["lambda"].as_f64().unwrap(),
            c["max_iter"].as_u64().unwrap() as usize,
        );
        assert!(out.iter().all(|v| *v >= 0.0), "{}", c["name"]);
    }
    let c = &cases()[0];
    let y = vec_of(&c["y"]);
    let z = baseline(&y, 1e5, 50);
    assert!(
        z[60] < y[60] - 3000.0,
        "the baseline must not follow the peak"
    );
}

#[test]
fn excluding_the_peaks_keeps_more_of_the_peak() {
    let c = &cases()[0];
    let y = vec_of(&c["y"]);
    let x: Vec<f64> = (0..y.len()).map(|i| i as f64 * 0.02).collect();
    let plain = correct(&y, 1e2, 50);
    let excl = correct_excluding_peaks(&x, &y, 1e2, 50);
    assert_eq!(excl.len(), y.len());
    assert!(excl.iter().all(|v| *v >= 0.0));
    assert!(excl[60] >= plain[60], "{} vs {}", excl[60], plain[60]);
    // with no peak found the result is that of the plain correction
    let flat = vec![5.0; 30];
    let xf: Vec<f64> = (0..30).map(|i| i as f64 * 0.02).collect();
    assert_eq!(
        correct_excluding_peaks(&xf, &flat, 1e5, 50),
        correct(&flat, 1e5, 50)
    );
}
