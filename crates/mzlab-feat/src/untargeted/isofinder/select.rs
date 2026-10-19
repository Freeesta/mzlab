//! Choice among the charges of the isotope finder: every charge that scores is ranked, the best one is the pattern of the feature and the close ones
//! are kept as alternatives, each with a probability. After `IsotopeFinderEngine.detect` (mzmine 4.10.90); the automatic detection of the elements is not done.

use super::{ChargeEval, Engine, Pt};

/// An alternative is kept if its quality is within this of the best one...
pub const ALT_MARGIN: f64 = 0.15;
/// ...and at least this.
pub const MIN_ALT_QUALITY: f64 = 0.1;

/// The pattern of one charge: its peaks (the kept candidates), charge, score (quality x intensity agreement, for display) and probability.
#[derive(Clone, Debug, PartialEq)]
pub struct IsotopePattern {
    pub charge: i32,
    pub peaks: Vec<Pt>,
    pub score: f64,
    /// Quality of this charge over the sum of the qualities of ALL the charges that scored, also those not kept as alternatives.
    pub probability: f64,
    pub eval: ChargeEval,
}

/// Result for one spectrum: the patterns winner first, the winner's charge, and the probabilities of every charge that scored.
#[derive(Clone, Debug, PartialEq)]
pub struct Detection {
    pub best_charge: i32,
    pub patterns: Vec<IsotopePattern>,
    /// `(charge, probability)` of every charge that scored, best first; they sum to 1.
    pub probabilities: Vec<(i32, f64)>,
}

/// Charges that score, best first: by `raw`, ties by `quality` (stable, so equal charges keep the lower one first).
fn ranked(engine: &Engine, mz: &[f64], y: &[f64], feature_mz: f64, sign: i32) -> Vec<ChargeEval> {
    let mut out: Vec<ChargeEval> = (1..=engine.cfg.max_charge)
        .filter_map(|z| {
            let cands = engine.candidates(z, mz, y, feature_mz);
            engine.score_charge(z, feature_mz, &cands, sign)
        })
        .filter(|e| e.raw > 0.0)
        .collect();
    out.sort_by(|a, b| {
        b.raw
            .total_cmp(&a.raw)
            .then(b.quality.total_cmp(&a.quality))
    });
    out
}

/// Detects the isotope pattern and charge of the feature at `feature_mz` in the spectrum `(mz, y)`; `sign` is the polarity (+1, -1, 0 unknown).
/// `None` when no charge scores.
pub fn detect(
    engine: &Engine,
    mz: &[f64],
    y: &[f64],
    feature_mz: f64,
    sign: i32,
) -> Option<Detection> {
    let scored = ranked(engine, mz, y, feature_mz, sign);
    let best = scored.first()?.quality;
    let sum: f64 = scored.iter().map(|e| e.quality).sum();
    let prob = |e: &ChargeEval| if sum > 0.0 { e.quality / sum } else { 0.0 };
    let probabilities = scored.iter().map(|e| (e.charge, prob(e))).collect();
    let patterns = scored
        .iter()
        .enumerate()
        .filter(|(i, e)| {
            *i == 0 || (e.quality >= best - ALT_MARGIN && e.quality >= MIN_ALT_QUALITY)
        })
        .map(|(_, e)| IsotopePattern {
            charge: e.charge,
            peaks: e.kept.clone(),
            score: e.quality * e.intensity_agreement,
            probability: prob(e),
            eval: e.clone(),
        })
        .collect::<Vec<_>>();
    Some(Detection {
        best_charge: scored[0].charge,
        patterns,
        probabilities,
    })
}
