//! First or second generation: `AICc` of A -> B -> against A -> I -> B -> (the intermediate is unseen). A hint with an interval, never a
//! verdict: with a parent that cannot be followed and a dozen points the two are often not separable ("undecidable").

use super::bank::{aicc, grid, Bank, Row};
use super::expm::chain3;
use super::forms::FormBank;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gen {
    First,
    Second,
    Undecidable,
}

impl Gen {
    pub fn name(self) -> &'static str {
        match self {
            Gen::First => "first",
            Gen::Second => "second",
            Gen::Undecidable => "undecidable",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Generation {
    pub verdict: Gen,
    pub aicc_first: f64,
    pub aicc_second: f64,
    /// `aicc_second - aicc_first`: negative favours the second generation.
    pub delta: f64,
    pub rates_first: [f64; 2],
    pub rates_second: [f64; 3],
}

/// Curves of the chain with three rates (parent rate >= `k_parent_min`) on a fixed vector of times: about 3 700 matrix exponentials, built once.
pub struct ChainBank {
    bank: Bank,
}

impl ChainBank {
    pub fn new(times: &[f64], k_parent_min: f64) -> ChainBank {
        let g = grid();
        let mut rows = Vec::new();
        // the curve is symmetric in the three rates (up to a factor that the normalisation removes): ascending triples whose largest rate is
        // >= `k_parent_min` (the parent's decay)
        for (i, &a) in g.iter().enumerate() {
            for (j, &b) in g.iter().enumerate().skip(i) {
                rows.extend(g[j..].iter().filter(|&&c| c >= k_parent_min).map(|&c| Row {
                    group: 0,
                    p: [a, b, c],
                }));
            }
        }
        let t = times.to_vec();
        ChainBank {
            bank: Bank::new(rows, t.len(), |r, j| chain3(r.p, t[j])),
        }
    }
}

/// Default parent-rate floor of the second-generation model (1/min) and `AICc` margin of the decision.
pub const K_PARENT_MIN: f64 = 0.02;
pub const MARGIN: f64 = 2.0;

/// `forms` and `chain` must be built on the same times as `y`.
pub fn generation(
    forms: &FormBank,
    chain: &ChainBank,
    y: &[f64],
    margin: f64,
) -> Option<Generation> {
    let m = y.len();
    if m < 4 || m != forms.n_times() || m != chain.bank.n_times() {
        return None;
    }
    let first = forms.best_bateman(y)?;
    let cols: Vec<usize> = (0..m).collect();
    let (yt, _) = Bank::normalise(y, &cols);
    let h = chain.bank.fit_groups(&cols, &yt, 1)[0]?;
    let aicc_second = aicc(h.sse, m, 4);
    let delta = aicc_second - first.aicc;
    let verdict = if delta < -margin {
        Gen::Second
    } else if delta > margin {
        Gen::First
    } else {
        Gen::Undecidable
    };
    Some(Generation {
        verdict,
        aicc_first: first.aicc,
        aicc_second,
        delta,
        rates_first: first.rates,
        rates_second: chain.bank.rows[h.row].p,
    })
}
