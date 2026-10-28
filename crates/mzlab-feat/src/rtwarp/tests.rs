//! The windowed LOESS and the weight-free RANSAC refit against the verbatim previous code, bit for bit.
use super::*;

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// Anchors sorted by reference RT, with many repeated RTs and a few outliers.
fn random_points(rng: &mut Rng, n: usize, repeats: bool) -> Vec<(f64, f64)> {
    let mut pts: Vec<(f64, f64)> = (0..n)
        .map(|_| {
            let x = if repeats {
                (rng.unit() * 12.0).floor() * 0.5
            } else {
                rng.unit() * 20.0
            };
            let out = if rng.unit() < 0.05 { 0.4 } else { 0.0 };
            (
                x,
                0.1 * x + 0.02 * (x * 0.7).sin() + (rng.unit() - 0.5) * 0.03 + out,
            )
        })
        .collect();
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    pts
}

#[test]
fn loess_window_equals_the_full_scan_on_10000_curves() {
    let mut rng = Rng(0xC0FFEE);
    for case in 0..10_000 {
        let n = [1, 2, 3, 4, 7, 20, 60, 450][case % 8];
        let pts = random_points(&mut rng, n, case % 3 != 0);
        let grid: Vec<f64> = (0..NODES).map(|j| -1.0 + 22.0 * j as f64 / 32.0).collect();
        let span = [0.05, 0.2, 0.5, 1.0][case % 4];
        assert_eq!(
            bits(&before::loess_old(&pts, &grid, span)),
            bits(&loess_curve(&pts, &grid, span)),
            "case {case}"
        );
    }
}

#[test]
fn loess_falls_back_on_non_finite_or_unsorted_points() {
    let grid = [0.0, 1.0, 2.0];
    let mut pts = vec![
        (0.0, 0.1),
        (1.0, 0.2),
        (f64::INFINITY, 0.3),
        (2.0, 0.4),
        (3.0, 0.5),
    ];
    assert_eq!(
        bits(&before::loess_old(&pts, &grid, 0.6)),
        bits(&loess_curve(&pts, &grid, 0.6))
    );
    pts = vec![(2.0, 0.4), (0.0, 0.1), (1.0, 0.2), (3.0, 0.5)];
    assert_eq!(
        bits(&before::loess_old(&pts, &grid, 0.6)),
        bits(&loess_curve(&pts, &grid, 0.6))
    );
}

#[test]
fn ransac_refit_equals_the_weighted_one() {
    let mut rng = Rng(99);
    for case in 0..3000 {
        let n = [0, 2, 3, 5, 30, 200][case % 6];
        let pts = random_points(&mut rng, n, case % 2 == 0);
        let grid: Vec<f64> = (0..NODES).map(|j| 20.0 * j as f64 / 32.0).collect();
        let inlier = [0.01, 0.05, 0.2][case % 3];
        assert_eq!(
            bits(&before::ransac_old(&pts, &grid, inlier, case as u64)),
            bits(&ransac_curve(&pts, &grid, inlier, case as u64)),
            "case {case}"
        );
    }
}
