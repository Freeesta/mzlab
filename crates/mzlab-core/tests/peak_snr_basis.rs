//! The S/N of `peak1d::find_peak`: height over 1.4826 MAD of the raw trace far from the peak (the default), against the old smoothed-trace noise
//! (`SnrBasis::Smoothed`), which reads 1.6-1.8 times too high at the default smoothing of 3.

use mzlab_core::peak1d::{find_peak, find_peak_basis, SnrBasis};

/// Deterministic standard normal noise (LCG + Box-Muller).
fn normal(seed: &mut u64) -> f64 {
    let mut u = || {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((*seed >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    };
    let (a, b) = (u(), u());
    (-2.0 * a.ln()).sqrt() * (2.0 * std::f64::consts::PI * b).cos()
}

/// A Gaussian of height 1000 (sigma 3 points) on noise SD 10: true height / sigma_raw = 100.
fn trace(seed: u64) -> (Vec<f64>, Vec<f64>) {
    let mut s = seed;
    let dt = 0.8 / 60.0;
    let rt: Vec<f64> = (0..600).map(|i| i as f64 * dt).collect();
    let y = rt
        .iter()
        .map(|t| 1000.0 * (-0.5 * ((t - 4.0) / (3.0 * dt)).powi(2)).exp() + 10.0 * normal(&mut s))
        .collect();
    (rt, y)
}

fn median_snr(smooth: usize, basis: Option<SnrBasis>) -> f64 {
    let mut v: Vec<f64> = (1..=21)
        .map(|seed| {
            let (rt, y) = trace(seed);
            match basis {
                Some(b) => find_peak_basis(&rt, &y, None, 0.25, smooth, 3, b),
                None => find_peak(&rt, &y, None, 0.25, smooth, 3),
            }
            .unwrap()
            .snr
        })
        .collect();
    mzlab_core::sortc::slice_by(&mut v, |a, b| a.total_cmp(b));
    v[10]
}

#[test]
fn default_snr_reads_the_raw_noise() {
    for smooth in [1, 3, 5] {
        let sn = median_snr(smooth, None);
        assert!((85.0..=110.0).contains(&sn), "smoothing {smooth}: S/N {sn}");
    }
}

#[test]
fn the_old_smoothed_basis_stays_selectable() {
    let old = median_snr(3, Some(SnrBasis::Smoothed));
    assert!(old > 140.0, "smoothed basis {old}");
    assert_eq!(SnrBasis::parse("smoothed"), Some(SnrBasis::Smoothed));
    assert_eq!(SnrBasis::parse("raw"), Some(SnrBasis::Raw));
    assert_eq!(SnrBasis::parse("x"), None);
}
