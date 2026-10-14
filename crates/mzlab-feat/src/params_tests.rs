use super::*;
use mzlab_core::rng::Pcg64;

/// Uniform in (0, 1) from the generator.
fn u(r: &mut Pcg64) -> f64 {
    (r.next64() >> 11) as f64 / (1u64 << 53) as f64 + 1e-12
}

/// Standard normal by Box-Muller.
fn gauss(r: &mut Pcg64) -> f64 {
    libm::sqrt(-2.0 * libm::log(u(r))) * libm::cos(6.283185307179586 * u(r))
}

/// Noise: log-normal around `noise` (0.25 decade); signal: log-normal around 1e6 (0.6 decade), 2 % of the peaks.
fn synthetic(noise: f64, seed: u32) -> Hist {
    let mut r = Pcg64::new(seed);
    let mut h = Hist::new();
    for _ in 0..200_000 {
        h.add(libm::pow(10.0, libm::log10(noise) + 0.25 * gauss(&mut r)));
    }
    for _ in 0..4_000 {
        h.add(libm::pow(10.0, 6.0 + 0.6 * gauss(&mut r)));
    }
    h
}

#[test]
fn the_floor_is_within_30_percent_of_the_true_one() {
    // true floor: 3 sigma (0.25 decade) above the center of the noise
    for (noise, seed) in [(3e3, 1), (5e3, 2), (1e4, 3), (2e4, 4)] {
        let n = noise_from_hist(&synthetic(noise, seed)).unwrap();
        let want = noise * libm::pow(10.0, 3.0 * 0.25);
        assert!(
            (n.floor / want - 1.0).abs() <= 0.3,
            "noise {noise}: floor {} against {want} ({n:?})",
            n.floor
        );
        assert!((n.sigma - 0.25).abs() < 0.08, "sigma {}", n.sigma);
    }
}

#[test]
fn a_histogram_without_a_lump_gives_no_floor() {
    assert!(noise_from_hist(&Hist::new()).is_none());
    let mut flat = Hist::new();
    for i in 10..90 {
        flat.counts[i] = 100;
    }
    assert!(noise_from_hist(&flat).is_none());
}

#[test]
fn tolerance_is_three_robust_sigma_inside_the_bounds() {
    let mut r = Pcg64::new(7);
    let mut e: Vec<f64> = (0..500).map(|_| 0.8 * gauss(&mut r)).collect();
    e.extend([50.0, -40.0, 90.0]); // outliers must not move it
    let (ppm, sigma) = tolerance_from_errors(&mut e).unwrap();
    assert!(
        (sigma - 0.8).abs() < 0.15 && (ppm - 2.4).abs() < 0.5,
        "{ppm} {sigma}"
    );
    assert!(tolerance_from_errors(&mut [1.0, 2.0]).is_none());
    let mut wide: Vec<f64> = (0..100).map(|i| (i as f64 - 50.0) * 2.0).collect();
    assert_eq!(tolerance_from_errors(&mut wide).unwrap().0, PPM_MAX);
}

#[test]
fn fwhm_of_a_gaussian_trace() {
    let x: Vec<f64> = (0..200).map(|i| i as f64 * 0.01).collect();
    let s = 0.1;
    let y: Vec<f64> = x
        .iter()
        .map(|t| 1e6 * libm::exp(-0.5 * ((t - 1.0) / s) * ((t - 1.0) / s)))
        .collect();
    let w = fwhm_of_trace(&x, &y, 100).unwrap();
    assert!((w - 2.3548 * s).abs() < 0.01, "{w}");
    assert!(fwhm_of_trace(&x, &vec![1e6; 200], 100).is_none());
}

#[test]
fn the_window_of_mz_is_twice_the_measured_tolerance_inside_the_bounds() {
    let mut r = Pcg64::new(11);
    let mut e: Vec<f64> = (0..400).map(|_| 0.08 * gauss(&mut r)).collect();
    e.extend([2.0, -1.5]); // outliers must not move it
    let (da, sigma) = da_from_offsets(&mut e).unwrap();
    assert!(
        (sigma - 0.08).abs() < 0.02 && (da - 0.48).abs() < 0.12,
        "{da} {sigma}"
    );
    let mut tight: Vec<f64> = (0..50).map(|i| 0.001 * (i % 3) as f64).collect();
    assert_eq!(da_from_offsets(&mut tight).unwrap().0, DA_MIN);
    let mut wide: Vec<f64> = (0..400).map(|_| 0.5 * gauss(&mut r)).collect();
    assert_eq!(da_from_offsets(&mut wide).unwrap().0, DA_MAX);
    assert!(da_from_offsets(&mut [0.0; 5]).is_none());
}

/// A run of 20 minutes, a point every 0.1 min, a flat TIC of 1e5; `void` adds a front of height 1e6 that falls off over `width` minutes.
fn run(void: Option<(f64, f64)>) -> (Vec<f64>, Vec<f64>) {
    let rt: Vec<f64> = (0..200).map(|i| i as f64 * 0.1).collect();
    let tic = rt
        .iter()
        .map(|&t| {
            1e5 + void.map_or(0.0, |(at, w)| {
                if t >= at {
                    1e6 * libm::exp(-(t - at) / w)
                } else {
                    0.0
                }
            })
        })
        .collect();
    (rt, tic)
}

#[test]
fn the_void_front_ends_where_the_tic_comes_back() {
    let (rt, tic) = run(Some((0.6, 0.2)));
    let d = dead_time(&rt, &tic).unwrap();
    assert!((1.0..=2.0).contains(&d), "{d}");
    let (rt, tic) = run(None);
    assert!(dead_time(&rt, &tic).is_none());
    let (rt, tic) = run(Some((0.6, 5.0))); // a front that lasts beyond a third of the run is not a void
    assert!(dead_time(&rt, &tic).is_none());
}
