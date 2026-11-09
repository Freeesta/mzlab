//! Golden tests of the low-resolution peak pipeline on synthetic MRM-like traces (noisy Gaussians sampled every 0.05 s):
//! one peak gets one label, the integrated area stays within 1 % of the analytic one, two overlapping peaks stay two, flat noise has none.

use mzlab_core::etichette_picchi::select;
use mzlab_core::peakq_lr::{lr_edges, LrParams};

const DT: f64 = 0.05 / 60.0;
const BASE: f64 = 50.0;

/// Deterministic noise in [-1, 1) (xorshift), so the test never flakes.
fn rng(seed: &mut u64) -> f64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed >> 11) as f64 / (1u64 << 52) as f64 - 1.0
}

fn trace(peaks: &[(f64, f64, f64)], noise: f64, seed: u64) -> (Vec<f64>, Vec<f64>) {
    let mut s = seed;
    let xs: Vec<f64> = (0..)
        .map(|i| 12.0 + i as f64 * DT)
        .take_while(|&t| t < 16.5)
        .collect();
    let ys = xs
        .iter()
        .map(|&t| {
            let g: f64 = peaks
                .iter()
                .map(|&(c, w, h)| h * (-0.5 * ((t - c) / w).powi(2)).exp())
                .sum();
            (g + BASE + noise * rng(&mut s)).max(0.0)
        })
        .collect();
    (xs, ys)
}

/// Trapezoid on the raw points between the edges, minus the known flat baseline of the synthetic trace (`BASE`): this checks that the edges
/// enclose the whole peak; the straight line between the edge values that the page subtracts depends on where the edges fall, not on the detection.
fn area(xs: &[f64], ys: &[f64], t0: f64, t1: f64) -> f64 {
    let idx: Vec<usize> = (0..xs.len())
        .filter(|&i| xs[i] >= t0 && xs[i] <= t1)
        .collect();
    let (i0, i1) = (idx[0], idx[idx.len() - 1]);
    let mut a = 0.0;
    for i in i0..i1 {
        a += 0.5 * (ys[i] + ys[i + 1]) * (xs[i + 1] - xs[i]);
    }
    a - BASE * (xs[i1] - xs[i0])
}

#[test]
fn noisy_gaussian_is_one_label_and_one_area() {
    let (c, w, h) = (14.33, 0.05, 6000.0);
    for seed in [1u64, 7, 99, 12345] {
        let (xs, ys) = trace(&[(c, w, h)], 60.0, seed + 0x9e3779b97f4a7c15);
        let labels = select(&xs, &ys, 12, 0.05, 5.0, false);
        assert_eq!(labels.len(), 1, "seed {seed}: {labels:?}");
        assert!((labels[0].rt - c).abs() < 0.5 * w, "apex {}", labels[0].rt);
        let (t0, t1) =
            lr_edges(&xs, &ys, labels[0].rt, 0.0, true, &LrParams::default()).expect("edges");
        assert!(t0 < c && t1 > c);
        let got = area(&xs, &ys, t0, t1);
        let exact = h * w * (2.0 * std::f64::consts::PI).sqrt();
        assert!(
            (got / exact - 1.0).abs() < 0.01,
            "seed {seed}: area {got} vs {exact}"
        );
    }
}

#[test]
fn two_overlapping_peaks_stay_two() {
    let (xs, ys) = trace(
        &[(14.10, 0.04, 6000.0), (14.30, 0.04, 5000.0)],
        300.0,
        0x1234_5678_9abc,
    );
    let labels = select(&xs, &ys, 12, 0.05, 5.0, false);
    assert_eq!(labels.len(), 2, "{labels:?}");
}

#[test]
fn flat_noise_has_no_label() {
    let (xs, ys) = trace(&[], 100.0, 0xfeed_beef_1234);
    assert!(select(&xs, &ys, 12, 0.05, 5.0, false).is_empty());
}

#[test]
fn heavy_noise_still_one_label_per_peak() {
    for seed in [3u64, 11, 5150, 777] {
        let (xs, ys) = trace(&[(14.33, 0.05, 6000.0)], 400.0, seed + 0x9e3779b97f4a7c15);
        let labels = select(&xs, &ys, 12, 0.05, 5.0, false);
        assert_eq!(labels.len(), 1, "seed {seed}: {labels:?}");
        assert!((labels[0].rt - 14.33).abs() < 0.05);
    }
}
