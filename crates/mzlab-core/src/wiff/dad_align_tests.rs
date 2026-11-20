#![allow(clippy::cast_possible_truncation, clippy::unwrap_used)]
use super::*;

/// A run of Gaussian peaks (centres `peaks`, height 100, sigma 0.04 min) on a slow drift, 5 channels, read every `dt` minutes from 0 to 10.
fn run(dt: f64, peaks: &[f64], drift: f64) -> DadData {
    let n = (10.0 / dt) as usize + 1;
    let time_min: Vec<f64> = (0..n).map(|i| dt * i as f64).collect();
    let wavelengths_nm: Vec<f64> = (0..5).map(|j| 220.0 + 10.0 * j as f64).collect();
    let matrix_mau = time_min
        .iter()
        .flat_map(|&t| {
            let h: f64 = peaks
                .iter()
                .map(|c| 100.0 * (-((t - c) / 0.04).powi(2) / 2.0).exp())
                .sum::<f64>()
                + drift * t;
            (0..5).map(move |_| h as f32)
        })
        .collect();
    DadData {
        duration_min: 10.0,
        twc_mau: vec![0.0; n],
        wl_start_nm: 220.0,
        wl_stop_nm: 260.0,
        wl_step_nm: 10.0,
        time_min,
        wavelengths_nm,
        matrix_mau,
    }
}

#[test]
fn a_later_copy_is_found_with_the_sign_of_the_subtraction() {
    let a = run(0.01, &[2.0, 4.0, 7.0], 0.5);
    let b = run(0.01, &[2.12, 4.12, 7.12], 0.5);
    let r = dad_align(&a, &b, None);
    // B is read at t - shift: B is later by 0.12, so it is read 0.12 earlier
    assert!((r.shift_min + 0.12).abs() < 0.005, "{}", r.shift_min);
    assert!(r.corr > 0.99 && r.corr > r.corr_at_zero);
    assert_eq!(r.basis, "run");
    assert!(r.warnings.is_empty());
}

#[test]
fn a_shift_between_two_steps_is_refined() {
    let a = run(0.05, &[2.0, 4.0, 7.0], 0.0);
    let b = run(0.05, &[1.93, 3.93, 6.93], 0.0);
    let r = dad_align(&a, &b, None);
    assert!((r.shift_min - 0.07).abs() < 0.01, "{}", r.shift_min);
}

#[test]
fn equal_runs_give_no_shift() {
    let a = run(0.02, &[3.0, 6.0], 0.2);
    let r = dad_align(&a, &a, None);
    assert_eq!(r.shift_min, 0.0);
    assert!((r.corr - 1.0).abs() < 1e-9);
}

#[test]
fn the_window_is_used_when_it_holds_enough_points() {
    let a = run(0.01, &[2.0, 4.0, 7.0], 0.0);
    let b = run(0.01, &[2.1, 4.1, 7.1], 0.0);
    let r = dad_align(&a, &b, Some((3.0, 5.0)));
    assert_eq!(r.basis, "window");
    assert_eq!(r.window_min, (3.0, 5.0));
    assert!((r.shift_min + 0.1).abs() < 0.005, "{}", r.shift_min);
    // a window too short to correlate falls back to the whole run
    assert_eq!(dad_align(&a, &b, Some((3.0, 3.1))).basis, "run");
}

#[test]
fn a_flat_run_has_nothing_to_align_on_and_says_so() {
    let a = run(0.01, &[], 0.0);
    let r = dad_align(&a, &a, None);
    assert_eq!(r.shift_min, 0.0);
    assert_eq!(r.warnings, vec!["shift_auto_flat"]);
}

#[test]
fn a_shift_beyond_the_search_gives_a_weak_correlation() {
    let a = run(0.01, &[2.0, 5.0, 8.0], 0.0);
    let b = run(0.01, &[2.9, 5.9, 8.9], 0.0);
    let r = dad_align(&a, &b, None);
    assert!(r.warnings.contains(&"shift_auto_weak"), "{r:?}");
}

#[test]
fn the_json_has_stable_keys() {
    let a = run(0.02, &[3.0, 6.0], 0.0);
    let j = dad_align(&a, &a, None).to_json();
    for k in [
        "method",
        "shift_min",
        "corr",
        "corr_at_zero",
        "search_min",
        "basis",
        "window_min",
        "points",
        "step_min",
        "warnings",
    ] {
        assert!(j.contains(&format!("\"{k}\":")), "{k} in {j}");
    }
}
