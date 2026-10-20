//! The contributions of the matched pairs to the library scores: each score (entropy similarity, cosine, modified cosine) is the sum of one contribution per
//! pair, so a score can be opened in its terms (`mzlab_core::score_terms`, step option `score_terms`).

use crate::clean::weighted;
use crate::scores::{pair_term, ModCosine, Spec};

/// The entropy similarity of `entropy_from_pairs` is the sum of one contribution per matched pair (`pair_term` of the entropy-weighted intensities):
/// `(m/z of the query peak, contribution)` for each pair.
pub fn entropy_contributions(a: &Spec, b: &Spec, pairs: &[(usize, usize)]) -> Vec<(f64, f64)> {
    let (pa, pb) = (weighted(&a.it), weighted(&b.it));
    pairs
        .iter()
        .map(|&(i, j)| (a.mz[i], pair_term(pa[i], pb[j])))
        .collect()
}

/// The cosine of `cosine_from_pairs` is the sum of `sqrt(a b) / sqrt(Σa Σb)` over its matched pairs.
pub fn cosine_contributions(a: &Spec, b: &Spec, pairs: &[(usize, usize)]) -> Vec<(f64, f64)> {
    let norm = (a.it.iter().sum::<f64>() * b.it.iter().sum::<f64>()).sqrt();
    pairs
        .iter()
        .map(|&(i, j)| (a.mz[i], (a.it[i] * b.it[j]).sqrt() / norm))
        .collect()
}

/// The modified cosine is the sum of the same contributions over its pairs (in the 32 strongest peaks of each spectrum, the shifted pairs included).
pub fn modified_cosine_contributions(m: &ModCosine) -> Vec<(f64, f64)> {
    let norm = (m.a_top.it.iter().sum::<f64>() * m.b_top.it.iter().sum::<f64>()).sqrt();
    m.pairs
        .iter()
        .map(|&(i, j, _)| (m.a_top.mz[i], (m.a_top.it[i] * m.b_top.it[j]).sqrt() / norm))
        .collect()
}
