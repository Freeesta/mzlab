//! Unit tests of the scores.

use super::*;

fn sp(mz: &[f64], it: &[f64], prec: f64) -> Spec {
    Spec {
        mz: mz.to_vec(),
        it: it.to_vec(),
        prec,
    }
}

#[test]
fn identical_spectra_score_one() {
    let s = sp(
        &[100.0, 150.0, 200.0, 250.0],
        &[1000.0, 500.0, 250.0, 100.0],
        400.0,
    );
    let r = pair_scores(&s, &s, 0.01);
    assert!((r.cosine - 1.0).abs() < 1e-12 && (r.entropy - 1.0).abs() < 1e-12 && r.shared == 4);
    assert!(
        (r.forward - 1.0).abs() < 1e-12
            && (r.reverse - 1.0).abs() < 1e-12
            && (r.explained - 1.0).abs() < 1e-12
    );
}

#[test]
fn no_common_peaks_score_zero() {
    let a = sp(&[100.0, 150.0, 200.0], &[10.0, 5.0, 2.0], 0.0);
    let b = sp(&[101.0, 151.5, 300.0], &[10.0, 5.0, 2.0], 0.0);
    let r = pair_scores(&a, &b, 0.01);
    assert!(r.cosine == 0.0 && r.entropy.abs() < 1e-12 && r.shared == 0 && r.explained == 0.0);
}

#[test]
fn tolerance_decides_the_pairs() {
    let a = sp(&[100.0, 150.0, 200.0], &[10.0, 8.0, 6.0], 0.0);
    let b = sp(&[100.004, 150.5, 200.008], &[10.0, 8.0, 6.0], 0.0);
    let (tight, loose) = (pair_scores(&a, &b, 0.01), pair_scores(&a, &b, 1.0));
    assert!(
        tight.shared == 2
            && loose.shared == 3
            && 0.0 < tight.cosine
            && tight.cosine < loose.cosine
            && loose.cosine <= 1.0
            && 0.0 < tight.entropy
            && tight.entropy < 1.0
    );
}

#[test]
fn entropy_matches_a_hand_computation_for_two_peaks() {
    // A = two equal peaks, B = one of them: S_A = ln 2 < 3 -> weight 0.25 + 0.25 ln 2 (both stay 0.5); B is a single peak (p = 1)
    let a = sp(&[100.0, 200.0], &[1.0, 1.0], 0.0);
    let b = sp(&[100.0], &[1.0], 0.0);
    let r = pair_scores(&a, &b, 0.01);
    let h = |p: &[f64]| {
        -p.iter()
            .filter(|v| **v > 0.0)
            .map(|v| v * v.ln())
            .sum::<f64>()
    };
    let m = [(0.5 + 1.0) / 2.0, 0.5 / 2.0];
    let expected = 1.0 - (2.0 * h(&m) - h(&[0.5, 0.5]) - 0.0) / 4f64.ln();
    assert!(
        (r.entropy - expected).abs() < 1e-12,
        "{} vs {expected}",
        r.entropy
    );
}

#[test]
fn the_pair_term_sums_to_the_similarity() {
    let a = sp(
        &[100.0, 150.0, 200.0, 250.0],
        &[1000.0, 500.0, 250.0, 100.0],
        0.0,
    );
    let b = sp(
        &[100.0, 150.0, 220.0, 250.0],
        &[800.0, 100.0, 600.0, 300.0],
        0.0,
    );
    let r = pair_scores(&a, &b, 0.01);
    let (pa, pb) = (weighted(&a.it), weighted(&b.it));
    let sum: f64 = r.pairs.iter().map(|&(i, j)| pair_term(pa[i], pb[j])).sum();
    assert!((sum - r.entropy).abs() < 1e-12, "{sum} vs {}", r.entropy);
}

#[test]
fn modified_cosine_pairs_at_shift_zero_and_at_the_precursor_difference() {
    let lib = sp(
        &[60.5, 100.1, 150.2, 200.3, 250.4],
        &[100.0, 400.0, 900.0, 300.0, 200.0],
        300.1,
    );
    let q = sp(
        &[60.5, 100.1, 166.2, 216.3, 266.4],
        &[100.0, 400.0, 900.0, 300.0, 200.0],
        316.1,
    );
    let r = modified_cosine(&q, &lib, 0.01);
    let plain = pair_scores(&q, &lib, 0.01);
    assert!(
        r.matched == 5
            && r.shifted == 3
            && r.cosine > 0.99
            && plain.cosine < 0.5
            && (r.delta - 16.0).abs() < 1e-9
    );
}

#[test]
fn gnps_rules() {
    let a = sp(
        &[100.0, 120.0, 150.0, 200.0, 299.9],
        &[100.0, 30.0, 50.0, 0.5, 100.0],
        300.0,
    );
    let b = sp(
        &[100.01, 150.0, 200.0, 299.9],
        &[100.0, 50.0, 80.0, 100.0],
        300.0,
    );
    let r = gnps_cosine(&a, &b, 0.02, false);
    // a: the peak at 299.9 is in the precursor region and the one at 200 is below 1%; the one at 120 has no partner
    assert_eq!(r.matched, 2);
    assert!(
        r.reverse > r.cosine,
        "reverse ignores the unmatched peaks of the sample"
    );
}
