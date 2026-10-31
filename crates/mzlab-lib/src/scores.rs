//! Spectrum-to-spectrum scores. Peaks are `(mz, intensity)` slices sorted by m/z; intensities are positive (any scale: every score is scale free).
//!
//! * `entropy` / `weighted_cosine` / `forward` / `reverse` / `explained` / `shared`: one-to-one pairs within an absolute tolerance, the pairs with the
//!   highest intensity product first (the rules of the former JS search, `libreria-worker.js`).
//! * `entropy_similarity` is the spectral entropy similarity of Li et al. (Nat. Methods 2021), the same quantity Flash entropy search computes;
//!   the brute-force reference of the golden data stays `mzlab-core::entropy`.
//! * `modified_cosine`: the former JS «coseno modificato» (32 strongest peaks, a peak may also pair at the precursor difference).
//! * `gnps_cosine`: the GNPS-style cosine with the rules of the precursor-aware pipeline of Attributio (rewritten from the rules): precursor region
//!   and peaks below 1% of the base peak removed, square-root intensities, greedy pairing by decreasing product within an absolute tolerance.

use crate::clean::weighted;
use crate::frag_tol::FragTol;
pub use crate::stein::{hybrid_entropy, ri_factor, stein_cosine, SteinOpts, SteinScore};
use mzlab_picchi::sortc;

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
    tol: impl Into<FragTol>,
) -> Vec<(usize, usize)> {
    let mut s = MatchScratch::default();
    match_peaks_into(a_mz, a_it, b_mz, b_it, tol, &mut s);
    s.out
}

/// The buffers of `match_peaks_into`, reused from one library spectrum to the next.
#[derive(Default)]
pub struct MatchScratch {
    cand: Vec<(usize, usize, f64)>,
    ua: Vec<bool>,
    ub: Vec<bool>,
    /// The pairs of the last call.
    pub out: Vec<(usize, usize)>,
}

/// `match_peaks` with the result in `s.out` and no allocation once the buffers have grown.
pub fn match_peaks_into(
    a_mz: &[f64],
    a_it: &[f64],
    b_mz: &[f64],
    b_it: &[f64],
    tol: impl Into<FragTol>,
    s: &mut MatchScratch,
) {
    let tol = tol.into();
    s.cand.clear();
    for i in 0..a_mz.len() {
        let w = tol.at(a_mz[i]);
        let mut j = lower_bound(b_mz, a_mz[i] - w);
        while j < b_mz.len() && b_mz[j] <= a_mz[i] + w {
            s.cand.push((i, j, a_it[i] * b_it[j]));
            j += 1;
        }
    }
    sortc::by(&mut s.cand, |x, y| y.2.total_cmp(&x.2));
    s.ua.clear();
    s.ua.resize(a_mz.len(), false);
    s.ub.clear();
    s.ub.resize(b_mz.len(), false);
    s.out.clear();
    for &(i, j, _) in &s.cand {
        if !s.ua[i] && !s.ub[j] {
            s.ua[i] = true;
            s.ub[j] = true;
            s.out.push((i, j));
        }
    }
}

