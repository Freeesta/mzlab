//! Matches by chance (target–decoy): at unit resolution a hypothesis "matches" when any ion that passes the criteria lies within the tolerance of its
//! m/z, so the number of matches says little until it is compared with the matches of the same masses shifted by a few Da that no transformation
//! gives (decoys). `by_chance` = mean number of decoy masses that match; `p` = Poisson probability of at least `matched` matches with the mean
//! `(decoy matches + 1) / shifts` (the +1 keeps one coincidence from ever looking significant). A class with `p ≥ P_MAX` carries no evidence of
//! the formulas: its rows get no level 4 (`summary::level`). The criteria of a row do not change.

use mzlab_core::json::{round, Out};

use super::rows::Hyp;

/// Shifts of the decoys, Da: integers (the ions sit on the nominal channels) that no common transformation or loss gives.
pub const SHIFTS: [f64; 8] = [-19.0, -17.0, -13.0, -11.0, 11.0, 13.0, 17.0, 19.0];
/// A class of hypotheses is above chance when `p` is below this.
pub const P_MAX: f64 = 0.05;

/// One class of hypotheses (by number of steps).
#[derive(Debug, Clone, PartialEq)]
pub struct Chance {
    pub steps: u32,
    pub hypotheses: usize,
    pub matched: usize,
    pub by_chance: f64,
    pub p: f64,
}

impl Chance {
    pub fn above(&self) -> bool {
        self.p < P_MAX
    }
}

/// P(X ≥ k) for a Poisson variable of mean `lambda`.
pub fn poisson_sf(k: usize, lambda: f64) -> f64 {
    let (mut term, mut cdf) = ((-lambda).exp(), 0.0);
    for i in 0..k {
        cdf += term;
        term *= lambda / (i + 1) as f64;
    }
    (1.0 - cdf).max(0.0)
}

fn hits(masses: &[f64], ions: &[f64], tol: f64) -> usize {
    masses
        .iter()
        .filter(|m| ions.iter().any(|i| (i - *m).abs() <= tol))
        .count()
}

/// `ions` = m/z (as read, with the offset) of the rows that pass the criteria; `off` = the calibration offset applied to the hypotheses.
pub fn estimate(hyps: &[Hyp], ions: &[f64], off: f64, tol: f64, parent_mz: f64) -> Vec<Chance> {
    let mut steps: Vec<u32> = hyps.iter().map(|h| h.steps).collect();
    steps.sort_unstable();
    steps.dedup();
    steps
        .into_iter()
        .map(|n| {
            let mut m: Vec<f64> = hyps
                .iter()
                .filter(|h| h.steps == n && (h.mz - parent_mz).abs() > tol)
                .map(|h| h.mz + off)
                .collect();
            m.sort_by(f64::total_cmp);
            m.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
            let decoy: usize = SHIFTS
                .iter()
                .map(|d| hits(&m.iter().map(|x| x + d).collect::<Vec<_>>(), ions, tol))
                .sum();
            let matched = hits(&m, ions, tol);
            let n_s = SHIFTS.len() as f64;
            Chance {
                steps: n,
                hypotheses: m.len(),
                matched,
                by_chance: decoy as f64 / n_s,
                p: poisson_sf(matched, (decoy as f64 + 1.0) / n_s),
            }
        })
        .collect()
}

pub fn json(o: &mut Out, c: &[Chance]) {
    o.raw("[");
    for (i, x) in c.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"steps\":")
            .int(i64::from(x.steps))
            .raw(",\"hypotheses\":")
            .int(x.hypotheses as i64);
        o.raw(",\"matched\":")
            .int(x.matched as i64)
            .raw(",\"by_chance\":")
            .num(round(x.by_chance, 1))
            .raw(",\"p\":")
            .num(round(x.p, 4))
            .raw("}");
    }
    o.raw("]");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(mz: f64, steps: u32) -> Hyp {
        Hyp {
            label: String::new(),
            formula: String::new(),
            mz,
            neutral: mz - 1.007,
            steps,
            alternatives: Vec::new(),
        }
    }

    #[test]
    fn decoys_count_the_coverage_of_the_ions() {
        // ions on every nominal mass from 100 to 400: every hypothesis and every decoy matches
        let ions: Vec<f64> = (100..=400).map(|n| n as f64 + 0.1).collect();
        let hy: Vec<Hyp> = (0..10).map(|i| h(200.0 + 7.0 * i as f64, 2)).collect();
        let c = estimate(&hy, &ions, 0.0, 0.35, 364.0);
        assert_eq!(
            (c[0].hypotheses, c[0].matched, c[0].by_chance),
            (10, 10, 10.0)
        );
        assert!(!c[0].above());
        // one ion at a hypothesis and no decoy on it: one coincidence is not significant, three are
        let c = estimate(&hy, &[221.05], 0.0, 0.35, 364.0);
        assert_eq!((c[0].matched, c[0].by_chance), (1, 0.0));
        assert!(!c[0].above());
        let c = estimate(&hy, &[200.05, 221.05, 242.1], 0.0, 0.35, 364.0);
        assert!(c[0].above(), "{c:?}");
    }

    #[test]
    fn poisson_tail() {
        assert!((poisson_sf(0, 2.0) - 1.0).abs() < 1e-12);
        assert!((poisson_sf(1, 2.0) - (1.0 - (-2.0f64).exp())).abs() < 1e-12);
        assert!((poisson_sf(3, 0.8) - 0.0474).abs() < 1e-3);
    }
}
