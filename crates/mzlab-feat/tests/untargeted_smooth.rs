//! Smoothing of a run: truncated windows, even windows, Savitzky-Golay never negative, weights summing to 1.

use mzlab_feat::untargeted::smooth::{runs, smooth, smooth_run, weights, window_size, Method};

#[test]
fn the_window_is_odd_and_at_least_3() {
    assert_eq!(
        (
            window_size(0),
            window_size(4),
            window_size(5),
            window_size(6)
        ),
        (3, 5, 5, 7)
    );
}

#[test]
fn the_weights_sum_to_one() {
    for m in [Method::Gaussian, Method::SavitzkyGolay] {
        for w in [3, 5, 9] {
            let s: f64 = weights(m, w).iter().sum();
            assert!((s - 1.0).abs() < 1e-12, "{m:?} {w}");
        }
    }
    // quadratic Savitzky-Golay, window 5: (17, 12, -3) / 35
    let sg = weights(Method::SavitzkyGolay, 5);
    let want = [-3.0, 12.0, 17.0, 12.0, -3.0].map(|v| v / 35.0);
    assert!(sg.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-12));
}

#[test]
fn the_gaussian_window_is_cut_at_the_ends_without_renormalising() {
    // window 3: sigma 0.5, weights (e^-2, 1, e^-2) / (1 + 2 e^-2)
    let e = (-2.0f64).exp();
    let sum = 1.0 + 2.0 * e;
    let out = smooth_run(&[1.0, 0.0, 0.0], 0, 3, Method::Gaussian, 3);
    assert!((out[0] - 1.0 / sum).abs() < 1e-12);
    assert!((out[1] - e / sum).abs() < 1e-12);
    assert_eq!(out[2], 0.0);
}

#[test]
fn an_even_window_becomes_the_next_odd_one() {
    let y = [0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 0.0];
    assert_eq!(
        smooth_run(&y, 0, 7, Method::Gaussian, 4),
        smooth_run(&y, 0, 7, Method::Gaussian, 5)
    );
}

#[test]
fn savitzky_golay_is_not_negative_and_the_run_is_cut() {
    let y = [0.0, 0.0, 10.0, 0.0, 0.0];
    let out = smooth_run(&y, 0, 5, Method::SavitzkyGolay, 5);
    assert!((out[2] - 10.0 * 17.0 / 35.0).abs() < 1e-12);
    assert_eq!(out[0], 0.0); // -30/35 clamped
    assert!(out.iter().all(|v| *v >= 0.0));
    // zeros outside [1, 3)
    let part = smooth_run(&[5.0, 5.0, 5.0, 5.0], 1, 3, Method::Gaussian, 3);
    assert_eq!((part[0], part[3]), (0.0, 0.0));
}

#[test]
fn the_runs_split_at_the_mz_jumps() {
    let mz = [100.0, 100.1, 105.0, 105.1, 105.2];
    assert_eq!(runs(&mz, 1.0), vec![(0, 2), (2, 5)]);
    let y = [1.0, 2.0, 3.0, 4.0, 5.0];
    assert_eq!(smooth(&mz, &y, 1.0, Method::None, 5), y.to_vec());
}
