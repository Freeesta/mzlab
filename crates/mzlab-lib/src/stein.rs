//! Scores for GC-EI and the hybrid pair score: the Stein weighted cosine (no precursor needed) with the Gaussian retention-index factor, and the hybrid
//! entropy similarity of a pair (direct plus precursor-shifted peaks, the quantity the Flash hybrid search computes). Re-exported by `scores`.

use crate::frag_tol::FragTol;
use crate::scores::{entropy_from_pairs, hybrid_pairs, match_peaks, Spec};

/// Weights of the composite cosine: peak `i` counts `I^a * (m/z)^b`. The defaults are the NIST-style values reported for Kim, Koo, Wei & Zhang (2012,
/// Bioinformatics 28:1158, optimal weight factors for GC-MS identification); Stein & Scott (1994) used 0.6 and 3 on the same form. Both are parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SteinOpts {
    pub a: f64,
    pub b: f64,
    /// Width of the Gaussian retention-index factor, in RI units.
    pub ri_sigma: f64,
}

impl Default for SteinOpts {
    fn default() -> Self {
        SteinOpts {
            a: 0.53,
            b: 1.3,
            ri_sigma: 10.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SteinScore {
    /// Composite cosine times the retention-index factor (1 when either spectrum has no RI).
    pub score: f64,
    pub forward: f64,
    pub reverse: f64,
    pub ri_factor: f64,
    pub pairs: Vec<(usize, usize)>,
}

/// `exp(-(dri / sigma)^2 / 2)`.
pub fn ri_factor(dri: f64, sigma: f64) -> f64 {
    if sigma <= 0.0 {
        return 1.0;
    }
    (-0.5 * (dri / sigma).powi(2)).exp()
}

/// Stein weighted cosine of `q` against `l`: one-to-one pairs within `tol` (highest weighted product first), `Σ wq wl / sqrt(Σ wq² Σ wl²)`.
/// `forward` uses all query peaks and the matched library ones, `reverse` the opposite. The precursors are not read.
pub fn stein_cosine(
    q: &Spec,
    l: &Spec,
    tol: impl Into<FragTol>,
    o: &SteinOpts,
    q_ri: Option<f64>,
    l_ri: Option<f64>,
) -> SteinScore {
    let w = |s: &Spec| -> Vec<f64> {
        s.mz.iter()
            .zip(&s.it)
            .map(|(m, i)| i.max(0.0).powf(o.a) * m.powf(o.b))
            .collect()
    };
    let (wq, wl) = (w(q), w(l));
    let pairs = match_peaks(&q.mz, &wq, &l.mz, &wl, tol);
    let sq = |v: &[f64]| v.iter().map(|x| x * x).sum::<f64>();
    let dot: f64 = pairs.iter().map(|&(i, j)| wq[i] * wl[j]).sum();
    let (nq, nl) = (sq(&wq), sq(&wl));
    let nq_m: f64 = pairs.iter().map(|&(i, _)| wq[i] * wq[i]).sum();
    let nl_m: f64 = pairs.iter().map(|&(_, j)| wl[j] * wl[j]).sum();
    let ratio = |x: f64, y: f64| {
        if x > 0.0 && y > 0.0 {
            dot / (x * y).sqrt()
        } else {
            0.0
        }
    };
    let f = match (q_ri, l_ri) {
        (Some(a), Some(b)) => ri_factor(a - b, o.ri_sigma),
        _ => 1.0,
    };
    SteinScore {
        score: ratio(nq, nl) * f,
        forward: ratio(nq, nl_m),
        reverse: ratio(nq_m, nl),
        ri_factor: f,
        pairs,
    }
}

/// Hybrid entropy similarity of a pair and its pairs (indices into `a` and `b`): peaks pair unshifted or shifted by `a.prec - b.prec` within `tol` Da.
/// Same pairs as `hybrid_pairs`, the reference of the Flash hybrid search.
pub fn hybrid_entropy(a: &Spec, b: &Spec, tol: f64) -> (f64, Vec<(usize, usize)>) {
    let pairs = hybrid_pairs(a, b, tol);
    (entropy_from_pairs(&a.it, &b.it, &pairs), pairs)
}
