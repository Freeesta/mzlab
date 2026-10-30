//! `peakq::auto_limits` (one-click integration of every peak of an HR trace) on synthetic traces: several peaks, valley, shoulder, noise, nothing.

use crate::peakq::{auto_limits, ProminenceParams};

/// Deterministic noise in [-1, 1).
fn lcg(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*seed >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
}

/// 6 minutes at 0.015 min: a sum of Gaussians `(centre, sigma, height)` on a baseline of 10 times the noise, plus the noise.
fn trace(parts: &[(f64, f64, f64)], noise: f64) -> (Vec<f64>, Vec<f64>) {
    let rt: Vec<f64> = (0..400).map(|i| i as f64 * 0.015).collect();
    let mut seed = 7u64;
    let y = rt
        .iter()
        .map(|t| {
            parts
                .iter()
                .map(|(c, s, h)| h * (-((t - c).powi(2)) / (2.0 * s * s)).exp())
                .sum::<f64>()
                + noise * lcg(&mut seed)
                + 10.0 * noise
        })
        .collect();
    (rt, y)
}

/// The parameters of the one-click integration: those of `detect_peaks` mode 1 of mzlab-wasm.
fn auto(rt: &[f64], y: &[f64], x0: f64, x1: f64) -> Vec<(usize, usize)> {
    auto_limits(
        rt,
        y,
        x0,
        x1,
        ProminenceParams {
            local_snr: 5.0,
            ..Default::default()
        },
    )
}

#[test]
fn auto_limits_one_window_per_peak() {
    let (rt, y) = trace(
        &[(1.0, 0.04, 8.0e4), (2.5, 0.05, 2.0e4), (4.5, 0.06, 1.0e5)],
        50.0,
    );
    let w = auto(&rt, &y, 0.0, 6.0);
    assert_eq!(w.len(), 3, "{w:?}");
    for (k, c) in [1.0, 2.5, 4.5].iter().enumerate() {
        assert!(
            rt[w[k].0] < *c && rt[w[k].1] > *c,
            "window {k} {:?} misses {c}",
            (rt[w[k].0], rt[w[k].1])
        );
    }
    assert!(
        w.windows(2).all(|p| p[0].1 <= p[1].0),
        "sorted and apart: {w:?}"
    );
    // only the apexes inside the time window, whichever way round it is given
    assert_eq!(auto(&rt, &y, 3.0, 0.5), w[..2].to_vec());
}

#[test]
fn auto_limits_double_touch_at_the_valley_and_shoulder_is_one() {
    let (rt, y) = trace(&[(2.5, 0.06, 1.0e5), (2.5 + 3.4 * 0.06, 0.06, 0.7e5)], 50.0);
    let w = auto(&rt, &y, 0.0, 6.0);
    assert_eq!(w.len(), 2, "{w:?}");
    assert_eq!(w[0].1, w[1].0, "the two windows share the valley");
    let (rt, y) = trace(&[(2.5, 0.06, 1.0e5), (2.5 + 1.4 * 0.06, 0.06, 0.4e5)], 50.0);
    let w = auto(&rt, &y, 0.0, 6.0);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(
        rt[w[0].1] > 2.5 + 1.4 * 0.06,
        "the shoulder is inside the window"
    );
}

#[test]
fn auto_limits_noisy_baseline_one_peak_and_nothing() {
    let (rt, y) = trace(&[(3.0, 0.06, 5.0e3)], 200.0);
    let w = auto(&rt, &y, 0.0, 6.0);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(rt[w[0].0] < 3.0 && rt[w[0].1] > 3.0);
    let (rt, y) = trace(&[], 200.0);
    assert!(auto(&rt, &y, 0.0, 6.0).is_empty());
    // a higher S/N threshold drops the small peak; an even smoothing window or a zero width are taken as the nearest sensible ones
    let (rt, y) = trace(&[(1.0, 0.05, 1.0e5), (4.0, 0.05, 3.0e3)], 50.0);
    assert_eq!(auto(&rt, &y, 0.0, 6.0).len(), 2);
    let strict = ProminenceParams {
        snr: 100.0,
        smooth: 4,
        min_scans: 0,
        local_snr: 5.0,
        ..Default::default()
    };
    assert_eq!(auto_limits(&rt, &y, 0.0, 6.0, strict).len(), 1);
}
