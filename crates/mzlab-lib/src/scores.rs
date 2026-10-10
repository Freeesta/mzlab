//! Spectrum-to-spectrum scores. Peaks are `(mz, intensity)` slices sorted by m/z; intensities are positive (any scale: every score is scale free).
//!
//! * `entropy` / `weighted_cosine` / `forward` / `reverse` / `explained` / `shared`: one-to-one pairs within an absolute tolerance, the pairs with the
//!   highest intensity product first (the rules of the former JS search, `libreria-worker.js`).
//! * `entropy_similarity` is the spectral entropy similarity of Li et al. (Nat. Methods 2021), the same quantity Flash entropy search computes;
//!   the brute-force reference of the golden data stays `mzlab-core::entropy`.
//! * `modified_cosine`: the former JS «coseno modificato» (32 strongest peaks, a peak may also pair at the precursor difference).
//! * `gnps_cosine`: the GNPS-style cosine with the rules of the precursor-aware pipeline of Attributio (rewritten from the rules): precursor region
//!   and peaks below 1% of the base peak removed, square-root intensities, greedy pairing by decreasing product within an absolute tolerance.

use crate::clean::{entropy_of, weighted};

/// A spectrum for scoring.
#[derive(Debug, Clone, Default)]
pub struct Spec {
    pub mz: Vec<f64>,
    pub it: Vec<f64>,
    pub prec: f64,
}

fn lower_bound(a: &[f64], x: f64) -> usize {
    a.partition_point(|v| *v < x)
}

/// One-to-one pairs `(i, j)` within `tol` (Da): the highest product `a_it[i] * b_it[j]` first; ties keep the order `i`, then `j`.
pub fn match_peaks(
    a_mz: &[f64],
    a_it: &[f64],
    b_mz: &[f64],
    b_it: &[f64],
    tol: f64,
) -> Vec<(usize, usize)> {
    let mut c: Vec<(usize, usize, f64)> = Vec::new();
    for i in 0..a_mz.len() {
        let mut j = lower_bound(b_mz, a_mz[i] - tol);
        while j < b_mz.len() && b_mz[j] <= a_mz[i] + tol {
            c.push((i, j, a_it[i] * b_it[j]));
            j += 1;
        }
    }
    c.sort_by(|x, y| y.2.total_cmp(&x.2));
    let (mut ua, mut ub) = (vec![false; a_mz.len()], vec![false; b_mz.len()]);
    let mut out = Vec::new();
    for (i, j, _) in c {
        if !ua[i] && !ub[j] {
            ua[i] = true;
            ub[j] = true;
            out.push((i, j));
        }
    }
    out
}

/// Entropy similarity for given pairs: `1 - (2 S_AB - S_A - S_B) / ln 4`, clipped to [0, 1].
pub fn entropy_from_pairs(a_it: &[f64], b_it: &[f64], pairs: &[(usize, usize)]) -> f64 {
    let (pa, pb) = (weighted(a_it), weighted(b_it));
    let (mut ia, mut ib) = (vec![false; pa.len()], vec![false; pb.len()]);
    let mut m: Vec<f64> = Vec::with_capacity(pa.len() + pb.len());
    for &(i, j) in pairs {
        m.push((pa[i] + pb[j]) / 2.0);
        ia[i] = true;
        ib[j] = true;
    }
    m.extend(
        pa.iter()
            .enumerate()
            .filter(|(i, _)| !ia[*i])
            .map(|(_, v)| v / 2.0),
    );
    m.extend(
        pb.iter()
            .enumerate()
            .filter(|(j, _)| !ib[*j])
            .map(|(_, v)| v / 2.0),
    );
    (1.0 - (2.0 * entropy_of(&m) - entropy_of(&pa) - entropy_of(&pb)) / 4f64.ln()).clamp(0.0, 1.0)
}

/// Contribution of one matched pair of entropy-weighted intensities `(a, b)` to the entropy similarity (the similarity is the sum over the pairs).
pub fn pair_term(a: f64, b: f64) -> f64 {
    if a <= 0.0 || b <= 0.0 {
        return 0.0;
    }
    (a + b) / 2.0
        - (a * a.ln() + b * b.ln() - (a + b) * ((a + b) / 2.0).ln())
            / (2.0 * std::f64::consts::LN_2)
}

/// Cosine of the square-root intensities over the given pairs: `Σ sqrt(a b) / sqrt(Σ a Σ b)`.
pub fn cosine_from_pairs(a_it: &[f64], b_it: &[f64], pairs: &[(usize, usize)]) -> f64 {
    let (na, nb): (f64, f64) = (a_it.iter().sum(), b_it.iter().sum());
    let dot: f64 = pairs.iter().map(|&(i, j)| (a_it[i] * b_it[j]).sqrt()).sum();
    if na > 0.0 && nb > 0.0 {
        dot / (na * nb).sqrt()
    } else {
        0.0
    }
}

/// All the scores of a query `a` against a library spectrum `b` on one set of pairs.
#[derive(Debug, Clone, PartialEq)]
pub struct PairScores {
    pub entropy: f64,
    pub cosine: f64,
    pub forward: f64,
    pub reverse: f64,
    pub shared: usize,
    pub explained: f64,
    pub pairs: Vec<(usize, usize)>,
}

