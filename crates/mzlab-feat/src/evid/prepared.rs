//! What a graph of proofs computes once for all its rows (`Prepared`) and the key that orders the identities, kept apart from the identities themselves
//! so that `out::GraphStream` can make an identity twice instead of holding them all.

use super::graph::*;
use super::*;
use mzlab_core::roles::PROTON;

/// What every identity of a graph shares: the hypothesis, library hit and N of each row, the parent row and the neutral masses.
pub struct Prepared {
    hyp: Vec<Option<usize>>,
    best: Vec<Option<usize>>,
    ns: Vec<usize>,
    parent: Option<usize>,
    neutral: Vec<f64>,
}

impl Prepared {
    pub fn new(inp: &Inputs) -> Prepared {
        let n = inp.aligned.len();
        let hyp = match_hypotheses_in(&inp.registry.hyps, inp.aligned, inp.table.window(HYPOTHESIS_PPM));
        let best = best_hits(inp);
        let sign = f64::from(inp.registry.polarity.signum());
        let neutral: Vec<f64> = (0..n)
            .map(|r| {
                hyp[r].map_or(inp.aligned.mz[r] - sign * PROTON, |h| {
                    inp.registry.hyps[h].neutral
                })
            })
            .collect();
        let cands: Vec<Candidate> = (0..n)
            .map(|r| Candidate {
                mass: neutral[r],
                ms2: best[r].map(|b| inp.libs[b].entropy),
            })
            .collect();
        Prepared {
            ns: counts_of_n(inp.table, &cands),
            parent: parent_row(inp),
            hyp,
            best,
            neutral,
        }
    }

    /// The identity of row `r` (a pure function of the row: it can be made again whenever it is needed).
    #[inline(never)]
    pub fn identity(&self, inp: &Inputs, r: usize) -> Identity {
        identity_of(
            inp,
            r,
            self.hyp[r],
            self.best[r],
            self.ns[r],
            self.parent,
            self.neutral[r],
        )
    }
}

/// What orders the identities, kept apart from the identity itself.
#[derive(Clone, Copy)]
pub struct Key {
    total: f64,
    against: usize,
    area_max: f64,
}

impl Key {
    pub fn of(i: &Identity) -> Key {
        Key {
            total: i.breakdown.total,
            against: i.breakdown.against,
            area_max: i.area_max,
        }
    }
}

/// Order of the candidates: the explanation of the points, then the larger area (a stable sort keeps the order of the rows for the rest).
pub fn order(a: &Key, b: &Key) -> Ordering {
    b.total
        .total_cmp(&a.total)
        .then_with(|| a.against.cmp(&b.against))
        .then_with(|| b.area_max.total_cmp(&a.area_max))
}