/// Entropy similarity for given pairs: `1 - (2 S_AB - S_A - S_B) / ln 4`, clipped to [0, 1].
pub fn entropy_from_pairs(a_it: &[f64], b_it: &[f64], pairs: &[(usize, usize)]) -> f64 {
    crate::prep::entropy_from_pairs(a_it, b_it, pairs)
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

pub fn pair_scores(a: &Spec, b: &Spec, tol: impl Into<FragTol>) -> PairScores {
    let mut sc = crate::prep::Scorer::new(a);
    let mut r = sc.score(a, b, tol);
    r.pairs = sc.pairs().to_vec();
    r
}

/// True when the values are in ascending order (no NaN): the window searches below need it, and the callers' spectra have it.
fn ascending(v: &[f64]) -> bool {
    v.windows(2).all(|w| w[0] <= w[1])
}

/// Indexes `j` of the ascending `b` with `|(a - b[j]) - shift| <= tol` as a range. `a - b[j]` and the subtractions after it only ever round the same
/// way for larger `b[j]`, so the values that satisfy the test are contiguous and the two bounds are binary searches.
fn window_of(b: &[f64], a: f64, shift: f64, tol: f64) -> std::ops::Range<usize> {
    let lo = b.partition_point(|&m| (a - m) - shift > tol);
    let hi = b.partition_point(|&m| (a - m) - shift >= -tol);
    if lo < hi {
        lo..hi
    } else {
        0..0
    }
}

/// Hybrid pairs (the reference of the Flash hybrid search): a peak of `a` pairs with a peak of `b` at the same m/z or shifted by
/// `delta = prec(a) - prec(b)`, within `tol`; one-to-one, the pairs with the highest contribution to the entropy similarity first.
pub fn hybrid_pairs(a: &Spec, b: &Spec, tol: f64) -> Vec<(usize, usize)> {
    let (pa, pb) = (weighted(&a.it), weighted(&b.it));
    let delta = a.prec - b.prec;
    let mut c: Vec<(usize, usize, f64)> = Vec::new();
    if ascending(&b.mz) {
        for (i, (&ma, &wa)) in a.mz.iter().zip(&pa).enumerate() {
            let (w1, w2) = (
                window_of(&b.mz, ma, 0.0, tol),
                window_of(&b.mz, ma, delta, tol),
            );
            // the candidates of both windows in the order of `j`, each once
            let (mut j1, mut j2) = (w1.start, w2.start);
            while j1 < w1.end || j2 < w2.end {
                let j = if j1 < w1.end && (j2 >= w2.end || j1 <= j2) {
                    j1
                } else {
                    j2
                };
                if j1 == j {
                    j1 += 1;
                }
                if j2 == j {
                    j2 += 1;
                }
                c.push((i, j, pair_term(wa, pb[j])));
            }
        }
    } else {
        for (i, (&ma, &wa)) in a.mz.iter().zip(&pa).enumerate() {
            for (j, (&mb, &wb)) in b.mz.iter().zip(&pb).enumerate() {
                let d = ma - mb;
                if d.abs() <= tol || (d - delta).abs() <= tol {
                    c.push((i, j, pair_term(wa, wb)));
                }
            }
        }
    }
    sortc::by(&mut c, |x, y| y.2.total_cmp(&x.2));
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
    common_losses_with(a, b, tol, &mut MatchScratch::default())
}

/// `common_losses` with the buffers of the caller. The losses are the precursor minus each m/z, taken from the last peak to the first (so they
/// ascend); they are computed where they are used, with the same operations as the vectors they replace, so the count is the same.
pub fn common_losses_with(a: &Spec, b: &Spec, tol: f64, s: &mut MatchScratch) -> usize {
    if a.prec <= 0.0 || b.prec <= 0.0 {
        return 0;
    }
    let (na, nb) = (a.mz.len(), b.mz.len());
    let la = |k: usize| a.prec - a.mz[na - 1 - k];
    let lb = |k: usize| b.prec - b.mz[nb - 1 - k];
    s.cand.clear();
    for i in 0..na {
        let x = la(i);
        let (mut lo, mut hi) = (0, nb);
        while lo < hi {
            let mid = (lo + hi) / 2;
            if lb(mid) < x - tol {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let mut j = lo;
        while j < nb && lb(j) <= x + tol {
            s.cand.push((i, j, a.it[na - 1 - i] * b.it[nb - 1 - j]));
            j += 1;
        }
    }
    sortc::by(&mut s.cand, |x, y| y.2.total_cmp(&x.2));
    s.ua.clear();
    s.ua.resize(na, false);
    s.ub.clear();
    s.ub.resize(nb, false);
    let mut n = 0;
    for &(i, j, _) in &s.cand {
        if !s.ua[i] && !s.ub[j] {
            s.ua[i] = true;
            s.ub[j] = true;
            n += 1;
        }
    }
    n
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
        sortc::by(&mut idx, |&a, &b| s.it[b].total_cmp(&s.it[a]));
        idx.truncate(AN_TOP);
    }
    sortc::by(&mut idx, |a, b| a.cmp(b));
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
pub fn modified_cosine(q: &Spec, lib: &Spec, tol: impl Into<FragTol>) -> ModCosine {
    let tol = tol.into();
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
            let w = tol.at(a.mz[i]);
            let mut j = lower_bound(&b.mz, t - w);
            while j < b.mz.len() && b.mz[j] <= t + w {
                c.push((i, j, (a.it[i] * b.it[j]).sqrt(), sh != 0.0));
                j += 1;
            }
        }
    }
    sortc::by(&mut c, |x, y| y.2.total_cmp(&x.2));
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

mod gnps;
pub use gnps::{gnps_cosine, gnps_match_prepared, gnps_prepare, GnpsMatch};

#[cfg(test)]
mod tests;