pub fn pair_scores(a: &Spec, b: &Spec, tol: f64) -> PairScores {
    let pairs = match_peaks(&a.mz, &a.it, &b.mz, &b.it, tol);
    let entropy = entropy_from_pairs(&a.it, &b.it, &pairs);
    let cosine = cosine_from_pairs(&a.it, &b.it, &pairs);
    let dot: f64 = pairs.iter().map(|&(i, j)| (a.it[i] * b.it[j]).sqrt()).sum();
    let na: f64 = a.it.iter().sum();
    let nb: f64 = b.it.iter().sum();
    let na_m: f64 = pairs.iter().map(|&(i, _)| a.it[i]).sum();
    let nb_m: f64 = pairs.iter().map(|&(_, j)| b.it[j]).sum();
    let ratio = |d: f64, x: f64, y: f64| {
        if x > 0.0 && y > 0.0 {
            d / (x * y).sqrt()
        } else {
            0.0
        }
    };
    PairScores {
        entropy,
        cosine,
        // forward: all peaks of the query, only the matched ones of the library; reverse: the opposite (NIST naming)
        forward: ratio(dot, na, nb_m),
        reverse: ratio(dot, na_m, nb),
        shared: pairs.len(),
        explained: if na > 0.0 { na_m / na } else { 0.0 },
        pairs,
    }
}

/// Hybrid pairs (brute force, the reference of the Flash hybrid search): a peak of `a` pairs with a peak of `b` at the same m/z or shifted by
/// `delta = prec(a) - prec(b)`, within `tol`; one-to-one, the pairs with the highest contribution to the entropy similarity first.
pub fn hybrid_pairs(a: &Spec, b: &Spec, tol: f64) -> Vec<(usize, usize)> {
    let (pa, pb) = (weighted(&a.it), weighted(&b.it));
    let delta = a.prec - b.prec;
    let mut c: Vec<(usize, usize, f64)> = Vec::new();
    for i in 0..a.mz.len() {
        for j in 0..b.mz.len() {
            let d = a.mz[i] - b.mz[j];
            if d.abs() <= tol || (d - delta).abs() <= tol {
                c.push((i, j, pair_term(pa[i], pb[j])));
            }
        }
    }
    c.sort_by(|x, y| y.2.total_cmp(&x.2));
    let (mut ua, mut ub) = (vec![false; a.mz.len()], vec![false; b.mz.len()]);
    let mut out = Vec::new();
    for (i, j, _) in c {
        if !ua[i] && !ub[j] {
            ua[i] = true;
            ub[j] = true;
            out.push((i, j));
        }
    }
    out
}

/// Neutral losses (precursor − m/z) in common, one-to-one within `tol` (the pairs with the highest intensity product first).
pub fn common_losses(a: &Spec, b: &Spec, tol: f64) -> usize {
    if a.prec <= 0.0 || b.prec <= 0.0 {
        return 0;
    }
    let la: Vec<f64> = a.mz.iter().rev().map(|m| a.prec - m).collect();
    let lb: Vec<f64> = b.mz.iter().rev().map(|m| b.prec - m).collect();
    let ia: Vec<f64> = a.it.iter().rev().copied().collect();
    let ib: Vec<f64> = b.it.iter().rev().copied().collect();
    match_peaks(&la, &ia, &lb, &ib, tol).len()
}

// ---------------------------------------------------------------- modified cosine (the analogue score of the library window)

/// Peaks used for the modified cosine: the most intense.
pub const AN_TOP: usize = 32;

/// The precursor region (within 1.5 of it) and the noise (< 1% of the highest) are left out, then the 32 most intense, sorted by m/z.
pub fn top32(s: &Spec) -> Spec {
    let mx = s.it.iter().cloned().fold(0.0, f64::max);
    let mut idx: Vec<usize> = (0..s.mz.len())
        .filter(|&i| s.mz[i] < s.prec - 1.5 && s.it[i] >= 0.01 * mx && s.it[i] > 0.0)
        .collect();
    if idx.len() > AN_TOP {
        idx.sort_by(|&a, &b| s.it[b].total_cmp(&s.it[a]));
        idx.truncate(AN_TOP);
    }
    idx.sort_unstable();
    Spec {
        mz: idx.iter().map(|&i| s.mz[i]).collect(),
        it: idx.iter().map(|&i| s.it[i]).collect(),
        prec: s.prec,
    }
}

#[derive(Debug, Clone)]
pub struct ModCosine {
    pub cosine: f64,
    pub matched: usize,
    pub shifted: usize,
    pub delta: f64,
    /// `(index in a, index in b, pair needed the shift)`, indices into the top-32 spectra `a_top` and `b_top`.
    pub pairs: Vec<(usize, usize, bool)>,
    pub a_top: Spec,
    pub b_top: Spec,
}

