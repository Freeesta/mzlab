//! Width and noise of the low-resolution peak routes against the analytic Gaussian: `peak1d::find_peak` FWHM = 2.3548 sigma read at half
//! height by linear interpolation, and the S/N gate of `peakq_lr::lr_edges` against the true height over the true noise sigma (the residual
//! of a Savitzky-Golay smoothing has SD sigma x sqrt(1 - c0), so it must be scaled back).

use mzlab_core::peak1d::find_peak;
use mzlab_core::peakq_lr::{lr_edges, LrParams};

const FWHM_PER_SIGMA: f64 = 2.354_820_045;

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

/// `n` points `dt` minutes apart, a Gaussian of `height` and `sigma` minutes at `centre`, plus `noise` x N(0, 1).
fn gauss(
    n: usize,
    dt: f64,
    centre: f64,
    sigma: f64,
    height: f64,
    noise: f64,
    seed: u64,
) -> (Vec<f64>, Vec<f64>) {
    let mut s = seed;
    let rt: Vec<f64> = (0..n).map(|i| i as f64 * dt).collect();
    let y = rt
        .iter()
        .map(|t| height * (-0.5 * ((t - centre) / sigma).powi(2)).exp() + noise * normal(&mut s))
        .collect();
    (rt, y)
}

#[test]
fn find_peak_fwhm_is_interpolated_at_half_height() {
    // dt 0.8 s; sigma 1.5, 3 and 6 points; the apex on a point and between two points
    let dt = 0.8 / 60.0;
    for sig_pts in [1.5, 3.0, 6.0] {
        for phase in [0.0, 0.25, 0.5] {
            let sigma = sig_pts * dt;
            let (rt, y) = gauss(400, dt, (200.0 + phase) * dt, sigma, 1e4, 0.0, 1);
            let f = find_peak(&rt, &y, None, 0.25, 1, 3).unwrap();
            let truth = FWHM_PER_SIGMA * sigma * 60.0;
            let rel = f.fwhm / truth - 1.0;
            // at 1.5 points per sigma the sampled apex sits up to 5 % under the true one, which lowers the half level
            let tol = if sig_pts < 2.0 { 0.07 } else { 0.02 };
            assert!(
                rel.abs() < tol,
                "sigma {sig_pts} points, phase {phase}: FWHM {} s against {truth} s ({rel:+.3})",
                f.fwhm
            );
        }
    }
}

/// The S/N `lr_edges` sees: the largest `sn_min` that still keeps the peak (bisection).
fn gate_sn(rt: &[f64], y: &[f64]) -> f64 {
    let at = |sn: f64| {
        lr_edges(
            rt,
            y,
            6.0,
            0.0,
            false,
            &LrParams {
                sn_min: sn,
                ..Default::default()
            },
        )
        .is_some()
    };
    let (mut lo, mut hi) = (1.0, 1000.0);
    for _ in 0..40 {
        let m = 0.5 * (lo + hi);
        if at(m) {
            lo = m;
        } else {
            hi = m;
        }
    }
    lo
}

#[test]
fn lr_edges_sn_gate_uses_the_true_noise() {
    // true S/N = 100 (height 1e4, sigma 100) on 20 noise draws; the median S/N of the gate is within 10 %. Under 3 points per sigma the
    // smoothed apex itself is lower than the true one (0.88 at 1 point), which is the height convention, not the noise.
    let dt = 0.01;
    for sig_pts in [3.0, 10.0, 25.0] {
        let mut v: Vec<f64> = (1..=20u64)
            .map(|seed| {
                let (rt, y) = gauss(1200, dt, 6.0, sig_pts * dt, 1e4, 100.0, seed);
                gate_sn(&rt, &y)
            })
            .collect();
        mzlab_core::sortc::slice_by(&mut v, |a, b| a.total_cmp(b));
        let med = 0.5 * (v[9] + v[10]);
        assert!(
            (med / 100.0 - 1.0).abs() < 0.1,
            "sigma {sig_pts} points: median S/N {med:.1} against 100"
        );
    }
}
