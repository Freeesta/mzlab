//! Choice among the charges of the isotope finder: every charge that scores is ranked, the best one is the pattern of the feature and the close ones
//! are kept as alternatives, each with a probability. After `IsotopeFinderEngine.detect` (mzmine 4.10.90); the automatic detection of the elements is not done.

use super::auto::{self, Composition};
use super::envelope::m1_bounds;
use super::{ChargeEval, ElementMode, Engine, Pt};

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
    /// Heavy elements found in the spectrum around the winning pattern (empty when the automatic detection is off or finds none).
    pub composition: Composition,
}

/// A charge that scored, with the candidates it was scored on.
struct Scored {
    eval: ChargeEval,
    cands: Vec<Pt>,
}

/// Charges that score, best first: by `raw`, ties by `quality` (stable, so equal charges keep the lower one first).
fn ranked(engine: &Engine, mz: &[f64], y: &[f64], feature_mz: f64, sign: i32) -> Vec<Scored> {
    let mut out: Vec<Scored> = (1..=engine.cfg.max_charge)
        .filter_map(|z| {
            let cands = engine.candidates(z, mz, y, feature_mz);
            let eval = engine.score_charge(z, feature_mz, &cands, sign)?;
            (eval.raw > 0.0).then_some(Scored { eval, cands })
        })
        .collect();
    out.sort_by(|a, b| {
        b.eval
            .raw
            .total_cmp(&a.eval.raw)
            .then(b.eval.quality.total_cmp(&a.eval.quality))
    });
    out
}

/// Second pass on the winner: the heavy elements seen in the raw spectrum around its pattern widen the upper bound of its envelope and that charge is
/// scored again. The winner stays first. Returns the elements found.
fn refine_winner(
    engine: &Engine,
    scored: &mut [Scored],
    mz: &[f64],
    y: &[f64],
    feature_mz: f64,
    sign: i32,
) -> Composition {
    let cfg = &engine.cfg;
    if cfg.mode == ElementMode::UserDefined {
        return Vec::new();
    }
    let w = &scored[0];
    let z = w.eval.charge;
    let lo = w.eval.kept.iter().fold(feature_mz, |m, p| m.min(p.0));
    let hi = w.eval.kept.iter().fold(feature_mz, |m, p| m.max(p.0));
    let window = auto::detection_window(mz, y, lo, hi, z);
    let candidates: Vec<&str> = cfg.auto_candidates.iter().map(String::as_str).collect();
    let found = auto::detect(&window, z, &cfg.tol, &candidates);
    if found.is_empty() {
        return found;
    }
    let counts: Vec<(&str, i64)> = found
        .iter()
        .map(|f| (f.symbol.as_str(), f.atoms.max(1)))
        .collect();
    let user = cfg.elements.iter().map(String::as_str).collect::<Vec<_>>();
    let env = cfg.model.build(
        feature_mz,
        z,
        sign,
        &user,
        Some(&counts),
        cfg.mode == ElementMode::UserPlusAuto,
    );
    let bounds = m1_bounds(&cfg.model, feature_mz, z, sign);
    if let Some(e) = engine
        .score_with(z, feature_mz, &w.cands, &env, bounds)
        .filter(|e| e.raw > 0.0)
    {
        scored[0].eval = e;
    }
    found
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
    let mut scored = ranked(engine, mz, y, feature_mz, sign);
    if scored.is_empty() {
        return None;
    }
    let composition = refine_winner(engine, &mut scored, mz, y, feature_mz, sign);
    let evals: Vec<&ChargeEval> = scored.iter().map(|s| &s.eval).collect();
    let best = evals.iter().fold(0.0f64, |m, e| m.max(e.quality));
    let sum: f64 = evals.iter().map(|e| e.quality).sum();
    let prob = |e: &ChargeEval| if sum > 0.0 { e.quality / sum } else { 0.0 };
    let probabilities = evals.iter().map(|e| (e.charge, prob(e))).collect();
    let patterns = evals
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
            eval: (*e).clone(),
        })
        .collect();
    Some(Detection {
        best_charge: evals[0].charge,
        patterns,
        probabilities,
        composition,
    })
}
