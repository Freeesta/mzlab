//! `peakq::reach_min` against walking out from each point.

use crate::peakq::reach_min;

/// Deterministic noise in [-1, 1).
fn lcg(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*seed >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
}

#[test]
fn reach_min_matches_the_walk() {
    let mut seed = 11u64;
    for n in [1usize, 2, 3, 50, 700] {
        let ys: Vec<f64> = (0..n)
            .map(|i| ((lcg(&mut seed) * 4.0).round() + (i % 40) as f64 * 0.1).max(0.0))
            .collect();
        let (l, r) = (reach_min(&ys, false), reach_min(&ys, true));
        for j in 0..n {
            let (mut lm, mut k) = (ys[j], j);
            while k > 0 && ys[k - 1] <= ys[j] {
                k -= 1;
                lm = lm.min(ys[k]);
            }
            let (mut rm, mut k) = (ys[j], j);
            while k + 1 < n && ys[k + 1] <= ys[j] {
                k += 1;
                rm = rm.min(ys[k]);
            }
            assert_eq!(
                (l[j].to_bits(), r[j].to_bits()),
                (lm.to_bits(), rm.to_bits()),
                "n={n} j={j}"
            );
        }
    }
}
