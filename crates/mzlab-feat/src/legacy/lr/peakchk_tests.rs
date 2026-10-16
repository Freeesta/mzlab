//! Tests of `peakchk`: the limits on synthetic traces.

use super::*;

fn trace(f: impl Fn(f64) -> f64) -> (Vec<f64>, Vec<f64>) {
    let rt: Vec<f64> = (0..600).map(|i| 0.4 + i as f64 * 0.04).collect();
    let y = rt
        .iter()
        .enumerate()
        .map(|(i, &t)| f(t) + 1000.0 + 300.0 * (((i * 7919) % 13) as f64 / 13.0))
        .collect();
    (rt, y)
}

#[test]
fn a_gaussian_peak_passes() {
    let (rt, y) = trace(|t| 2e5 * (-((t - 12.0) / 0.1f64).powi(2) / 2.0).exp());
    let q = judge(&rt, &y, 12.0, &Settings::default());
    assert!(q.ok(), "{q:?}");
    assert!(q.fwhm_s > 10.0 && q.fwhm_s < 25.0, "{q:?}");
}

#[test]
fn a_small_isomer_in_a_sparse_trace_passes() {
    // single noise ions in one scan out of seven, a strong isomer 0.9 min before: the isomer is not noise
    let rt: Vec<f64> = (0..270).map(|i| 0.5 + i as f64 * 0.05).collect();
    let y: Vec<f64> = rt
        .iter()
        .enumerate()
        .map(|(i, &t)| {
            let g = |c: f64, a: f64| a * (-((t - c) / 0.07f64).powi(2) / 2.0).exp();
            g(7.4, 1.1e6) + g(8.3, 8.0e4) + if i % 7 == 3 { 3e3 } else { 0.0 }
        })
        .collect();
    let q = judge(&rt, &y, 8.3, &Settings::default());
    assert!(q.ok() && q.snr > 20.0, "{q:?}");
}

#[test]
fn a_hump_a_spike_and_the_front_fail() {
    let s = Settings::default();
    let (rt, y) = trace(|t| 3e3 * (-((t - 10.0) / 6.0f64).powi(2) / 2.0).exp());
    assert_eq!(judge(&rt, &y, 10.0, &s).why, Some("S/N too low"));
    let (rt, y) = trace(|t| 2e6 * (-((t - 10.0) / 2.0f64).powi(2) / 2.0).exp());
    assert_eq!(
        judge(&rt, &y, 10.0, &s).why,
        Some("too broad for a chromatographic peak")
    );
    let (rt, y) = trace(|t| if (t - 8.0).abs() < 0.02 { 5e5 } else { 0.0 });
    assert_eq!(judge(&rt, &y, 8.0, &s).why, Some("too few scans"));
    let (rt, y) = trace(|t| 2e5 * (-((t - 0.5) / 0.1f64).powi(2) / 2.0).exp());
    assert_eq!(judge(&rt, &y, 0.5, &s).why, Some("apex in the dead volume"));
}
