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
