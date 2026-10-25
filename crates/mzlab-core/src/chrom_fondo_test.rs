//! Tests of `chrom_fondo`: synthetic trace with a flat floor, a ramp and a peak on it.
use super::*;
use crate::chrom_fondo::*;

/// Deterministic pseudo-noise in [-1, 1].
fn noise(i: usize) -> f64 {
    let x = (i as f64 * 12.9898).sin() * 43758.5453;
    (x - x.floor()) * 2.0 - 1.0
}

/// Flat floor (first 40 %), rising ramp with a peak on it (60 %).
fn trace() -> (Vec<f64>, Vec<f64>) {
    let n = 400;
    let t: Vec<f64> = (0..n).map(|i| i as f64 * 0.05).collect();
    let y = (0..n)
        .map(|i| {
            let ramp = if i < 160 { 0.0 } else { (i - 160) as f64 * 2.0 };
            let peak = 500.0 * (-(((i as f64) - 300.0) / 5.0).powi(2) / 2.0).exp();
            20.0 + 2.0 * noise(i) + ramp + peak
        })
        .collect();
    (t, y)
}

#[test]
fn flat_window_falls_in_the_flat_zone() {
    let (t, y) = trace();
    let b = auto_background(&t, &y, None, &FondoParams::default()).unwrap();
    let (s, e) = b.window.unwrap();
    assert_eq!(b.origin, Origin::FlatWindow);
    assert!(s < 160 && e <= 160, "window {s}..{e} outside the flat zone");
    assert_eq!(e - s, 20);
    assert!((b.level[0] - 20.0).abs() < 2.0);
}

#[test]
fn blank_is_used_and_interpolated() {
    let (t, y) = trace();
    let bt = [0.0, 10.0, 19.95];
    let by = [10.0, 30.0, 30.0];
    let b = auto_background(&t, &y, Some((&bt, &by)), &FondoParams::default()).unwrap();
    assert_eq!(b.origin, Origin::Blank);
    assert!(b.window.is_none());
    assert!((b.level[0] - 10.0).abs() < 1e-9);
    assert!((b.level[100] - 20.0).abs() < 1e-9);
    assert!((b.level[399] - 30.0).abs() < 1e-9);
}

#[test]
fn short_or_bad_input_is_none() {
    let p = FondoParams::default();
    assert!(auto_background(&[0.0], &[1.0], None, &p).is_none());
    assert!(auto_background(&[0.0, 1.0], &[1.0, 2.0], None, &p).is_none());
    assert!(auto_background(&[1.0, 0.0], &[1.0, 2.0], None, &p).is_none());
}

#[test]
fn local_background_follows_the_ramp() {
    let (t, y) = trace();
    let l = local_background(&t, &y, 300, &FondoParams::default()).unwrap();
    assert!(l.left < 300 && l.right > 300);
    // the ramp under the apex is about 20 + 2 * 140 = 300: the corrected height is the Gaussian's 500, not the raw ~800
    assert!((l.height - 500.0).abs() < 40.0, "height {}", l.height);
    let gauss_area = 500.0 * 5.0 * (2.0 * std::f64::consts::PI).sqrt() * 0.05;
    assert!(
        (l.area - gauss_area).abs() / gauss_area < 0.15,
        "area {}",
        l.area
    );
}

#[test]
fn user_window_gives_its_median() {
    let (_, y) = trace();
    let b = background_at(&y, 0, 30).unwrap();
    assert_eq!(b.window, Some((0, 30)));
    assert!((b.level[5] - 20.0).abs() < 2.0);
    assert!(background_at(&y, 5, 6).is_none());
}
