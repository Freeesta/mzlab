//! The click on a low-resolution trace: nearest apex, edges, evidence and the junk guard (synthetic traces only).
use mzlab_core::peakq_click::{lr_click, ClickGuard};
use mzlab_core::peakq_lr::LrParams;

/// 0.02 min sampling from 10 to 14 min; Gaussians (t, height, sigma) on a flat 50 count level with deterministic noise of about `noise` counts.
fn trace(peaks: &[(f64, f64, f64)], noise: f64) -> (Vec<f64>, Vec<f64>) {
    let mut seed = 12345u64;
    let mut rnd = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((seed >> 33) as f64 / (1u64 << 31) as f64) - 0.5
    };
    let xs: Vec<f64> = (0..200).map(|i| 10.0 + i as f64 * 0.02).collect();
    let ys = xs
        .iter()
        .map(|&t| {
            50.0 + noise * 3.4 * rnd()
                + peaks
                    .iter()
                    .map(|&(c, h, s)| h * (-((t - c) / s).powi(2) / 2.0).exp())
                    .sum::<f64>()
        })
        .collect();
    (xs, ys)
}

fn click(xs: &[f64], ys: &[f64], at: f64, g: ClickGuard) -> mzlab_core::peakq_click::Click {
    lr_click(xs, ys, at, true, &LrParams::default(), &g)
}

#[test]
fn a_real_peak_is_kept_with_its_evidence() {
    let (x, y) = trace(&[(11.7, 5000.0, 0.06)], 10.0);
    let c = click(&x, &y, 11.72, ClickGuard::default());
    assert!(
        c.ok() && c.refused.is_empty() && !c.forced,
        "{}",
        c.to_json()
    );
    let (a, b) = c.edges.unwrap();
    assert!(
        a < 11.7 && b > 11.7 && c.points >= 5 && c.sn > 100.0,
        "{}",
        c.to_json()
    );
    assert!((c.apex.unwrap() - 11.7).abs() < 0.03);
}

#[test]
fn the_apex_nearest_the_click_wins_over_the_taller_one() {
    let (x, y) = trace(&[(11.5, 800.0, 0.05), (11.7, 5000.0, 0.05)], 5.0);
    let near_small = click(&x, &y, 11.52, ClickGuard::default());
    assert!(
        (near_small.apex.unwrap() - 11.5).abs() < 0.03,
        "{}",
        near_small.to_json()
    );
    let near_big = click(&x, &y, 11.69, ClickGuard::default());
    assert!(
        (near_big.apex.unwrap() - 11.7).abs() < 0.03,
        "{}",
        near_big.to_json()
    );
}

#[test]
fn noise_is_refused_with_its_reasons_and_can_be_forced() {
    let (x, y) = trace(&[], 20.0);
    let c = click(&x, &y, 11.72, ClickGuard::default());
    assert!(
        !c.ok() && c.refused[0] == "no_peak" && c.apex.is_none(),
        "{}",
        c.to_json()
    );
    assert!(
        c.edges.is_some(),
        "the evidence is still given: {}",
        c.to_json()
    );
    let f = click(
        &x,
        &y,
        11.72,
        ClickGuard {
            force: true,
            ..Default::default()
        },
    );
    assert!(
        f.ok() && f.forced && f.refused[0] == "no_peak" && f.edges == c.edges,
        "{}",
        f.to_json()
    );
}

#[test]
fn a_narrow_spike_is_too_few_points_and_the_guard_numbers_are_logged() {
    let (x, mut y) = trace(&[], 3.0);
    y[86] += 400.0;
    y[87] += 380.0;
    let c = click(
        &x,
        &y,
        x[86],
        ClickGuard {
            min_points: 9,
            ..Default::default()
        },
    );
    assert!(c.refused.contains(&"points") && !c.ok(), "{}", c.to_json());
    let j = c.to_json();
    assert!(
        j.contains("\"params\":{\"min_points\":9,\"min_sn\":3,\"reach\":0.25,\"force\":false}"),
        "{j}"
    );
}

#[test]
fn a_low_sn_hump_is_refused_by_sn_until_the_limit_is_lowered() {
    let (x, y) = trace(&[(11.7, 30.0, 0.08)], 12.0);
    let c = click(
        &x,
        &y,
        11.7,
        ClickGuard {
            min_sn: 50.0,
            ..Default::default()
        },
    );
    assert!(c.refused.contains(&"sn") && !c.ok(), "{}", c.to_json());
    assert!(click(
        &x,
        &y,
        11.7,
        ClickGuard {
            min_sn: 0.0,
            min_points: 0,
            ..Default::default()
        }
    )
    .refused
    .iter()
    .all(|r| *r != "sn"));
}

#[test]
fn a_short_trace_has_no_peak() {
    let c = click(&[1.0, 2.0], &[1.0, 2.0], 1.5, ClickGuard::default());
    assert!(!c.ok() && c.refused == ["no_peak"]);
    // nothing to keep without a window: forcing cannot make it ok
    let f = click(
        &[1.0, 2.0],
        &[1.0, 2.0],
        1.5,
        ClickGuard {
            force: true,
            ..ClickGuard::default()
        },
    );
    assert!(!f.ok() && !f.forced && f.edges.is_none(), "{f:?}");
}
