//! Tests of `dad_uv.rs`: the UV spectrum, the wavelength trace and the XWC band proposed, on small synthetic DAD matrices.
use super::*;

fn dad(nt: usize, nl: usize) -> DadData {
    DadData {
        time_min: (0..nt).map(|i| i as f64 * 0.01).collect(),
        twc_mau: vec![0.0; nt],
        wavelengths_nm: (0..nl).map(|j| 200.0 + j as f64).collect(),
        wl_start_nm: 200.0,
        wl_stop_nm: 200.0 + (nl - 1) as f64,
        wl_step_nm: 1.0,
        duration_min: nt as f64 * 0.01,
        matrix_mau: (0..nt * nl).map(|k| k as f32).collect(),
    }
}

#[test]
fn uv_spectrum_is_the_mean_of_the_window_or_the_nearest_row() {
    let d = dad(5, 3);
    // rows 1 and 2 (0.01, 0.02): means of [3,4,5] and [6,7,8]
    let j = dad_spectrum_json(&d, 0.01, 0.02).unwrap_or_default();
    assert!(j.contains("\"nm\":[200.00,201.00,202.00]"), "{j}");
    assert!(
        j.contains("\"mau\":[4.5000,5.5000,6.5000]") && j.contains("\"n\":2"),
        "{j}"
    );
    // between two rows, reversed bounds: the nearest row (0.03 -> row 3)
    let j = dad_spectrum_json(&d, 0.0305, 0.0295).unwrap_or_default();
    assert!(
        j.contains("\"mau\":[9.0000,10.0000,11.0000]") && j.contains("\"n\":1"),
        "{j}"
    );
    assert!(dad_spectrum_json(&d, f64::NAN, 0.0).is_none());
}

#[test]
fn wavelength_trace_is_the_mean_of_the_channels_in_range() {
    let d = dad(3, 3);
    // channels 0 and 1 (200, 201): rows [0,1,2] -> 0.5, [3,4,5] -> 3.5, [6,7,8] -> 6.5
    let j = dad_wavelength_json(&d, 201.0, 200.0).unwrap_or_default();
    assert!(
        j.contains("\"y\":[0.5000,3.5000,6.5000]") && j.contains("\"n\":2"),
        "{j}"
    );
    // no channel inside: the nearest one (202)
    let j = dad_wavelength_json(&d, 250.0, 260.0).unwrap_or_default();
    assert!(
        j.contains("\"y\":[2.0000,5.0000,8.0000]") && j.contains("\"nm0\":202"),
        "{j}"
    );
    assert!(dad_wavelength_json(&d, f64::NAN, 0.0).is_none());
}
/// A run of `nt` rows at 0.01 min, 200..=299 nm: a Gaussian elution at row `at` whose spectrum is `band(nm)`, plus `end(nm)` on every row.
fn synth(nt: usize, at: f64, band: impl Fn(f64) -> f64, end: impl Fn(f64) -> f64) -> DadData {
    let nm: Vec<f64> = (0..100).map(|j| 200.0 + j as f64).collect();
    let mut m = Vec::with_capacity(nt * nm.len());
    for i in 0..nt {
        let c = (-((i as f64 - at) / 4.0).powi(2) / 2.0).exp();
        m.extend(nm.iter().map(|&w| (c * band(w) + end(w)) as f32));
    }
    DadData {
        time_min: (0..nt).map(|i| i as f64 * 0.01).collect(),
        twc_mau: vec![],
        wl_start_nm: 200.0,
        wl_stop_nm: 299.0,
        wl_step_nm: 1.0,
        duration_min: nt as f64 * 0.01,
        wavelengths_nm: nm,
        matrix_mau: m,
    }
}
fn gauss(w: f64, c: f64, s: f64, h: f64) -> f64 {
    h * (-((w - c) / s).powi(2) / 2.0).exp()
}

#[test]
fn proposal_is_the_lambda_max_of_the_strongest_peak() {
    // two compounds: a strong one at row 30 (band at 274 nm) and a weak one at row 70 (band at 230 nm)
    let d = synth(100, 30.0, |w| gauss(w, 274.0, 6.0, 300.0), |_| 0.0);
    let mut d2 = synth(100, 70.0, |w| gauss(w, 230.0, 6.0, 40.0), |_| 0.0);
    for (a, b) in d2.matrix_mau.iter_mut().zip(&d.matrix_mau) {
        *a += b;
    }
    let j = dad_propose_json(&d2, f64::NAN, f64::NAN).unwrap_or_default();
    assert_eq!(j, "{\"nm\":274,\"bw\":4,\"rt\":0.3000,\"n\":3}");
    // a window on the weak compound: its own λmax
    let j = dad_propose_json(&d2, 0.69, 0.71).unwrap_or_default();
    assert!(j.starts_with("{\"nm\":230,\"bw\":4,"), "{j}");
}

#[test]
fn end_absorption_is_skipped_unless_nothing_else_absorbs() {
    // the compound absorbs most at 200 nm (end absorption) and has a real band at 254 nm
    let d = synth(
        100,
        50.0,
        |w| gauss(w, 200.0, 3.0, 900.0) + gauss(w, 254.0, 5.0, 120.0),
        |_| 0.0,
    );
    assert!(dad_propose_json(&d, f64::NAN, f64::NAN)
        .unwrap_or_default()
        .starts_with("{\"nm\":254,"));
    // only end absorption: it is the answer
    let d = synth(100, 50.0, |w| gauss(w, 200.0, 3.0, 900.0), |_| 0.0);
    assert!(dad_propose_json(&d, f64::NAN, f64::NAN)
        .unwrap_or_default()
        .starts_with("{\"nm\":200,"));
    // a constant solvent background (baseline) is removed before the search
    let d = synth(
        100,
        50.0,
        |w| gauss(w, 280.0, 5.0, 50.0),
        |w| gauss(w, 240.0, 8.0, 500.0),
    );
    assert!(dad_propose_json(&d, f64::NAN, f64::NAN)
        .unwrap_or_default()
        .starts_with("{\"nm\":280,"));
}

#[test]
fn a_flat_matrix_proposes_the_middle_and_an_empty_one_nothing() {
    let d = synth(40, 20.0, |_| 0.0, |_| 5.0);
    assert!(dad_propose_json(&d, f64::NAN, f64::NAN)
        .unwrap_or_default()
        .starts_with("{\"nm\":250,\"bw\":4,"));
    assert!(dad_propose_json(&dad(0, 3), f64::NAN, f64::NAN).is_none());
}
