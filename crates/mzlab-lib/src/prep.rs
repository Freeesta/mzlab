//! Query-side preparation of the pair scores: the entropy weights and sums of one query, computed once for the whole library scan
//! (`search::identity`); `scores::pair_scores` and `scores::entropy_from_pairs` are the one-shot forms of the same code.

use crate::clean::{entropy_of, weighted};
use crate::scores::{cosine_from_pairs, match_peaks, PairScores, Spec};

/// What the scores derive from the query alone.
pub struct QueryPrep {
    pa: Vec<f64>,
    ha: f64,
    sum: f64,
}

impl QueryPrep {
    pub fn new(a: &Spec) -> QueryPrep {
        let pa = weighted(&a.it);
        let ha = entropy_of(&pa);
        QueryPrep {
            pa,
            ha,
            sum: a.it.iter().sum(),
        }
    }
}

/// Entropy similarity for given pairs: `1 - (2 S_AB - S_A - S_B) / ln 4`, clipped to [0, 1].
pub fn entropy_from_pairs(a_it: &[f64], b_it: &[f64], pairs: &[(usize, usize)]) -> f64 {
    let pa = weighted(a_it);
    let ha = entropy_of(&pa);
    entropy_with_query(&pa, ha, b_it, pairs)
}

fn entropy_with_query(pa: &[f64], ha: f64, b_it: &[f64], pairs: &[(usize, usize)]) -> f64 {
    let pb = weighted(b_it);
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
    (1.0 - (2.0 * entropy_of(&m) - ha - entropy_of(&pb)) / 4f64.ln()).clamp(0.0, 1.0)
}

/// All the scores of the query `a` (with `prep = QueryPrep::new(a)`) against `b` on one set of pairs.
pub fn pair_scores_for(prep: &QueryPrep, a: &Spec, b: &Spec, tol: f64) -> PairScores {
    let pairs = match_peaks(&a.mz, &a.it, &b.mz, &b.it, tol);
    let entropy = entropy_with_query(&prep.pa, prep.ha, &b.it, &pairs);
    let cosine = cosine_from_pairs(&a.it, &b.it, &pairs);
    let dot: f64 = pairs.iter().map(|&(i, j)| (a.it[i] * b.it[j]).sqrt()).sum();
    let na = prep.sum;
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
