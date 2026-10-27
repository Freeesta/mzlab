//! Peaks on made-up traces with known truth (Gaussians, minutes): the area of `detect_prominence` on a sloped baseline and with a close
//! neighbour, and the FWHM of `tracestat::detect` (the path of the ion-origin evidence) when a neighbour's valley sits above half height.

use mzlab_core::peakq::{detect_prominence, ProminenceParams};
use mzlab_core::tracestat::detect;

const SIGMA: f64 = 0.05;
const FWHM: f64 = 2.354_820_045 * SIGMA;

fn g(t: f64, c: f64, a: f64) -> f64 {
    a * (-(t - c).powi(2) / (2.0 * SIGMA * SIGMA)).exp()
}

fn area(a: f64) -> f64 {
    a * SIGMA * (2.0 * std::f64::consts::PI).sqrt()
}

fn trace(f: impl Fn(f64) -> f64) -> (Vec<f64>, Vec<f64>) {
    let rt: Vec<f64> = (0..400).map(|i| f64::from(i) * 0.01).collect();
    let y = rt.iter().map(|&t| f(t)).collect();
    (rt, y)
}

/// Area of the prominence peak whose apex is nearest `c`, as a ratio to the true area.
fn area_ratio(rt: &[f64], y: &[f64], c: f64, a: f64) -> f64 {
    let pk = detect_prominence(rt, y, ProminenceParams::default());
    let p = pk
        .iter()
        .min_by(|p, q| {
            (rt[p.apex_i] - c)
                .abs()
                .total_cmp(&(rt[q.apex_i] - c).abs())
        })
        .expect("a peak");
    p.area / area(a)
}

fn fwhm_ratio(rt: &[f64], y: &[f64], c: f64) -> f64 {
    let ps = detect(rt, y);
    let p = ps
        .iter()
        .min_by(|p, q| {
            (rt[p.p.apex_i] - c)
                .abs()
                .total_cmp(&(rt[q.p.apex_i] - c).abs())
        })
        .expect("a peak");
    p.e.fwhm_min / FWHM
}

/// The valley of two peaks sits above half height of the smaller: its FWHM stays inside its own limits (it was 2.1-2.5 times the true one).
#[test]
fn fwhm_stays_inside_the_limits_with_a_close_neighbour() {
    for d in [0.15, 0.16] {
        let (rt, y) = trace(|t| g(t, 2.0, 1000.0) + g(t, 2.0 + d, 700.0));
        for c in [2.0, 2.0 + d] {
            let r = fwhm_ratio(&rt, &y, c);
            assert!((0.9..1.5).contains(&r), "d {d} apex {c}: FWHM ratio {r}");
        }
    }
}

/// A far neighbour does not force a horizontal baseline: the limit is a straight line unless it IS the valley shared with the neighbour.
#[test]
fn area_of_a_peak_next_to_a_far_neighbour() {
    let (rt, y) = trace(|t| 300.0 - 200.0 * t + g(t, 1.0, 1000.0) + g(t, 3.0, 1000.0));
    let r = area_ratio(&rt, &y, 1.0, 1000.0);
    assert!((0.95..1.05).contains(&r), "area ratio {r}");
}

/// Neighbours that overlap keep the drop line: both areas stay below the true ones, never above.
#[test]
fn overlapping_peaks_keep_the_drop_line() {
    for d in [0.15, 0.16, 0.18] {
        let (rt, y) = trace(|t| g(t, 2.0, 1000.0) + g(t, 2.0 + d, 700.0));
        for (c, a) in [(2.0, 1000.0), (2.0 + d, 700.0)] {
            let r = area_ratio(&rt, &y, c, a);
            assert!((0.8..1.0).contains(&r), "d {d} apex {c}: area ratio {r}");
        }
    }
}

/// Complementary error function (Numerical Recipes `erfcc`, relative error below 1.2e-7).
fn erfc(x: f64) -> f64 {
    let z = x.abs();
    let t = 1.0 / (1.0 + 0.5 * z);
    let poly = -z * z - 1.265_512_23
        + t * (1.000_023_68
            + t * (0.374_091_96
                + t * (0.096_784_18
                    + t * (-0.186_288_06
                        + t * (0.278_868_07
                            + t * (-1.135_203_98
                                + t * (1.488_515_87 + t * (-0.822_152_23 + t * 0.170_872_77))))))));
    let r = t * poly.exp();
    if x >= 0.0 {
        r
    } else {
        2.0 - r
    }
}

/// Exponentially modified Gaussian: a narrow peak with a steep rise and a longer right tail.
fn emg(t: f64, c: f64, tau: f64, a: f64) -> f64 {
    a / (2.0 * tau)
        * (SIGMA * SIGMA / (2.0 * tau * tau) - (t - c) / tau).exp()
        * erfc((SIGMA * SIGMA / tau - (t - c)) / (SIGMA * 2f64.sqrt()))
}

/// One asymmetric peak (sigma 0.05 min, right tail) with counting noise gets exactly one label: the ripple on its rising side or top was
/// a second peak in some of these 200 noisy copies.
#[test]
fn one_asymmetric_peak_is_one_peak() {
    for (dt, tau, amp) in [
        (0.01, 0.05, 300.0),
        (0.01, 0.1, 300.0),
        (0.01, 0.1, 3000.0),
        (0.02, 0.1, 300.0),
    ] {
        for seed0 in 0..200u64 {
            let mut seed = seed0 * 7919 + 13;
            let mut rnd = || {
                seed = seed
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                (seed >> 33) as f64 / f64::from(1u32 << 31) - 0.5
            };
            let rt: Vec<f64> = (0..(20.0 / dt) as usize).map(|i| i as f64 * dt).collect();
            let y: Vec<f64> = rt
                .iter()
                .map(|&t| {
                    let m = emg(t, 11.2, tau, amp) * 0.125 + 20.0;
                    let noise: f64 = (0..12).map(|_| rnd() + 0.5).sum::<f64>() - 6.0;
                    (m + m.sqrt() * noise).max(0.0).round()
                })
                .collect();
            let params = ProminenceParams {
                local_snr: 5.0,
                ..Default::default()
            };
            let near = detect_prominence(&rt, &y, params)
                .iter()
                .filter(|p| (rt[p.apex_i] - 11.25).abs() < 0.5)
                .count();
            assert_eq!(near, 1, "dt {dt} tau {tau} amp {amp} seed {seed0}");
        }
    }
}
