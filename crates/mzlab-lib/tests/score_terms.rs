//! The contributions of the matched pairs add up to the library scores (entropy similarity, cosine, modified cosine) to 1e-12, on every case of a small family
//! of synthetic spectra (shared peaks, extra peaks, a precursor shift).

use mzlab_lib::contrib::{
    cosine_contributions, entropy_contributions, modified_cosine_contributions,
};
use mzlab_lib::scores::{match_peaks, modified_cosine, pair_scores, Spec};

/// A deterministic pseudo-random spectrum of `n` peaks above `base` m/z.
fn spec(seed: u64, n: usize, base: f64, prec: f64) -> Spec {
    let mut x = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let mut next = || {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (x >> 33) as f64 / (1u64 << 31) as f64
    };
    let mut mz: Vec<f64> = (0..n).map(|_| base + 400.0 * next()).collect();
    mz.sort_by(|a, b| a.total_cmp(b));
    mz.dedup_by(|a, b| (*a - *b).abs() < 0.05);
    let it = mz.iter().map(|_| 1.0 + 999.0 * next()).collect();
    Spec { mz, it, prec }
}

/// `b` shares part of the peaks of `a` (shifted by `shift` for the shared ones) and has peaks of its own.
fn related(a: &Spec, seed: u64, shift: f64) -> Spec {
    let own = spec(seed, 12, 60.0, a.prec - shift);
    let mut mz: Vec<f64> = a.mz.iter().step_by(2).map(|m| m - shift + 0.003).collect();
    let mut it: Vec<f64> = a.it.iter().step_by(2).map(|v| v * 0.7).collect();
    mz.extend(&own.mz);
    it.extend(&own.it);
    let mut ix: Vec<usize> = (0..mz.len()).collect();
    ix.sort_by(|&p, &q| mz[p].total_cmp(&mz[q]));
    Spec {
        mz: ix.iter().map(|&k| mz[k]).collect(),
        it: ix.iter().map(|&k| it[k]).collect(),
        prec: a.prec - shift,
    }
}

fn sum(v: &[(f64, f64)]) -> f64 {
    v.iter().map(|p| p.1).sum()
}

#[test]
fn the_pair_contributions_add_up_to_the_scores() {
    for seed in 1..=40u64 {
        let a = spec(seed, 30, 60.0, 450.0);
        let b = related(&a, seed + 100, if seed % 2 == 0 { 0.0 } else { 14.0157 });
        let pairs = match_peaks(&a.mz, &a.it, &b.mz, &b.it, 0.02);
        let s = pair_scores(&a, &b, 0.02);
        assert!(
            (sum(&entropy_contributions(&a, &b, &pairs)) - s.entropy).abs() < 1e-12,
            "entropy, seed {seed}"
        );
        assert!(
            (sum(&cosine_contributions(&a, &b, &pairs)) - s.cosine).abs() < 1e-12,
            "cosine, seed {seed}"
        );
        let m = modified_cosine(&a, &b, 0.02);
        assert!(
            (sum(&modified_cosine_contributions(&m)) - m.cosine).abs() < 1e-12,
            "modified cosine, seed {seed}"
        );
    }
}
