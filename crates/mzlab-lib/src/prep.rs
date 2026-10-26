//! Query-side preparation and reusable buffers for the pair scores: `Scorer` holds what one query derives alone (entropy weights, sum) and the
//! scratch vectors, so a library scan scores every spectrum without allocating; `scores::pair_scores` and `scores::entropy_from_pairs` are the
//! one-shot forms of the same code (same arithmetic, same order: the numbers are identical bit for bit).

use crate::clean::{entropy_of, weighted, weighted_into};
use crate::frag_tol::FragTol;
use crate::scores::{cosine_from_pairs, match_peaks_into, MatchScratch, PairScores, Spec};

/// The buffers of the entropy of the pairs.
#[derive(Default)]
struct EntropyScratch {
    pb: Vec<f64>,
    ia: Vec<bool>,
    ib: Vec<bool>,
    m: Vec<f64>,
}

/// Entropy similarity for given pairs: `1 - (2 S_AB - S_A - S_B) / ln 4`, clipped to [0, 1].
pub fn entropy_from_pairs(a_it: &[f64], b_it: &[f64], pairs: &[(usize, usize)]) -> f64 {
    let pa = weighted(a_it);
    let ha = entropy_of(&pa);
    entropy_with_query(&pa, ha, b_it, pairs, &mut EntropyScratch::default())
}

fn entropy_with_query(
    pa: &[f64],
    ha: f64,
    b_it: &[f64],
    pairs: &[(usize, usize)],
    s: &mut EntropyScratch,
) -> f64 {
    weighted_into(b_it, &mut s.pb);
    let pb = &s.pb;
    s.ia.clear();
    s.ia.resize(pa.len(), false);
    s.ib.clear();
    s.ib.resize(pb.len(), false);
    s.m.clear();
    for &(i, j) in pairs {
        s.m.push((pa[i] + pb[j]) / 2.0);
        s.ia[i] = true;
        s.ib[j] = true;
    }
    s.m.extend(
        pa.iter()
            .enumerate()
            .filter(|(i, _)| !s.ia[*i])
            .map(|(_, v)| v / 2.0),
    );
    s.m.extend(
        pb.iter()
            .enumerate()
            .filter(|(j, _)| !s.ib[*j])
            .map(|(_, v)| v / 2.0),
    );
    (1.0 - (2.0 * entropy_of(&s.m) - ha - entropy_of(pb)) / 4f64.ln()).clamp(0.0, 1.0)
}

/// Scores one query `a` against many library spectra.
pub struct Scorer {
    pa: Vec<f64>,
    ha: f64,
    sum: f64,
    m: MatchScratch,
    e: EntropyScratch,
}

impl Scorer {
    pub fn new(a: &Spec) -> Scorer {
        let pa = weighted(&a.it);
        let ha = entropy_of(&pa);
        Scorer {
            pa,
            ha,
            sum: a.it.iter().sum(),
            m: MatchScratch::default(),
            e: EntropyScratch::default(),
        }
    }

    /// The pairs of the last `score` call.
    pub fn pairs(&self) -> &[(usize, usize)] {
        &self.m.out
    }

    /// All the scores of `a` (the spectrum this scorer was made from) against `b`; `pairs` of the result is empty, read them with `pairs()`.
    pub fn score(&mut self, a: &Spec, b: &Spec, tol: impl Into<FragTol>) -> PairScores {
        match_peaks_into(&a.mz, &a.it, &b.mz, &b.it, tol, &mut self.m);
        let pairs = &self.m.out;
        let entropy = entropy_with_query(&self.pa, self.ha, &b.it, pairs, &mut self.e);
        let cosine = cosine_from_pairs(&a.it, &b.it, pairs);
        let dot: f64 = pairs.iter().map(|&(i, j)| (a.it[i] * b.it[j]).sqrt()).sum();
        let na = self.sum;
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
            pairs: Vec::new(),
        }
    }
}