/// Modified cosine: candidate pairs at shift 0 and at `delta = prec(q) - prec(lib)`, one-to-one greedy by decreasing `sqrt(Iq Il)`.
pub fn modified_cosine(q: &Spec, lib: &Spec, tol: f64) -> ModCosine {
    let a = top32(q);
    let b = top32(lib);
    let delta = q.prec - lib.prec;
    let shifts: &[f64] = if delta.abs() < 1e-6 {
        &[0.0]
    } else {
        &[0.0, delta]
    };
    let mut c: Vec<(usize, usize, f64, bool)> = Vec::new();
    for i in 0..a.mz.len() {
        for &sh in shifts {
            let t = a.mz[i] - sh;
            let mut j = lower_bound(&b.mz, t - tol);
            while j < b.mz.len() && b.mz[j] <= t + tol {
                c.push((i, j, (a.it[i] * b.it[j]).sqrt(), sh != 0.0));
                j += 1;
            }
        }
    }
    c.sort_by(|x, y| y.2.total_cmp(&x.2));
    let (mut ua, mut ub) = (vec![false; a.mz.len()], vec![false; b.mz.len()]);
    let (mut dot, mut shifted) = (0.0, 0usize);
    let mut pairs = Vec::new();
    for (i, j, w, sh) in c {
        if !ua[i] && !ub[j] {
            ua[i] = true;
            ub[j] = true;
            dot += w;
            pairs.push((i, j, sh));
            if sh {
                shifted += 1;
            }
        }
    }
    let (na, nb): (f64, f64) = (a.it.iter().sum(), b.it.iter().sum());
    let cosine = if na > 0.0 && nb > 0.0 {
        dot / (na * nb).sqrt()
    } else {
        0.0
    };
    ModCosine {
        cosine,
        matched: pairs.len(),
        shifted,
        delta,
        pairs,
        a_top: a,
        b_top: b,
    }
}

// ---------------------------------------------------------------- GNPS-style cosine

#[derive(Debug, Clone, PartialEq)]
pub struct GnpsMatch {
    pub cosine: f64,
    pub reverse: f64,
    pub matched: usize,
    pub n_shifted: usize,
    /// `(index in the cleaned sample, index in the cleaned reference, shifted)`.
    pub pairs: Vec<(usize, usize, bool)>,
}

/// Peaks at or above `precursor - 0.5` and below 1% of the base peak are removed; intensities become square roots.
pub fn gnps_prepare(mz: &[f64], it: &[f64], prec: f64) -> (Vec<f64>, Vec<f64>) {
    let mx = it.iter().cloned().fold(0.0, f64::max);
    let (mut m, mut v) = (Vec::new(), Vec::new());
    for (x, y) in mz.iter().zip(it) {
        if *y > 0.0 && *x < prec - 0.5 && *y >= 0.01 * mx {
            m.push(*x);
            v.push(y.sqrt());
        }
    }
    (m, v)
}

/// GNPS cosine of a sample against a reference with an absolute tolerance in Da. `modified`: a reference peak at `m` may also pair with a sample
/// peak at `m + delta`, `delta = prec(sample) - prec(reference)`. Ties in the product: the unshifted pair first, then the higher indices.
pub fn gnps_cosine(sample: &Spec, reference: &Spec, tol: f64, modified: bool) -> GnpsMatch {
    let (am, ai) = gnps_prepare(&sample.mz, &sample.it, sample.prec);
    let (bm, bi) = gnps_prepare(&reference.mz, &reference.it, reference.prec);
    let delta = sample.prec - reference.prec;
    let mut c: Vec<(usize, usize, f64, bool)> = Vec::new();
    for i in 0..am.len() {
        for j in 0..bm.len() {
            if (am[i] - bm[j]).abs() <= tol {
                c.push((i, j, ai[i] * bi[j], false));
            } else if modified && delta.abs() > tol && (am[i] - (bm[j] + delta)).abs() <= tol {
                c.push((i, j, ai[i] * bi[j], true));
            }
        }
    }
    c.sort_by(|x, y| {
        y.2.total_cmp(&x.2)
            .then(x.3.cmp(&y.3))
            .then(y.0.cmp(&x.0))
            .then(y.1.cmp(&x.1))
    });
    let (mut ua, mut ub) = (vec![false; am.len()], vec![false; bm.len()]);
    let (mut dot, mut na_m, mut n_shifted) = (0.0, 0.0, 0usize);
    let mut pairs = Vec::new();
    for (i, j, w, sh) in c {
        if !ua[i] && !ub[j] {
            ua[i] = true;
            ub[j] = true;
            dot += w;
            na_m += ai[i] * ai[i];
            pairs.push((i, j, sh));
            if sh {
                n_shifted += 1;
            }
        }
    }
    let na: f64 = ai.iter().map(|v| v * v).sum::<f64>().sqrt();
    let nb: f64 = bi.iter().map(|v| v * v).sum::<f64>().sqrt();
    let ratio = |d: f64, x: f64, y: f64| if x > 0.0 && y > 0.0 { d / (x * y) } else { 0.0 };
    GnpsMatch {
        cosine: ratio(dot, na, nb),
        reverse: ratio(dot, na_m.sqrt(), nb),
        matched: pairs.len(),
        n_shifted,
        pairs,
    }
}

#[cfg(test)]
mod tests {
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
}
