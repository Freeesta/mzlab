//! `peakq_auto::auto_integrate` (one-click integration of every peak of an HR trace) on synthetic traces with known areas: several peaks,
//! valley, shoulder, tailing, noisy and drifting baseline, nothing.

use crate::peakq::ProminenceParams;
use crate::peakq_auto::{auto_integrate, AutoParams, AutoPeak};

/// Deterministic noise in [-1, 1).
fn lcg(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*seed >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
}

/// 6 minutes at 0.015 min: Gaussians `(centre, sigma, height)` on a baseline `base(t)`, plus the noise.
fn trace_on(
    parts: &[(f64, f64, f64)],
    noise: f64,
    base: impl Fn(f64) -> f64,
) -> (Vec<f64>, Vec<f64>) {
    let rt: Vec<f64> = (0..400).map(|i| i as f64 * 0.015).collect();
    let mut seed = 7u64;
    let y = rt
        .iter()
        .map(|&t| {
            parts
                .iter()
                .map(|(c, s, h)| h * (-((t - c).powi(2)) / (2.0 * s * s)).exp())
                .sum::<f64>()
                + noise * lcg(&mut seed)
                + base(t)
        })
        .collect();
    (rt, y)
}

fn trace(parts: &[(f64, f64, f64)], noise: f64) -> (Vec<f64>, Vec<f64>) {
    trace_on(parts, noise, |_| 10.0 * noise)
}

fn auto(rt: &[f64], y: &[f64], x0: f64, x1: f64) -> Vec<AutoPeak> {
    auto_integrate(rt, y, x0, x1, AutoParams::default())
}

fn area(s: f64, h: f64) -> f64 {
    h * s * (2.0 * std::f64::consts::PI).sqrt()
}

fn close(got: f64, want: f64, tol: f64) -> bool {
    (got - want).abs() <= tol * want
}

#[test]
fn auto_integrate_one_window_per_peak_with_its_area() {
    let parts = [(1.0, 0.04, 8.0e4), (2.5, 0.05, 2.0e4), (4.5, 0.06, 1.0e5)];
    let (rt, y) = trace(&parts, 50.0);
    let w = auto(&rt, &y, 0.0, 6.0);
    assert_eq!(w.len(), 3, "{w:?}");
    for (k, (c, s, h)) in parts.iter().enumerate() {
        assert!(
            rt[w[k].lo] < *c && rt[w[k].hi] > *c,
            "window {k} misses {c}"
        );
        assert!(
            close(w[k].area, area(*s, *h), 0.03),
            "area {k}: {} vs {}",
            w[k].area,
            area(*s, *h)
        );
        assert!(
            close(w[k].base_lo, 500.0, 0.2) && close(w[k].base_hi, 500.0, 0.2),
            "baseline {k}: {w:?}"
        );
    }
    assert!(
        w.windows(2).all(|p| p[0].hi <= p[1].lo),
        "sorted and apart: {w:?}"
    );
    // only the apexes inside the time window, whichever way round it is given; the background is that of the whole trace
    assert_eq!(auto(&rt, &y, 3.0, 0.5), w[..2].to_vec());
}

#[test]
fn auto_integrate_double_shares_the_valley_and_shoulder_is_one() {
    let (rt, y) = trace(&[(2.5, 0.06, 1.0e5), (2.5 + 3.4 * 0.06, 0.06, 0.7e5)], 50.0);
    let w = auto(&rt, &y, 0.0, 6.0);
    assert_eq!(w.len(), 2, "{w:?}");
    assert_eq!(w[0].hi, w[1].lo, "the two windows share the valley");
    let total = area(0.06, 1.0e5) + area(0.06, 0.7e5);
    assert!(
        close(w[0].area + w[1].area, total, 0.03),
        "{} vs {total}",
        w[0].area + w[1].area
    );
    let (rt, y) = trace(&[(2.5, 0.06, 1.0e5), (2.5 + 1.4 * 0.06, 0.06, 0.4e5)], 50.0);
    let w = auto(&rt, &y, 0.0, 6.0);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(
        rt[w[0].hi] > 2.5 + 1.4 * 0.06,
        "the shoulder is inside the window"
    );
    assert!(
        close(w[0].area, area(0.06, 1.4e5), 0.03),
        "the group area: {}",
        w[0].area
    );
}

#[test]
fn auto_integrate_noisy_drifting_baseline_and_nothing() {
    let (rt, y) = trace(&[(3.0, 0.06, 5.0e3)], 200.0);
    let w = auto(&rt, &y, 0.0, 6.0);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(
        rt[w[0].lo] < 3.0 && rt[w[0].hi] > 3.0 && close(w[0].area, area(0.06, 5.0e3), 0.15),
        "{w:?}"
    );
    let (rt, y) = trace(&[], 200.0);
    assert!(auto(&rt, &y, 0.0, 6.0).is_empty());
    // a sloping baseline: the line between the two baseline ends follows it
    let (rt, y) = trace_on(&[(3.0, 0.05, 4.0e4)], 50.0, |t| 2.0e3 + 1.5e3 * t);
    let w = auto(&rt, &y, 0.0, 6.0);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(
        close(w[0].area, area(0.05, 4.0e4), 0.03),
        "{} vs {}",
        w[0].area,
        area(0.05, 4.0e4)
    );
    assert!(w[0].base_hi > w[0].base_lo, "{w:?}");
    // a higher S/N threshold drops the small peak; an even smoothing window or a zero width are taken as the nearest sensible ones
    let (rt, y) = trace(&[(1.0, 0.05, 1.0e5), (4.0, 0.05, 3.0e3)], 50.0);
    assert_eq!(auto(&rt, &y, 0.0, 6.0).len(), 2);
    let pick = ProminenceParams {
        snr: 100.0,
        smooth: 4,
        min_scans: 0,
        local_snr: 5.0,
        ..Default::default()
    };
    assert_eq!(
        auto_integrate(
            &rt,
            &y,
            0.0,
            6.0,
            AutoParams {
                pick,
                half_window: 3,
                ..Default::default()
            }
        )
        .len(),
        1
    );
}

#[test]
fn auto_integrate_tailing_peak_keeps_its_tail() {
    // exponentially modified Gaussian (tau = 1.6 sigma) as a running sum; the area is that of the Gaussian
    let rt: Vec<f64> = (0..400).map(|i| i as f64 * 0.015).collect();
    let (s, tau, h) = (0.05, 0.08, 6.0e4);
    let g: Vec<f64> = rt
        .iter()
        .map(|t| h * (-((t - 2.5).powi(2)) / (2.0 * s * s)).exp())
        .collect();
    let k = (-0.015f64 / tau).exp();
    let mut acc = 0.0;
    let mut seed = 3u64;
    let y: Vec<f64> = g
        .iter()
        .map(|v| {
            acc = acc * k + v * (1.0 - k);
            acc + 500.0 + 50.0 * lcg(&mut seed)
        })
        .collect();
    let w = auto(&rt, &y, 0.0, 6.0);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(
        close(w[0].area, area(s, h), 0.04),
        "{} vs {}",
        w[0].area,
        area(s, h)
    );
    assert!(
        rt[w[0].hi] - 2.5 > 2.5 - rt[w[0].lo],
        "the window reaches further on the tail side"
    );
}
