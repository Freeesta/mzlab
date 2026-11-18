//! `detect_prominence` on a noise-free Gaussian on a zero baseline: the default limits keep its area (study item B2); the old limit is `frac` 0.05.

use mzlab_core::peakq::{detect_prominence, ProminenceParams};

#[test]
fn a_clean_gaussian_keeps_its_area() {
    for sigma in [2.5_f64, 5.0, 10.0, 20.0] {
        let rt: Vec<f64> = (0..3000).map(|i| i as f64 * 0.01).collect();
        let y: Vec<f64> = rt
            .iter()
            .map(|t| 1e5 * (-0.5 * ((t - 15.0) / (sigma * 0.01)).powi(2)).exp())
            .collect();
        let truth = 1e5 * sigma * 0.01 * (2.0 * std::f64::consts::PI).sqrt();
        let area = |frac: f64| {
            detect_prominence(
                &rt,
                &y,
                ProminenceParams {
                    frac,
                    ..Default::default()
                },
            )[0]
            .area
                / truth
        };
        let (new, old) = (area(ProminenceParams::default().frac), area(0.05));
        assert!((new - 1.0).abs() < 0.005, "sigma {sigma}: {new}");
        assert!((old - 0.876).abs() < 0.003, "sigma {sigma}: {old}");
    }
}
