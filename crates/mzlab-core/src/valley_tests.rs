//! Two Gaussians that overlap split at their valley and their parts add up to the whole window; a shoulder and the noise of a tail never split; a
//! deeper depth asks for a deeper valley; Hp/Hv is the height of a part over its valley.

use super::*;

fn gauss(t: f64, at: f64, h: f64, w: f64) -> f64 {
    h * (-((t - at) / w).powi(2) / 2.0).exp()
}

/// Times every 0.01 min over 0..=2 and the sum of the peaks `(apex, height, sigma)`, plus `noise` (a fixed saw of ± noise).
fn trace(peaks: &[(f64, f64, f64)], noise: f64) -> (Vec<f64>, Vec<f64>) {
    let t: Vec<f64> = (0..=200).map(|i| i as f64 / 100.0).collect();
    let y = t
        .iter()
        .enumerate()
        .map(|(i, &x)| {
            let saw = [0.0, 1.0, -1.0, 0.5, -0.5][i % 5] * noise;
            peaks
                .iter()
                .map(|&(a, h, w)| gauss(x, a, h, w))
                .sum::<f64>()
                + saw
        })
        .collect();
    (t, y)
}

#[test]
fn two_overlapping_peaks_split_at_the_valley_and_keep_the_area() {
    let (t, y) = trace(&[(0.8, 1000.0, 0.08), (1.2, 600.0, 0.08)], 0.0);
    let v = valleys(&y, 40, 160, DEPTH);
    assert_eq!(v.len(), 1);
    assert!(t[v[0]] > 0.95 && t[v[0]] < 1.1, "valley at {}", t[v[0]]);
    let whole = integrate(&t, &y, 40, 160, |_| 0.0).area;
    let p = parts(&t, &y, 40, 160, &v, |_| 0.0);
    assert_eq!(p.len(), 2);
    assert_eq!((p[0].lo, p[0].hi, p[1].lo, p[1].hi), (40, v[0], v[0], 160));
    let sum = p[0].area + p[1].area;
    assert!((sum - whole).abs() <= 1e-9 * whole, "{sum} vs {whole}");
    assert!((t[p[0].apex] - 0.8).abs() < 1e-9 && (t[p[1].apex] - 1.2).abs() < 1e-9);
    let hv = y[v[0]];
    assert_eq!(p[0].pv, Some(p[0].height / hv));
    assert_eq!(p[1].pv, Some(p[1].height / hv));
}

#[test]
fn a_sloping_line_is_shared_by_the_parts() {
    let (t, y) = trace(
        &[(0.6, 900.0, 0.1), (1.0, 900.0, 0.1), (1.4, 500.0, 0.1)],
        0.0,
    );
    let line = |x: f64| 50.0 + 20.0 * x;
    let d: Vec<f64> = t.iter().zip(&y).map(|(&x, &v)| v - line(x)).collect();
    let v = valleys(&d, 20, 180, DEPTH);
    assert_eq!(v.len(), 2, "three peaks, two valleys");
    let whole = integrate(&t, &y, 20, 180, line).area;
    let sum: f64 = parts(&t, &y, 20, 180, &v, line)
        .iter()
        .map(|p| p.area)
        .sum();
    assert!((sum - whole).abs() <= 1e-9 * whole.abs());
    let mid = parts(&t, &y, 20, 180, &v, line)[1];
    assert!(mid.pv.is_some(), "the middle part has two valleys");
}

#[test]
fn a_shoulder_is_not_split() {
    // the second peak sits on the tail of the first: no minimum between them
    let (_, y) = trace(&[(0.8, 1000.0, 0.1), (0.98, 300.0, 0.05)], 0.0);
    assert!(valleys(&y, 30, 170, 0.0).is_empty());
}

#[test]
fn the_noise_of_a_tail_does_not_split_but_a_depth_of_zero_does() {
    let (_, y) = trace(&[(1.0, 1000.0, 0.1)], 5.0);
    assert!(valleys(&y, 40, 160, DEPTH).is_empty());
    assert!(
        !valleys(&y, 40, 160, 0.0).is_empty(),
        "every wiggle is a valley at depth 0"
    );
}

#[test]
fn a_deeper_depth_asks_for_a_deeper_valley() {
    // valley at about 40 % of the smaller peak: split at 0.1, not at 0.5 of the highest point
    let (t, y) = trace(&[(0.8, 1000.0, 0.1), (1.12, 500.0, 0.1)], 0.0);
    let at = valleys(&y, 30, 170, 0.1);
    assert_eq!(at.len(), 1, "{at:?}");
    assert!(t[at[0]] > 0.8 && t[at[0]] < 1.12);
    assert!(valleys(&y, 30, 170, 0.5).is_empty());
    assert_eq!(depth_of(f64::NAN), DEPTH);
    assert_eq!(depth_of(3.0), 1.0);
}

#[test]
fn cuts_outside_the_window_are_ignored() {
    let (t, y) = trace(&[(1.0, 1000.0, 0.1)], 0.0);
    let p = parts(&t, &y, 50, 150, &[50, 150, 10, 190], |_| 0.0);
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].pv, None, "no valley, no Hp/Hv");
    assert!(valleys(&y, 50, 51, DEPTH).is_empty());
}
