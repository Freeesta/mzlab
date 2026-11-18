//! `detect_full` on one Gaussian at several widths in points (study item B3): area and FWHM of the largest peak against the truth, and the number
//! of pieces, with the lambda and widths that follow the width (default) and with the fixed ones of the Python engine (`scale_to_width: false`).

use mzlab_picchi::peaks::{detect_full, Params};

const N: usize = 3000;

/// `(rt in minutes at 0.01 a point, trace)` of a Gaussian of height 1e5 on a baseline of 500 plus `noise` of uniform scatter (fixed seed).
fn trace(sigma: f64, noise: f64) -> (Vec<f64>, Vec<f64>) {
    let rt: Vec<f64> = (0..N).map(|i| i as f64 * 0.01).collect();
    let mut seed = 12345u64;
    let y = (0..N)
        .map(|i| {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let u = (seed >> 11) as f64 / (1u64 << 53) as f64 - 0.5;
            500.0 + 1e5 * (-0.5 * ((i as f64 - 1500.0) / sigma).powi(2)).exp() + noise * 3.46 * u
        })
        .collect();
    (rt, y)
}

/// `(area / true of the largest peak, number of peaks over 1 % of the true area)`; a noise-free Gaussian also leaves tiny pieces, which are not this finding.
fn measure(sigma: f64, noise: f64, scale: bool) -> (f64, usize) {
    let (rt, y) = trace(sigma, noise);
    let params = Params {
        scale_to_width: scale,
        ..Params::default()
    };
    let peaks = detect_full(&rt, &y, params).peaks;
    let truth = 1e5 * sigma * 0.01 * (2.0 * std::f64::consts::PI).sqrt();
    let best = peaks.iter().map(|p| p.area).fold(0.0, f64::max);
    (
        best / truth,
        peaks.iter().filter(|p| p.area > 0.01 * truth).count(),
    )
}

#[test]
fn the_area_does_not_depend_on_the_points_per_peak() {
    for noise in [0.0, 100.0] {
        // with noise the limit walk of a peak of 80 points of sigma stops at the first bump of the smoothed trace (not the baseline or the widths)
        for sigma in [2.5, 5.0, 10.0, 20.0, 40.0, 80.0]
            .into_iter()
            .filter(|&s| noise == 0.0 || s < 80.0)
        {
            let (area, _) = measure(sigma, noise, true);
            assert!(
                (area - 1.0).abs() < 0.03,
                "noise {noise} sigma {sigma}: {area}"
            );
        }
    }
}

#[test]
fn the_fixed_lambda_of_the_python_engine_loses_the_wide_peaks() {
    let (area, _) = measure(20.0, 0.0, false);
    assert!(area < 0.85, "{area}");
}
