//! Entropy similarity and the full score set of a query against one library spectrum on a set of pairs (`scores::pair_scores`).
//! `Prepared` holds the entropy weights of a query so a search over many candidates computes them once (`search::identity`).

use crate::clean::{entropy_of, weighted};
use crate::scores::{cosine_from_pairs, match_peaks, Spec};

/// Entropy weights of one spectrum and their entropy, the query side of the entropy similarity.
pub struct Prepared {
    pw: Vec<f64>,
    h: f64,
}

impl Prepared {
    pub fn new(it: &[f64]) -> Prepared {
        let pw = weighted(it);
        let h = entropy_of(&pw);
        Prepared { pw, h }
    }
}

fn entropy_prepared(a: &Prepared, b_it: &[f64], pairs: &[(usize, usize)]) -> f64 {
    let (pa, pb) = (&a.pw, weighted(b_it));
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
    (1.0 - (2.0 * entropy_of(&m) - a.h - entropy_of(&pb)) / 4f64.ln()).clamp(0.0, 1.0)
}

/// Entropy similarity for given pairs: `1 - (2 S_AB - S_A - S_B) / ln 4`, clipped to [0, 1].
pub fn entropy_from_pairs(a_it: &[f64], b_it: &[f64], pairs: &[(usize, usize)]) -> f64 {
    entropy_prepared(&Prepared::new(a_it), b_it, pairs)
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
    pair_scores_prepared(&Prepared::new(&a.it), a, b, tol)
}

/// `pair_scores` with the entropy weights of `a` computed once (`prep` = `Prepared::new(&a.it)`); the same bits.
pub fn pair_scores_prepared(prep: &Prepared, a: &Spec, b: &Spec, tol: f64) -> PairScores {
    let pairs = match_peaks(&a.mz, &a.it, &b.mz, &b.it, tol);
    let entropy = entropy_prepared(prep, &b.it, &pairs);
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
